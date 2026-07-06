use serde::Serialize;
use tokio::sync::broadcast;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Serialize)]
pub struct MetricPoint {
    pub epoch: usize,
    pub step: usize,
    pub loss: f32,
}

static METRICS_SENDER: Lazy<broadcast::Sender<MetricPoint>> = Lazy::new(|| {
    let (tx, _) = broadcast::channel(200);
    tx
});

pub fn publish_metric(point: MetricPoint) {
    let _ = METRICS_SENDER.send(point);
}

pub fn subscribe_metrics() -> broadcast::Receiver<MetricPoint> {
    METRICS_SENDER.subscribe()
}
