//! Plain-English diagnostics — the trust loop for someone who can't read a
//! loss curve or spot a leaky feature on their own.
//!
//! A zero-knowledge user can be handed a perfectly valid graph and still end
//! up with a useless model: their data is 95% one class, their learning rate
//! is so high the loss diverges, or their "features" secretly contain the
//! answer. Shape validation (`component::validation`) catches *structural*
//! problems before training; this module catches *statistical* ones — before
//! training (by inspecting the data) and during/after it (by inspecting the
//! loss curve) — and explains each in words a beginner can act on, with a
//! concrete suggested fix.
//!
//! Everything here is deliberately pure and deterministic (no I/O, no model
//! calls) so each rule is unit-testable against hand-built inputs; the callers
//! that actually read the folder / stream the losses live in the GUI command
//! layer and reuse the existing ingestion code.
use serde::Serialize;

/// How much a finding should alarm the user. `Error` means the run is almost
/// certainly wasted as-is; `Warning` means it'll probably train but the result
/// may be misleading; `Info` is a neutral heads-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// One finding: what's wrong, why it matters, and what to do about it — all in
/// plain language, no jargon assumed.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    /// A short headline (e.g. "Your data is very imbalanced").
    pub title: String,
    /// A one- or two-sentence explanation of the problem in everyday terms.
    pub explanation: String,
    /// A concrete, actionable next step.
    pub suggestion: String,
}

impl Diagnostic {
    fn new(severity: Severity, title: &str, explanation: String, suggestion: &str) -> Self {
        Self {
            severity,
            title: title.to_string(),
            explanation,
            suggestion: suggestion.to_string(),
        }
    }
}

// --------------------------------------------------------------------------
// Data-time diagnostics (before training)
// --------------------------------------------------------------------------

/// Analyze class balance and dataset size from per-class example counts
/// (`(class_name, count)`). Real problems this catches:
/// - **Severe imbalance**: if the biggest class is ≥ 80% of the data, a model
///   can score high just by always guessing it, learning nothing useful.
/// - **Tiny classes**: a class with only a handful of examples can't be learned.
/// - **Too little data overall**.
pub fn analyze_class_balance(counts: &[(String, usize)]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let total: usize = counts.iter().map(|(_, c)| *c).sum();
    if total == 0 || counts.is_empty() {
        return out;
    }

    // Severe imbalance.
    if let Some((biggest_name, biggest)) = counts.iter().max_by_key(|(_, c)| *c) {
        let fraction = *biggest as f64 / total as f64;
        if counts.len() >= 2 && fraction >= 0.8 {
            out.push(Diagnostic::new(
                Severity::Warning,
                "Your data is very imbalanced",
                format!(
                    "{:.0}% of your {} examples are the class \"{}\". A model can look accurate just by always guessing that class, without actually learning anything.",
                    fraction * 100.0,
                    total,
                    biggest_name
                ),
                "Add more examples of the smaller classes, or remove some of the largest class, so the classes are closer to even.",
            ));
        }
    }

    // Tiny classes.
    let tiny: Vec<&String> = counts.iter().filter(|(_, c)| *c > 0 && *c < 5).map(|(n, _)| n).collect();
    if !tiny.is_empty() {
        let names = tiny.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", ");
        out.push(Diagnostic::new(
            Severity::Warning,
            "Some classes have very few examples",
            format!("These classes have fewer than 5 examples each: {names}. That's usually too few for the model to learn them."),
            "Aim for at least a few dozen examples per class — the more the better.",
        ));
    }

    // Too little data overall.
    if total < 2 * counts.len().max(1) {
        out.push(Diagnostic::new(
            Severity::Warning,
            "Very little training data",
            format!("You have only {total} example(s) across {} class(es). Models need repetition to learn.", counts.len()),
            "Collect more examples — even a few dozen per class makes a big difference.",
        ));
    }

    out
}

/// Detect a leaky feature in numeric tabular data: a feature column whose
/// values are (nearly) a perfect linear stand-in for the target. This is the
/// classic silent failure — the model "learns" by copying a column that
/// wouldn't be available at prediction time, gets ~perfect training loss, and
/// is useless in reality. `feature` and `target` are aligned columns (same
/// length, one value per row).
pub fn analyze_feature_target_leakage(
    feature_name: &str,
    feature: &[f64],
    target: &[f64],
) -> Option<Diagnostic> {
    if feature.len() != target.len() || feature.len() < 3 {
        return None;
    }
    let r = pearson_correlation(feature, target)?;
    if r.abs() >= 0.999 {
        return Some(Diagnostic::new(
            Severity::Error,
            "A feature may be leaking the answer",
            format!(
                "The column \"{feature_name}\" is almost perfectly correlated with what you're trying to predict (correlation {:.3}). The model will likely just copy it and score near-perfectly while learning nothing generalizable.",
                r
            ),
            "Check whether this column is actually available before you'd make a prediction. If it's derived from the answer, remove it from the inputs.",
        ));
    }
    None
}

/// Pearson correlation coefficient; `None` if either column has no variance
/// (a constant column can't be correlated with anything).
fn pearson_correlation(a: &[f64], b: &[f64]) -> Option<f64> {
    let n = a.len() as f64;
    let mean_a = a.iter().sum::<f64>() / n;
    let mean_b = b.iter().sum::<f64>() / n;
    let mut cov = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        let da = x - mean_a;
        let db = y - mean_b;
        cov += da * db;
        var_a += da * da;
        var_b += db * db;
    }
    if var_a <= f64::EPSILON || var_b <= f64::EPSILON {
        return None;
    }
    Some(cov / (var_a.sqrt() * var_b.sqrt()))
}

