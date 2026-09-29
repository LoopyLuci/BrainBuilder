// Free-text-column ingestion: the second most common shape of real personal
// data after image folders — a spreadsheet/CSV with a column of free text
// (a review, an email body, a support ticket) and a column of labels
// (positive/negative, spam/ham, a category). This turns each row into a
// fixed-width bag-of-words feature vector plus a class-index target — exactly
// the `(batch, features)` + last-column-target shape the existing MLP training
// path consumes, so a real text classifier trains with no new tensor plumbing.
//
// Bag-of-words (per-row token counts over a frequency-capped vocabulary) is a
// deliberately simple, real, order-independent representation — not embeddings
// or attention (those need the sequence path in `data::text`). It's the honest
// floor for "point at a spreadsheet of text and get a working classifier."
use crate::data::text::Vocab;
use crate::interop::protocol::BrainBuilderError;
use crate::Result;

/// A row's raw text and its raw label string, as pulled from the tabular file
/// (both stringified — see `source::load_text_column_dataset`, which uses the
/// same Arrow formatters as the preview so any column type works uniformly).
pub struct TextRow {
    pub text: String,
    pub label: String,
}

/// The fully-featurized dataset: the class-name→index mapping (sorted distinct
/// labels, so it's deterministic and inference can map a predicted index back
/// to a real label), the per-row feature count (= vocabulary size), and the
/// dense rows (`features` length == `feature_count`, `label` an index into
/// `class_names`).
#[derive(Debug)]
pub struct TextColumnDataset {
    pub class_names: Vec<String>,
    pub feature_count: usize,
    pub rows: Vec<(Vec<f32>, f32)>,
}

/// Build a bag-of-words dataset from raw `(text, label)` rows.
///
/// `vocab_size` caps the vocabulary to its most frequent words (see
/// `Vocab::build`); the feature vector length equals the realized vocabulary
/// size (`Vocab::len`, which includes the reserved `<unk>` slot). Counts are
/// L1-normalized per row (divided by that row's token count) so a long
/// document and a short one live on the same scale — the standard, assumption-
/// light normalization for bag-of-words, and it keeps inputs small for stable
/// training.
pub fn build_bag_of_words(rows: &[TextRow], vocab_size: usize) -> Result<TextColumnDataset> {
    if rows.len() < 2 {
        return Err(BrainBuilderError::ConfigError(format!(
            "text-column dataset has {} row(s); need at least 2 to train",
            rows.len()
        )));
    }

    // Deterministic class map: distinct labels, sorted.
    let mut class_names: Vec<String> = rows.iter().map(|r| r.label.clone()).collect();
    class_names.sort();
    class_names.dedup();
    if class_names.len() < 2 {
        return Err(BrainBuilderError::ConfigError(format!(
            "text-column dataset has only one distinct label (`{}`); need at least 2 classes to train a classifier",
            class_names.first().map(|s| s.as_str()).unwrap_or("")
        )));
    }
    let label_index = |label: &str| -> usize {
        class_names.iter().position(|c| c == label).expect("label was collected into class_names")
    };

    // One vocabulary over the whole corpus, so every row's feature vector
    // indexes the same word→column mapping.
    let corpus: String = rows.iter().map(|r| r.text.as_str()).collect::<Vec<_>>().join("\n");
    let vocab = Vocab::build(&corpus, vocab_size.max(2));
    let feature_count = vocab.len();

    let mut out_rows = Vec::with_capacity(rows.len());
    for row in rows {
        let ids = vocab.encode(&row.text);
        let mut features = vec![0f32; feature_count];
        for id in &ids {
            features[*id as usize] += 1.0;
        }
        // L1 normalize by token count (guard the empty-text case).
        if !ids.is_empty() {
            let inv = 1.0 / ids.len() as f32;
            for f in &mut features {
                *f *= inv;
            }
        }
        out_rows.push((features, label_index(&row.label) as f32));
    }

    Ok(TextColumnDataset { class_names, feature_count, rows: out_rows })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(text: &str, label: &str) -> TextRow {
        TextRow { text: text.to_string(), label: label.to_string() }
    }

    #[test]
    fn builds_deterministic_sorted_class_indices() {
        let rows = vec![
            row("great and wonderful", "pos"),
            row("awful and terrible", "neg"),
            row("wonderful joy", "pos"),
        ];
        let ds = build_bag_of_words(&rows, 100).unwrap();
        // Sorted distinct labels: "neg" -> 0, "pos" -> 1.
        assert_eq!(ds.class_names, vec!["neg".to_string(), "pos".to_string()]);
        assert_eq!(ds.rows[0].1, 1.0); // pos
        assert_eq!(ds.rows[1].1, 0.0); // neg
        assert_eq!(ds.rows[2].1, 1.0); // pos
    }

    #[test]
    fn feature_vectors_are_l1_normalized_bag_of_words() {
        let rows = vec![row("cat cat dog", "a"), row("dog", "b")];
        let ds = build_bag_of_words(&rows, 100).unwrap();
        // Every row's features sum to 1.0 (L1 normalized), and width == vocab.
        for (features, _) in &ds.rows {
            assert_eq!(features.len(), ds.feature_count);
            let sum: f32 = features.iter().sum();
            assert!((sum - 1.0).abs() < 1e-5, "row features should sum to ~1.0, got {sum}");
        }
        // "cat cat dog" has cat weighted 2/3, dog 1/3 — cat's column is larger.
        let max = ds.rows[0].0.iter().cloned().fold(0.0f32, f32::max);
        assert!((max - 2.0 / 3.0).abs() < 1e-5, "most frequent word should dominate: {max}");
    }

    #[test]
    fn rejects_single_class_data() {
        let rows = vec![row("a", "same"), row("b", "same")];
        let err = build_bag_of_words(&rows, 100).unwrap_err();
        assert!(err.to_string().contains("one distinct label"), "got: {err}");
    }
}
