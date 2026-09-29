// Proves the `text_column` data source wires end to end through the real
// DataFusion CSV reader and Arrow stringification into the bag-of-words
// featurizer — i.e. that pointing at an actual spreadsheet with a text column
// and a label column yields trainable `(batch, vocab), target` batches. No
// `torch` needed: this exercises the ingestion half (the part unique to this
// feature), so it runs in the normal (non-ignored) suite.
use brainbuilder_core::bbir::{DataSourceConfig, TrainingConfig};
use brainbuilder_core::data::source::load_dataset;

fn training_config(csv_path: &str) -> TrainingConfig {
    TrainingConfig {
        loss: "cross_entropy".to_string(),
        optimizer: "adam".to_string(),
        trainer_type: "standard".to_string(),
        hyperparams: serde_json::json!({"lr": 0.01, "epochs": 5}),
        data_source: DataSourceConfig {
            source_type: "text_column".to_string(),
            path_or_uri: csv_path.to_string(),
            batch_size: 4,
            preprocessing: Vec::new(),
            sequence_length: None,
            vocab_size: Some(50),
            image_size: None,
            grayscale: None,
            text_column: Some("review".to_string()),
            label_column: Some("sentiment".to_string()),
        },
        reproducibility: None,
    }
}

#[test]
fn text_column_source_produces_bag_of_words_batches_from_a_real_csv() {
    let dir = std::env::temp_dir().join(format!("bb_text_column_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let csv_path = dir.join("reviews.csv");

    // A real CSV with a free-text column (`review`) and a label column
    // (`sentiment`), including commas-in-quotes to prove the real CSV parser
    // (not a naive split) is doing the work.
    let csv = "review,sentiment\n\
        \"loved it, wonderful and great\",positive\n\
        \"awful, terrible and bad\",negative\n\
        \"great joy, so wonderful\",positive\n\
        \"bad and terrible experience\",negative\n";
    std::fs::write(&csv_path, csv).unwrap();

    let config = training_config(&csv_path.to_string_lossy());
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut iter = rt.block_on(load_dataset(&config)).expect("text_column ingestion should succeed");

    // Collect every batch's rows.
    let mut total_rows = 0usize;
    let mut feature_width = None;
    let mut targets: Vec<f32> = Vec::new();
    while let Some(batch) = iter.next().unwrap() {
        total_rows += batch.num_rows();
        let width = batch.num_columns() - 1; // minus the target column
        assert!(width >= 2, "bag-of-words width should be the vocabulary size, got {width}");
        feature_width.get_or_insert(width);
        assert_eq!(feature_width, Some(width), "every batch must have the same feature width");

        let target = batch
            .column(batch.num_columns() - 1)
            .as_any()
            .downcast_ref::<arrow::array::Float32Array>()
            .expect("target column is Float32");
        for i in 0..target.len() {
            targets.push(target.value(i));
        }
    }

    assert_eq!(total_rows, 4, "all four rows should be ingested");
    // Two classes, sorted: "negative" -> 0, "positive" -> 1. Row order is
    // preserved: pos, neg, pos, neg.
    assert_eq!(targets, vec![1.0, 0.0, 1.0, 0.0]);

    std::fs::remove_dir_all(&dir).ok();
}
