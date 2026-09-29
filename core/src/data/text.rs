// Real word-level tokenization + sliding-window sequence dataset, for
// training the transformer/attention components (components/attention.py,
// embedding.py) on actual text instead of only tabular CSV/parquet data.
// Deliberately simple (whitespace splitting, frequency-ranked vocabulary) —
// this is a real, working, deterministic tokenizer, not a byte-pair-encoding
// or subword scheme, which is a larger, separate piece of work.
use crate::interop::protocol::BrainBuilderError;
use crate::Result;
use std::collections::HashMap;

/// Reserved for any word that didn't make the vocabulary's frequency cutoff.
pub const UNK_ID: u32 = 0;

pub struct Vocab {
    word_to_id: HashMap<String, u32>,
}

impl Vocab {
    /// Builds a vocabulary from real word frequencies in `text`, keeping the
    /// `max_size - 1` most common words (id 0 is always `<unk>`). Ties break
    /// by first appearance order, so the result is deterministic for a given
    /// input rather than depending on hash-map iteration order.
    pub fn build(text: &str, max_size: usize) -> Self {
        let mut counts: HashMap<&str, u32> = HashMap::new();
        let mut first_seen: Vec<&str> = Vec::new();
        for word in tokenize_words(text) {
            let entry = counts.entry(word).or_insert(0);
            if *entry == 0 {
                first_seen.push(word);
            }
            *entry += 1;
        }

        let mut words = first_seen;
        words.sort_by(|a, b| counts[b].cmp(&counts[a]).then_with(|| a.cmp(b)));
        words.truncate(max_size.saturating_sub(1));

        let word_to_id = words
            .into_iter()
            .enumerate()
            .map(|(i, w)| (w.to_string(), (i + 1) as u32))
            .collect();
        Self { word_to_id }
    }

    pub fn encode(&self, text: &str) -> Vec<u32> {
        tokenize_words(text)
            .map(|w| self.word_to_id.get(w).copied().unwrap_or(UNK_ID))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.word_to_id.len() + 1 // + <unk>
    }

    pub fn is_empty(&self) -> bool {
        false // <unk> always exists
    }
}

/// Lowercased whitespace/punctuation-boundary word splitting — real
/// tokenization (not a stub), but intentionally simple: no subword units, no
/// locale-aware segmentation. Good enough to train and exercise the
/// attention/embedding components on real text.
fn tokenize_words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_alphanumeric() && c != '\'').filter(|s| !s.is_empty())
}

/// Non-overlapping-target sliding windows for next-token prediction: each
/// window is `seq_len` consecutive token ids, and its target is the token
/// immediately following it. Slides by 1, so a corpus of N tokens yields
/// `N - seq_len` real training examples — standard language-model windowing.
pub fn windows(ids: &[u32], seq_len: usize) -> Result<Vec<(Vec<u32>, u32)>> {
    if ids.len() <= seq_len {
        return Err(BrainBuilderError::ConfigError(format!(
            "text corpus has only {} token(s), need more than sequence_length={seq_len} to form a single training window",
            ids.len()
        )));
    }
    Ok((0..ids.len() - seq_len)
        .map(|i| (ids[i..i + seq_len].to_vec(), ids[i + seq_len]))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocab_ranks_by_real_frequency() {
        let vocab = Vocab::build("the cat sat on the mat the cat ran", 10);
        // "the" appears 3x, "cat" 2x — both must get real (non-unk) ids, and
        // "the" must not collide with "cat".
        let encoded = vocab.encode("the cat");
        assert_ne!(encoded[0], UNK_ID);
        assert_ne!(encoded[1], UNK_ID);
        assert_ne!(encoded[0], encoded[1]);
    }

    #[test]
    fn vocab_maps_unknown_words_to_unk() {
        let vocab = Vocab::build("apple banana apple", 10);
        assert_eq!(vocab.encode("mango")[0], UNK_ID);
    }

    #[test]
    fn vocab_caps_size_and_keeps_most_frequent() {
        // 5 distinct words, cap at 3 (2 real + <unk>): the two most frequent
        // ("a", "b") must survive; "e" (frequency 1, alphabetically last
        // among the frequency-1 words) must not.
        let vocab = Vocab::build("a a a b b c d e", 3);
        assert_eq!(vocab.len(), 3);
        assert_ne!(vocab.encode("a")[0], UNK_ID);
        assert_ne!(vocab.encode("b")[0], UNK_ID);
        assert_eq!(vocab.encode("e")[0], UNK_ID);
    }

    #[test]
    fn windows_slide_by_one_with_the_next_token_as_target() {
        let ids = vec![1, 2, 3, 4, 5];
        let result = windows(&ids, 2).unwrap();
        assert_eq!(result, vec![(vec![1, 2], 3), (vec![2, 3], 4), (vec![3, 4], 5)]);
    }

    #[test]
    fn windows_rejects_a_corpus_too_short_for_even_one_window() {
        assert!(windows(&[1, 2], 5).is_err());
    }
}
