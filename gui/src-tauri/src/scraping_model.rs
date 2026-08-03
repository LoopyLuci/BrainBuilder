#![allow(dead_code)]
//! Custom web-scraping / content-extraction model.
//!
//! Includes relevance scoring, deduplication, structured extraction,
//! persistence, and batch processing.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Section {
    pub heading: Option<String>,
    pub level: u8,
    pub text: String,
    pub char_start: usize,
    pub char_end: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Link {
    pub href: String,
    pub text: Option<String>,
    pub rel: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScrapeResult {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub text: String,
    pub sections: Vec<Section>,
    pub links: Vec<Link>,
    pub metadata: HashMap<String, String>,
    pub quality: f64,
    pub relevance: f64,
    pub raw_path: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractionFeatures {
    pub text_density: f64,
    pub link_density: f64,
    pub punctuation_ratio: f64,
    pub stopword_ratio: f64,
    pub heading_presence: bool,
    pub relevance: f64,
}

#[derive(Clone, Default)]
pub struct ScrapingModel {
    stop_words: Arc<Mutex<Vec<String>>>,
}

impl ScrapingModel {
    pub fn new() -> Self {
        let stop_words = vec![
            "the","and","for","with","from","this","that","have","been","into",
            "would","could","should","about","which","their","there","your","more","other",
            "some","than","also","just","over","into","been","have","that","this","from","with"
        ].into_iter().map(|s| s.to_string()).collect();
        Self { stop_words: Arc::new(Mutex::new(stop_words)) }
    }

    pub async fn extract(&self, raw_html: String, url: String, raw_path: PathBuf) -> ScrapeResult {
        let title = extract_tag(&raw_html, "title");
        let description = extract_meta(&raw_html, "description");
        let links = extract_links(&raw_html);
        let sections = extract_sections(&raw_html);
        let mut text = sections.iter().map(|s| s.text.clone()).collect::<Vec<_>>().join("\n\n");
        let metadata = extract_metadata(&raw_html);

        text = Self::normalize(&text);
        let quality = Self::score_quality(&text, &links, &sections);
        let relevance = self.score_relevance(&text, &title, &description);

        ScrapeResult { url, title, description, text, sections, links, metadata, quality, relevance, raw_path }
    }

    pub async fn batch_extract(&self, items: Vec<(String, String, PathBuf)>) -> Vec<ScrapeResult> {
        let mut out = Vec::new();
        let mut seen_hashes: HashSet<u64> = HashSet::new();
        for (html, url, path) in items {
            let mut result = self.extract(html, url.clone(), path).await;
            let hash = Self::simhash(&result.text);
            if seen_hashes.contains(&hash) {
                result.quality *= 0.1;
            } else {
                seen_hashes.insert(hash);
            }
            out.push(result);
        }
        out
    }

    pub fn normalize(text: &str) -> String {
        let mut out = text.to_string();
        out = out.replace('\r', "");
        while out.contains("\n\n\n") {
            out = out.replace("\n\n\n", "\n\n");
        }
        out = out.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n");
        out
    }

    pub fn score_quality(text: &str, links: &[Link], sections: &[Section]) -> f64 {
        let chars = text.chars().count() as f64;
        let link_count = links.len() as f64;
        let section_count = sections.len() as f64;
        let text_density = if chars > 0.0 { 1.0 } else { 0.0 };
        let link_density = if chars > 0.0 { (link_count * 80.0 / chars).min(1.0) } else { 1.0 };
        let section_score = section_count.min(20.0) / 20.0;
        let mut score = (text_density * 0.6 + (1.0 - link_density) * 0.25 + section_score * 0.15).clamp(0.0, 1.0);
        if chars < 200.0 { score *= 0.5; }
        score
    }

    pub fn score_relevance(&self, text: &str, title: &Option<String>, description: &Option<String>) -> f64 {
        let hay = format!("{} {} {}", title.clone().unwrap_or_default(), description.clone().unwrap_or_default(), text);
        let words: Vec<&str> = hay.split_whitespace().collect();
        let sw = self.stop_words.blocking_lock();
        let unique_content = words.iter().filter(|w| !sw.contains(&w.to_lowercase())).count();
        (unique_content as f64 / words.len().max(1) as f64).clamp(0.0, 1.0)
    }

    pub fn simhash(text: &str) -> u64 {
        let mut hash: [u64; 64] = [0; 64];
        for token in Self::tokenize(text) {
            let h = token.bytes().fold(0u64, |a, b| a.wrapping_add(b as u64));
            for i in 0..64 {
                if (h >> i) & 1 == 1 { hash[i] += 1; } else { hash[i] -= 1; }
            }
        }
        let mut fingerprint = 0u64;
        for i in 0..64 { if hash[i] > 0 { fingerprint |= 1 << i; } }
        fingerprint
    }

    pub fn deduplicate(results: Vec<ScrapeResult>, threshold: f64) -> Vec<ScrapeResult> {
        let mut unique = Vec::new();
        let mut seen: Vec<u64> = Vec::new();
        for r in results {
            let hash = Self::simhash(&r.text);
            let max_hamming = (threshold * 64.0) as u32;
            let is_dup = seen.iter().any(|&h| (hash ^ h).count_ones() as u32 <= max_hamming);
            if !is_dup { seen.push(hash); unique.push(r); }
        }
        unique
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|s| !s.is_empty() && s.len() > 2).map(|s| s.to_string()).collect()
    }
}

fn extract_tag(html: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = html.find(&open) {
        let rest = &html[start + open.len()..];
        if let Some(end) = rest.find(&close) { return Some(rest[..end].trim().to_string()); }
    }
    None
}

fn extract_meta(html: &str, name: &str) -> Option<String> {
    for pat in [format!("name=\"{}\"", name), format!("name='{}'", name)] {
        if let Some(pos) = html.find(&pat) {
            let after = &html[pos + pat.len()..];
            if let Some(content_pos) = after.find("content=\"") {
                let value = &after[content_pos + 8..];
                if let Some(end) = value.find("\"") { return Some(value[..end].to_string()); }
            }
        }
    }
    None
}

fn extract_links(html: &str) -> Vec<Link> {
    let mut out = Vec::new();
    let mut search = html;
    while let Some(start) = search.find("<a ") {
        search = &search[start + 3..];
        let end = search.find(">").unwrap_or(search.len());
        let attrs = &search[..end];
        let mut href = None;
        let mut rel = Vec::new();
        for part in attrs.split_whitespace() {
            if let Some(v) = part.strip_prefix("href=\"") { href = Some(v.trim_end_matches('"').to_string()); }
            else if let Some(v) = part.strip_prefix("rel=\"") { rel = v.trim_end_matches('"').split(',').map(|s| s.trim().to_string()).collect(); }
        }
        let text = extract_link_text(search, end);
        out.push(Link { href: href.unwrap_or_default(), text, rel });
    }
    out
}

fn extract_link_text(html: &str, attrs_end: usize) -> Option<String> {
    let rest = &html[attrs_end..];
    if let Some(end) = rest.find("</a>") { return Some(rest[..end].trim().to_string()); }
    None
}

fn extract_sections(html: &str) -> Vec<Section> {
    let mut out = Vec::new();
    let mut search = html;
    while let Some(start) = search.find('<') {
        search = &search[start..];
        let close = search.find('>').unwrap_or(search.len());
        let tag = &search[1..close];
        search = &search[close + 1..];
        if let Some(name) = tag.strip_prefix("h") {
            let level = name.chars().next().and_then(|c| c.to_digit(10)).unwrap_or(1) as u8;
            if let Some(end) = search.find(&format!("</h{}", level)) {
                let heading_text = search[..end].trim().to_string();
                let content = if let Some(next) = search.find(&format!("<h",)) { &search[..next.min(end)] } else { &search[..end] };
                out.push(Section { heading: Some(heading_text.clone()), level, text: ScrapingModel::normalize(content), char_start: 0, char_end: content.chars().count() });
            }
        }
    }
    out
}

fn extract_metadata(html: &str) -> HashMap<String, String> {
    let mut meta = HashMap::new();
    let mut search = html;
    while let Some(start) = search.find("<meta") {
        search = &search[start + 5..];
        let end = search.find(">").unwrap_or(search.len());
        let attrs = search[..end].to_string();
        let mut name = None;
        let mut content = None;
        for part in attrs.split_whitespace() {
            if let Some(v) = part.strip_prefix("name=\"") { name = Some(v.trim_end_matches('"').to_string()); }
            else if let Some(v) = part.strip_prefix("content=\"") { content = Some(v.trim_end_matches('"').to_string()); }
        }
        if let (Some(n), Some(c)) = (name, content) { meta.insert(n, c); }
    }
    meta
}
