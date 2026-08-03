#![allow(dead_code)]
//! Online Learning Loop — learns from web/online sources.
//!
//! This is intentionally separate from `active_learning_loop.rs`.
//! Active Learning = internal optimization without external connectivity.
//! Online Learning = ingestion + fact-checking + distillation from
//! scraped/compiled web data into trainable artifacts.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use serde_json::json;

use crate::fact_checking_model::{EvidenceChunk, FactCheckingModel, Verdict};
use crate::scraping_model::ScrapingModel;
use crate::data_assistant_model::{DataAssistantModel, DataSample};
use crate::meta_model_builder::MetaModelBuilder;


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FactCheckResult {
    pub claim: String,
    pub supported: bool,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub contradictions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataPackage {
    pub id: String,
    pub source_urls: Vec<String>,
    pub scraped_path: PathBuf,
    pub fact_check_path: PathBuf,
    pub distilled_path: PathBuf,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeneratedModelSpec {
    pub id: String,
    pub task: String,
    pub architecture: String,
    pub training_data_path: PathBuf,
    pub output_path: PathBuf,
    pub metrics: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScrapeJob {
    pub id: String,
    pub url: String,
    pub status: String,
    pub raw_path: PathBuf,
    pub created_at: String,
}

#[derive(Clone)]
pub struct OnlineLearningLoop {
    jobs: Arc<Mutex<Vec<ScrapeJob>>>,
    data_packages: Arc<Mutex<Vec<DataPackage>>>,
    generated_models: Arc<Mutex<Vec<GeneratedModelSpec>>>,
    base_dir: PathBuf,
    fact_checker: Arc<FactCheckingModel>,
    scraper: Arc<ScrapingModel>,
    data_assistant: Arc<DataAssistantModel>,
    model_builder: Arc<MetaModelBuilder>,
}

impl OnlineLearningLoop {
    pub fn new(base_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&base_dir).ok();
        std::fs::create_dir_all(base_dir.join("raw")).ok();
        std::fs::create_dir_all(base_dir.join("fact_checks")).ok();
        std::fs::create_dir_all(base_dir.join("distilled")).ok();
        std::fs::create_dir_all(base_dir.join("models")).ok();

        let fact_checker = Arc::new(FactCheckingModel::new());
        let scraper = Arc::new(ScrapingModel::new());
        let data_assistant = Arc::new(DataAssistantModel::new());
        let model_builder = Arc::new(MetaModelBuilder::new());

        Self {
            jobs: Arc::new(Mutex::new(Vec::new())),
            data_packages: Arc::new(Mutex::new(Vec::new())),
            generated_models: Arc::new(Mutex::new(Vec::new())),
            base_dir,
            fact_checker,
            scraper,
            data_assistant,
            model_builder,
        }
    }

    pub async fn enqueue_scrape(&self, url: impl Into<String>) -> Result<ScrapeJob, String> {
        let url = url.into();
        let id = uuid::Uuid::new_v4().to_string();
        let raw_path = self.base_dir.join("raw").join(format!("{}.json", id));
        std::fs::create_dir_all(raw_path.parent().unwrap()).map_err(|e| e.to_string())?;

        let job = ScrapeJob {
            id: id.clone(),
            url: url.clone(),
            status: "queued".into(),
            raw_path,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.jobs.lock().await.push(job.clone());
        info!(job_id=%id, url=%url, "online scrape job queued");
        Ok(job)
    }

    pub async fn list_scrape_jobs(&self) -> Vec<ScrapeJob> {
        self.jobs.lock().await.clone()
    }

    pub async fn run_fact_check(
        &self,
        claim: impl Into<String>,
        evidence: Vec<String>,
    ) -> Result<FactCheckResult, String> {
        let claim = claim.into();

        // Ingest evidence text into the fact-checking model's document store.
        self.fact_checker.ingest_documents(evidence.clone()).await;

        // Build EvidenceChunk list from raw evidence strings.
        let chunks: Vec<EvidenceChunk> = evidence
            .iter()
            .map(|text| EvidenceChunk {
                text: text.clone(),
                source_domain: "web".into(),
                stance: crate::fact_checking_model::Stance::Neutral,
                confidence: 0.5,
                tfidf_score: 0.0,
            })
            .collect();

        // Run the fact-checking model's verify method.
        let verification = self.fact_checker.verify(claim.clone(), chunks).await;

        // Translate model output into online-learning fact-check format.
        let contradictions: Vec<String> = verification
            .contradicting
            .iter()
            .map(|c| c.text.clone())
            .collect();

        let supported = matches!(
            verification.verdict,
            Verdict::Verified | Verdict::LikelyTrue
        );

        info!(claim=%claim, supported=%supported, confidence=%verification.confidence, contradictions=%contradictions.len(), "fact check completed");

        Ok(FactCheckResult {
            claim,
            supported,
            confidence: verification.confidence,
            evidence,
            contradictions,
        })
    }

    pub async fn build_data_package(
        &self,
        source_urls: Vec<String>,
        scraped_items: Vec<(String, String, PathBuf)>,
    ) -> Result<DataPackage, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let scraped_path = self.base_dir.join("raw").join(format!("{}.json", id));
        let fact_check_path = self.base_dir.join("fact_checks").join(format!("{}.json", id));
        let distilled_path = self.base_dir.join("distilled").join(format!("{}.json", id));

        std::fs::create_dir_all(scraped_path.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(fact_check_path.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(distilled_path.parent().unwrap()).map_err(|e| e.to_string())?;

        // Use the scraping model to extract and deduplicate the scraped items.
        let mut scrape_results = Vec::new();
        for (html, url, path) in scraped_items {
            let result = self.scraper.extract(html, url.clone(), path).await;
            scrape_results.push(result);
        }

        let unique_results = ScrapingModel::deduplicate(scrape_results, 0.85);

        // Persist scraped results.
        let serialized = serde_json::to_string_pretty(&unique_results).map_err(|e| e.to_string())?;
        std::fs::write(&scraped_path, serialized).map_err(|e| e.to_string())?;

        // Profile the scraped data with the data assistant.
        let dataset_id = format!("scrape-{}", id);
        let mut samples = Vec::new();
        for (i, r) in unique_results.iter().enumerate() {
            let mut features = std::collections::HashMap::new();
            features.insert("url".into(), serde_json::Value::String(r.url.clone()));
            features.insert(
                "quality".into(),
                serde_json::Value::Number(serde_json::Number::from_f64(r.quality).unwrap_or_else(|| serde_json::Number::from(0))),
            );
            features.insert(
                "relevance".into(),
                serde_json::Value::Number(serde_json::Number::from_f64(r.relevance).unwrap_or_else(|| serde_json::Number::from(0))),
            );
            features.insert("text_len".into(), serde_json::Value::Number(serde_json::Number::from(r.text.len())));
            samples.push(DataSample {
                id: format!("row-{}", i),
                features,
                label: None,
            });
        }
        self.data_assistant.ingest(dataset_id.clone(), samples).await;

        let profile = self.data_assistant.profile(&dataset_id).await;
        let profile_json = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
        std::fs::write(&fact_check_path, profile_json).map_err(|e| e.to_string())?;

        let pkg = DataPackage {
            id,
            source_urls,
            scraped_path,
            fact_check_path,
            distilled_path,
            status: "built".into(),
        };

        self.data_packages.lock().await.push(pkg.clone());
        info!(pkg_id=%pkg.id, rows=unique_results.len(), "data package built");
        Ok(pkg)
    }

    pub async fn generate_model_spec(
        &self,
        task: impl Into<String>,
        training_data_path: PathBuf,
        output_path: PathBuf,
    ) -> Result<GeneratedModelSpec, String> {
        let task = task.into();
        let blueprint = self.model_builder.recommend_blueprint(&task).await;
        let blueprint_id = blueprint.as_ref().map(|b| b.id.clone()).unwrap_or_else(|| "default".into());

        let spec = self
            .model_builder
            .build_model_spec(&blueprint_id, &task, training_data_path.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
            .await?;

        let id = uuid::Uuid::new_v4().to_string();
        let generated = GeneratedModelSpec {
            id,
            task,
            architecture: spec.blueprint_id.clone(),
            training_data_path,
            output_path,
            metrics: json!({
                "blueprint_id": spec.blueprint_id,
                "parameters": spec.parameters,
                "metrics": spec.metrics,
            }),
        };

        self.generated_models.lock().await.push(generated.clone());
        info!(model_id=%generated.id, architecture=%generated.architecture, "model spec generated");
        Ok(generated)
    }

    pub async fn list_data_packages(&self) -> Vec<DataPackage> {
        self.data_packages.lock().await.clone()
    }

    pub async fn list_generated_models(&self) -> Vec<GeneratedModelSpec> {
        self.generated_models.lock().await.clone()
    }
}