// --------------------------------------------------------------------------
// Training-time diagnostics (during / after training)
// --------------------------------------------------------------------------

/// Analyze a sequence of per-step loss values and explain, in plain English,
/// what the curve is saying. Catches the failure modes a beginner can't read:
/// - **Diverging / NaN**: loss blew up — the classic "learning rate too high".
/// - **Flat**: loss barely moved — either the LR is too small, or the model
///   can't fit the data.
/// - **Healthy**: loss dropped meaningfully — an affirming, confidence-building
///   signal (people need to know when things went *right*, too).
pub fn analyze_loss_curve(losses: &[f32]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    if losses.len() < 3 {
        return out; // not enough signal to say anything honest yet
    }

    // NaN / infinite loss — unambiguous divergence.
    if losses.iter().any(|l| !l.is_finite()) {
        out.push(Diagnostic::new(
            Severity::Error,
            "Training diverged (loss became invalid)",
            "The loss became NaN or infinite, which means the numbers blew up during training. This almost always means the learning rate is too high.".to_string(),
            "Lower the learning rate (try dividing it by 10) and train again.",
        ));
        return out;
    }

    let first = losses[0];
    let last = *losses.last().unwrap();
    let max = losses.iter().cloned().fold(f32::MIN, f32::max);

    // Loss trending upward / exploding without going NaN.
    if last > first * 2.0 || max > first * 5.0 {
        out.push(Diagnostic::new(
            Severity::Error,
            "The loss is going up, not down",
            format!("Training started at a loss of {first:.4} but it climbed to {last:.4}. That means the model is getting worse, usually because the learning rate is too high."),
            "Lower the learning rate (try dividing it by 10) and train again.",
        ));
        return out;
    }

    // Essentially flat.
    let relative_drop = if first.abs() > f32::EPSILON { (first - last) / first.abs() } else { 0.0 };
    if relative_drop < 0.02 {
        out.push(Diagnostic::new(
            Severity::Warning,
            "The loss barely changed",
            format!("The loss went from {first:.4} to {last:.4} — almost no improvement. Either the learning rate is too small to make progress, or the model can't capture the pattern in your data."),
            "Try raising the learning rate (e.g. multiply it by 10), training for more epochs, or using a larger model.",
        ));
        return out;
    }

    // Healthy drop — tell the user it worked.
    if relative_drop >= 0.2 {
        out.push(Diagnostic::new(
            Severity::Info,
            "Training is going well",
            format!("The loss dropped from {first:.4} to {last:.4} ({:.0}% lower) — the model is learning.", relative_drop * 100.0),
            "If accuracy is still short of what you want, train for more epochs or add more data.",
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_severe_class_imbalance() {
        let counts = vec![("cat".to_string(), 90), ("dog".to_string(), 10)];
        let diags = analyze_class_balance(&counts);
        assert!(diags.iter().any(|d| d.title.contains("imbalanced") && d.severity == Severity::Warning));
    }

    #[test]
    fn balanced_data_produces_no_imbalance_warning() {
        let counts = vec![("cat".to_string(), 50), ("dog".to_string(), 50)];
        let diags = analyze_class_balance(&counts);
        assert!(!diags.iter().any(|d| d.title.contains("imbalanced")));
    }

    #[test]
    fn flags_tiny_classes() {
        let counts = vec![("common".to_string(), 100), ("rare".to_string(), 2)];
        let diags = analyze_class_balance(&counts);
        assert!(diags.iter().any(|d| d.title.contains("few examples")));
    }

    #[test]
    fn detects_a_perfectly_leaky_feature() {
        // `answer_copy` == target * 1.0 => correlation 1.0.
        let target = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let leaky = target.clone();
        let d = analyze_feature_target_leakage("answer_copy", &leaky, &target).expect("should flag leakage");
        assert_eq!(d.severity, Severity::Error);
        assert!(d.title.contains("leaking"));
    }

    #[test]
    fn a_weakly_correlated_feature_is_not_flagged() {
        let target = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let noisy = vec![5.0, 1.0, 4.0, 2.0, 3.0];
        assert!(analyze_feature_target_leakage("noisy", &noisy, &target).is_none());
    }

    #[test]
    fn nan_loss_is_reported_as_divergence() {
        let losses = vec![1.0f32, 0.8, f32::NAN, 0.5];
        let diags = analyze_loss_curve(&losses);
        assert!(diags.iter().any(|d| d.severity == Severity::Error && d.title.contains("diverged")));
    }

    #[test]
    fn rising_loss_is_reported() {
        let losses = vec![1.0f32, 2.0, 4.0, 9.0];
        let diags = analyze_loss_curve(&losses);
        assert!(diags.iter().any(|d| d.title.contains("going up")));
    }

    #[test]
    fn flat_loss_is_reported() {
        let losses = vec![1.0f32, 0.999, 0.999, 0.998];
        let diags = analyze_loss_curve(&losses);
        assert!(diags.iter().any(|d| d.title.contains("barely changed")));
    }

    #[test]
    fn healthy_drop_is_affirmed() {
        let losses = vec![1.0f32, 0.6, 0.4, 0.2];
        let diags = analyze_loss_curve(&losses);
        assert!(diags.iter().any(|d| d.severity == Severity::Info && d.title.contains("going well")));
    }
}
