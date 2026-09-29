use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvalCase {
    pub id: String,
    pub prompt: String,
    pub expected_contains: Option<String>,
    pub forbidden_contains: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvalResult {
    pub case_id: String,
    pub passed: bool,
    pub detail: String,
    pub latency_ms: u64,
}

#[derive(Clone)]
pub struct EvalHarness {
    cases: Arc<Mutex<Vec<EvalCase>>>,
    history: Arc<Mutex<Vec<EvalResult>>>,
    _base_dir: PathBuf,
}

impl EvalHarness {
    pub fn new(base_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&base_dir).ok();
        Self {
            cases: Arc::new(Mutex::new(Vec::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            _base_dir: base_dir,
        }
    }

    pub async fn add_case(&self, case: EvalCase) {
        self.cases.lock().await.push(case);
    }

    pub async fn run<F, Fut>(&self, mut run_fn: F) -> Vec<EvalResult>
    where
        F: FnMut(String) -> Fut,
        Fut: Future<Output = String> + Send + 'static,
    {
        let mut results = Vec::new();
        let cases = self.cases.lock().await.clone();

        for case in cases {
            let start = std::time::Instant::now();
            let response = run_fn(case.prompt.clone()).await;
            let latency = start.elapsed().as_millis() as u64;

            let passed = case.expected_contains.as_ref().map(|s| response.contains(s)).unwrap_or(true)
                && case.forbidden_contains.as_ref().map(|s| !response.contains(s)).unwrap_or(true);

            let detail = if passed { "ok".into() } else { format!("response: {}", response) };
            results.push(EvalResult { case_id: case.id.clone(), passed, detail, latency_ms: latency });
        }

        self.history.lock().await.extend(results.clone());
        info!(eval_count=results.len(), passed=results.iter().filter(|r| r.passed).count(), "eval run complete");
        results
    }

    pub async fn history(&self) -> Vec<EvalResult> {
        self.history.lock().await.clone()
    }
}
