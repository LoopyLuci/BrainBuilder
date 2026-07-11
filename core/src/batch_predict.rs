// Real bulk inference: the Predict panel's existing preview only ever runs
// the model on a handful of rows kept in memory. This runs the trained
// checkpoint over every row of a dataset file — as many chunks as
// `data::source::load_all` streams the file in, one at a time, so the whole
// file is never held as a single giant batch — and writes a CSV of
// `<input columns...>,prediction` to disk, one line per row. Column handling
// matches the existing single-row `predict` command exactly (no target
// column is assumed or dropped): the dataset is expected to hold exactly the
// feature columns the trained graph was built on, the same as any real
// deployment input would.
use crate::bbir::BBIRGraph;
use crate::data::source::load_all;
use crate::interop::protocol::BrainBuilderError;
use crate::interpret::predict_flat;
use crate::orchestrator::Orchestrator;
use crate::Result;
use arrow::util::display::{ArrayFormatter, FormatOptions};
use std::io::Write;
use std::path::Path;

/// Runs `graph`'s trained checkpoint over every row of `dataset_path` and
/// writes the result to `output_path` as CSV. Returns the number of rows
/// written. Errors (without creating a partial file being mistaken for a
/// complete one — an already-created empty file is left in place, matching
/// `std::fs::File::create`'s own truncate-on-open semantics) if the dataset
/// has no rows, or if any chunk's forward pass fails (e.g. no checkpoint
/// trained yet).
pub async fn run_batch_predict(
    orchestrator: &Orchestrator,
    graph: &BBIRGraph,
    dataset_path: &str,
    output_path: &Path,
) -> Result<usize> {
    let mut data = load_all(dataset_path).await?;
    let mut file = std::fs::File::create(output_path).map_err(|e| {
        BrainBuilderError::ConfigError(format!("couldn't create `{}`: {e}", output_path.display()))
    })?;

    let mut total_rows = 0usize;
    let mut header_written = false;
    while let Some(batch) = data.next()? {
        if batch.num_rows() == 0 {
            continue;
        }
        if !header_written {
            let mut header: Vec<String> = batch.schema().fields().iter().map(|f| f.name().clone()).collect();
            header.push("prediction".to_string());
            writeln!(file, "{}", header.join(","))
                .map_err(|e| BrainBuilderError::ConfigError(format!("failed writing CSV header: {e}")))?;
            header_written = true;
        }

        let predictions = predict_flat(orchestrator, graph, batch.clone())?;
        let num_rows = batch.num_rows();
        // `predict_flat` returns one *tensor's worth* of values, not
        // necessarily one scalar per row — a classifier's real output is
        // `[batch, num_classes]`. Indexing it as if it were `num_rows`
        // scalars (`predictions.get(row)`) used to silently misread
        // class-interleaved values as row predictions for any such model.
        // `per_row` values divide evenly for a real model output (checked
        // below); when there's more than one, write the predicted class
        // index (argmax) — the natural "prediction" for a classification
        // row — instead of a meaningless raw logit.
        let per_row = if num_rows == 0 { 0 } else { predictions.len() / num_rows.max(1) };
        if per_row == 0 || predictions.len() != num_rows * per_row {
            return Err(BrainBuilderError::ConfigError(format!(
                "predicted output has {} value(s) for {num_rows} row(s) — not an even per-row count",
                predictions.len()
            )));
        }
        let opts = FormatOptions::default();
        let formatters: Vec<ArrayFormatter> = batch
            .columns()
            .iter()
            .map(|c| ArrayFormatter::try_new(c.as_ref(), &opts))
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| BrainBuilderError::ConfigError(e.to_string()))?;

        for row in 0..num_rows {
            let mut fields: Vec<String> = formatters.iter().map(|f| csv_escape(&f.value(row).to_string())).collect();
            let row_values = &predictions[row * per_row..(row + 1) * per_row];
            let pred = if per_row == 1 {
                row_values[0]
            } else {
                row_values
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, _)| i as f32)
                    .unwrap_or(f32::NAN)
            };
            fields.push(pred.to_string());
            writeln!(file, "{}", fields.join(","))
                .map_err(|e| BrainBuilderError::ConfigError(format!("failed writing CSV row: {e}")))?;
        }
        total_rows += batch.num_rows();
    }

    if !header_written {
        return Err(BrainBuilderError::ConfigError(format!(
            "`{dataset_path}` contains no rows to predict on"
        )));
    }
    Ok(total_rows)
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_escape_leaves_plain_values_untouched() {
        assert_eq!(csv_escape("3.14"), "3.14");
        assert_eq!(csv_escape("hello"), "hello");
    }

    #[test]
    fn csv_escape_quotes_and_doubles_embedded_quotes() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_escape("line\nbreak"), "\"line\nbreak\"");
    }
}
