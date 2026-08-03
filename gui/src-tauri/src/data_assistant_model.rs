#![allow(dead_code)]
//! Custom data-analysis / data-assistant model.
//!
//! Provides schema inference, outlier detection, profiling, correlation,
//! distribution stats, recommendations, natural-language queries, and
//! persistence over in-memory datasets. Self-contained and offline-capable.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataProfile {
    pub dataset_id: String,
    pub row_count: usize,
    pub column_count: usize,
    pub columns: HashMap<String, ColumnStats>,
    pub missing: HashMap<String, usize>,
    pub correlations: HashMap<String, f64>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ColumnStats {
    pub dtype: ColumnDtype,
    pub non_null: usize,
    pub unique: usize,
    pub mean: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub std: Option<f64>,
    pub median: Option<f64>,
    pub mode: Option<String>,
    pub percentiles: BTreeMap<usize, f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ColumnDtype {
    Numeric,
    Text,
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OutlierDetection {
    pub dataset_id: String,
    pub column: String,
    pub row_ids: Vec<String>,
    pub z_scores: Vec<(String, f64)>,
    pub threshold: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataQueryResult {
    pub dataset_id: String,
    pub question: String,
    pub answer: String,
    pub supporting_rows: Vec<usize>,


    pub confidence: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataSample {
    pub id: String,
    pub features: HashMap<String, serde_json::Value>,
    pub label: Option<serde_json::Value>,
}

#[derive(Clone, Default)]
pub struct DataAssistantModel {
    datasets: Arc<Mutex<HashMap<String, Vec<DataSample>>>>,
}

impl DataAssistantModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn ingest(&self, dataset_id: impl Into<String>, samples: Vec<DataSample>) {
        self.datasets.lock().await.insert(dataset_id.into(), samples);
    }

    pub async fn profile(&self, dataset_id: &str) -> Result<DataProfile, String> {
        let ds = self.datasets.lock().await.get(dataset_id).cloned().unwrap_or_default();
        if ds.is_empty() {
            return Ok(DataProfile {
                dataset_id: dataset_id.into(),
                row_count: 0,
                column_count: 0,
                columns: HashMap::new(),
                missing: HashMap::new(),
                correlations: HashMap::new(),
                recommendations: vec!["No data available".into()],
            });
        }

        let row_count = ds.len();
        let mut columns: BTreeMap<String, ColumnStats> = BTreeMap::new();
        let mut missing: HashMap<String, usize> = HashMap::new();

        for sample in &ds {
            for (k, v) in &sample.features {
                if v.is_null() {
                    *missing.entry(k.clone()).or_default() += 1;
                }
            }
        }

        let mut numeric_values: HashMap<String, Vec<(String, f64)>> = HashMap::new();
        for sample in &ds {
            for (k, v) in &sample.features {
                let entry = columns.entry(k.clone()).or_insert(ColumnStats {
                    dtype: ColumnDtype::Unknown,
                    non_null: 0,
                    unique: 0,
                    mean: None,
                    min: None,
                    max: None,
                    std: None,
                    median: None,
                    mode: None,
                    percentiles: BTreeMap::new(),
                });

                if !v.is_null() {
                    entry.non_null += 1;
                }

                if let Some(num) = v.as_f64() {
                    if entry.dtype != ColumnDtype::Numeric {
                        entry.dtype = ColumnDtype::Numeric;
                        entry.mean = Some(0.0);
                        entry.min = Some(num);
                        entry.max = Some(num);
                    } else {
                        let mean = entry.mean.unwrap_or(0.0);
                        entry.mean = Some((mean * (entry.non_null - 1) as f64 + num) / entry.non_null as f64);
                        entry.min = Some(entry.min.unwrap_or(num).min(num));
                        entry.max = Some(entry.max.unwrap_or(num).max(num));
                    }
                    numeric_values.entry(k.clone()).or_default().push((sample.id.clone(), num));
                } else if v.is_string() && entry.dtype == ColumnDtype::Unknown {
                    entry.dtype = ColumnDtype::Text;
                }
            }
        }

        let mut unique_counts: HashMap<String, HashSet<String>> = HashMap::new();
        for sample in &ds {
            for (k, v) in &sample.features {
                if !v.is_null() {
                    unique_counts.entry(k.clone()).or_default().insert(v.to_string());
                }
            }
        }

        for (k, entry) in columns.iter_mut() {
            entry.unique = unique_counts.get(k).map(|s| s.len()).unwrap_or(0);
        }

        // Compute std, median, percentiles for numeric columns
        for (col, values) in &numeric_values {
            if let Some(entry) = columns.get_mut(col) {
                let mut nums: Vec<f64> = values.iter().map(|(_, v)| *v).collect();
                nums.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
                let n = nums.len();
                if n > 0 {
                    entry.median = Some(if n % 2 == 0 { (nums[n/2 - 1] + nums[n/2]) / 2.0 } else { nums[n/2] });
                    let mean = entry.mean.unwrap_or(0.0);
                    let variance = nums.iter().map(|v| (*v - mean).powi(2)).sum::<f64>() / n as f64;
                    entry.std = Some(variance.sqrt());
                    entry.percentiles = BTreeMap::from([
                        (25, percentile(&nums, 0.25)),
                        (50, percentile(&nums, 0.50)),
                        (75, percentile(&nums, 0.75)),
                        (95, percentile(&nums, 0.95)),
                    ]);
                }
            }
        }

        // Simple correlation between first two numeric columns
        let mut correlations = HashMap::new();
        let num_cols: Vec<_> = columns.iter().filter(|(_, s)| s.dtype == ColumnDtype::Numeric).map(|(k, _)| k.clone()).collect();
        if num_cols.len() >= 2 {
            let a_vals: Vec<f64> = numeric_values.get(&num_cols[0]).map(|v| v.iter().map(|(_, x)| *x).collect()).unwrap_or_default();
            let b_vals: Vec<f64> = numeric_values.get(&num_cols[1]).map(|v| v.iter().map(|(_, x)| *x).collect()).unwrap_or_default();
            if a_vals.len() == b_vals.len() && !a_vals.is_empty() {
                let corr = pearson(&a_vals, &b_vals);
                correlations.insert(format!("{} <-> {}", num_cols[0], num_cols[1]), corr);
            }
        }

        // Recommendations
        let mut recommendations = Vec::new();
        if !missing.is_empty() {
            let worst = missing.iter().max_by_key(|(_, v)| *v).unwrap();
            recommendations.push(format!("Column '{}' has {} missing values - consider imputation", worst.0, worst.1));
        }
        for (col, stats) in &columns {
            if let Some(std) = stats.std {
                if std > stats.mean.unwrap_or(0.0).abs() * 2.0 {
                    recommendations.push(format!("Column '{}' has high variance - consider normalization", col));
                }
            }
        }

        info!(dataset=%dataset_id, rows=row_count, cols=columns.len(), "data profiling completed");

        Ok(DataProfile {
            dataset_id: dataset_id.into(),
            row_count,
            column_count: columns.len(),
            columns: columns.into_iter().collect(),
            missing,
            correlations,
            recommendations,
        })
    }

    pub async fn detect_outliers(
        &self,
        dataset_id: &str,
        column: &str,
        threshold: f64,
    ) -> Result<OutlierDetection, String> {
        let ds = self.datasets.lock().await.get(dataset_id).cloned().unwrap_or_default();
        let mut values: Vec<(String, f64)> = Vec::new();
        for sample in &ds {
            if let Some(v) = sample.features.get(column).and_then(|v| v.as_f64()) {
                values.push((sample.id.clone(), v));
            }
        }

        if values.len() < 2 {
            return Ok(OutlierDetection {
                dataset_id: dataset_id.into(),
                column: column.into(),
                row_ids: Vec::new(),
                z_scores: Vec::new(),
                threshold,
            });
        }

        let mean = values.iter().map(|(_, v)| *v).sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|(_, v)| (*v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std = variance.sqrt();

        let mut z_scores = Vec::new();
        let mut row_ids = Vec::new();
        for (id, v) in values {
            let z = if std > 0.0 { (v - mean) / std } else { 0.0 };
            z_scores.push((id.clone(), z));
            if z.abs() > threshold {
                row_ids.push(id);
            }
        }

        Ok(OutlierDetection { dataset_id: dataset_id.into(), column: column.into(), row_ids, z_scores, threshold })
    }

    pub async fn query(
        &self,
        dataset_id: &str,
        question: String,
    ) -> DataQueryResult {
        let _ds: Vec<DataSample> = self.datasets.lock().await.get(dataset_id).cloned().unwrap_or_default();
        let lower = question.to_lowercase();
        let ds = self.datasets.lock().await.get(dataset_id).cloned().unwrap_or_default();
        let mut supporting_vec = Vec::new();

        if lower.contains("how many") || lower.contains("count") {
            let answer = format!("{} rows found in dataset '{}'", ds.len(), dataset_id);
            return DataQueryResult {
                dataset_id: dataset_id.into(),
                question,
                answer,
                supporting_rows: supporting_vec,
                confidence: if ds.is_empty() { 0.0 } else { 0.9 },
            };
        }

        for token in lower.split_whitespace() {
            if token.len() > 3 {
                for (i, sample) in ds.iter().enumerate() {
                    let hay = sample.features.values().map(|v| v.to_string()).collect::<Vec<_>>().join(" ").to_lowercase();
                    if hay.contains(token) {
                        supporting_vec.push(i);
                    }
                }
            }
        }

        let answer = if supporting_vec.is_empty() {
            format!("No matching records found for '{}'", question)
        } else {
            format!("{} records matched for '{}' in dataset '{}'", supporting_vec.len(), question, dataset_id)
        };

        let confidence = if supporting_vec.is_empty() { 0.3 } else { 0.75 };
        DataQueryResult {
            dataset_id: dataset_id.into(),
            question,
            answer,
            supporting_rows: supporting_vec,
            confidence,
        }
    }

    pub async fn recommend(&self, dataset_id: &str) -> Vec<String> {
        let profile = self.profile(dataset_id).await.ok();
        profile.map(|p| p.recommendations).unwrap_or_default()
    }
}

fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() { return 0.0; }
    let pos = q * (sorted.len() - 1) as f64;
    let lower = sorted[pos.floor() as usize];
    let upper = sorted[pos.ceil() as usize];
    let frac = pos - pos.floor();
    lower * (1.0 - frac) + upper * frac
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let sum_x = x.iter().sum::<f64>();
    let sum_y = y.iter().sum::<f64>();
    let sum_xy = x.iter().zip(y).map(|(a, b)| a * b).sum::<f64>();
    let sum_x2 = x.iter().map(|v| v.powi(2)).sum::<f64>();
    let sum_y2 = y.iter().map(|v| v.powi(2)).sum::<f64>();
    let num = n * sum_xy - sum_x * sum_y;
    let den = ((n * sum_x2 - sum_x.powi(2)) * (n * sum_y2 - sum_y.powi(2))).sqrt();
    if den == 0.0 { 0.0 } else { num / den }
}
