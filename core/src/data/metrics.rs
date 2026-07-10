use serde::Serialize;
use tokio::sync::broadcast;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Serialize)]
pub struct MetricPoint {
    pub epoch: usize,
    pub step: usize,
    pub loss: f32,
    /// True only on the final point of a run that stopped before its
    /// configured `epochs` because loss hadn't improved for `patience`
    /// epochs (see `runtime::standard_trainer`) — false on every other
    /// point, including every point of a run that ran to completion.
    pub stopped_early: bool,
    /// The actual learning rate used for this point's step. Equal to the
    /// configured `lr` on every point unless `lr_decay_epochs` is set (see
    /// `runtime::standard_trainer`), in which case it periodically halves —
    /// exposed so the dashboard can show the real, current value instead of
    /// only ever the starting one.
    pub current_lr: f32,
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
