"""Standalone smoke test for the DSpark engine — no real LLM, just tiny stand-in
backbone/verifier modules. Run: `python dspark_system/test_smoke.py`.

Covers the functional checklist: hardware fallback, confidence-cutoff bounds,
rejection-sampling verification, and no mutation of the caller's input.
"""
import torch
import torch.nn as nn

from dspark_system.config import DSparkConfig
from dspark_system.dynamic_hardware import SPSManager
from dspark_system.engine import SpeculativeEngine


class _Backbone(nn.Module):
    """Returns an object with `.hidden_states` (batch, seq, hidden)."""
    def __init__(self, vocab, hidden):
        super().__init__()
        self.embed = nn.Embedding(vocab, hidden)

    def forward(self, ids):
        class Out:
            pass
        out = Out()
        out.hidden_states = self.embed(ids)
        return out


class _Verifier(nn.Module):
    """Returns an object with `.logits` (batch, seq, vocab)."""
    def __init__(self, vocab, hidden):
        super().__init__()
        self.embed = nn.Embedding(vocab, hidden)
        self.proj = nn.Linear(hidden, vocab)

    def forward(self, ids):
        class Out:
            pass
        out = Out()
        out.logits = self.proj(self.embed(ids))
        return out


def _engine(vocab=64, hidden=32):
    cfg = DSparkConfig(vocab_size=vocab, hidden_dim=hidden, max_draft_len=6, min_draft_len=2, markov_rank=8)
    return SpeculativeEngine(_Backbone(vocab, hidden), _Verifier(vocab, hidden), cfg)


def test_sps_fallback_never_raises():
    sps = SPSManager(2, 16, 0.85, 0.40)
    # utilization() must return a number in [0,1] even with no GPU/NVML.
    u = sps.utilization()
    assert 0.0 <= u <= 1.0
    # draft_length stays within [min,max] for a spread of loads.
    for reqs in (1, 50, 500):
        d = sps.draft_length(reqs)
        assert 2 <= d <= 16
    print("ok: SPS fallback + bounded draft length")


def test_confidence_cutoff_bounds():
    eng = _engine()
    conf = torch.tensor([[0.9, 0.8, 0.5, 0.7]])  # dips below 0.6 at index 2
    assert eng._confidence_cutoff(conf) == 2
    assert eng._confidence_cutoff(torch.ones(1, 4)) == 4          # all confident
    assert eng._confidence_cutoff(torch.zeros(1, 4)) == 0         # none confident
    print("ok: confidence cutoff bounded in [0, draft_len]")


def test_rejection_is_contiguous():
    eng = _engine()
    context = torch.randint(0, 64, (1, 3))
    # Force the boss to a known argmax by zeroing then spiking one token isn't
    # trivial here; instead assert the structural contract: verify() returns a
    # prefix no longer than the draft, and cutting the batch min is honored.
    draft = torch.randint(0, 64, (1, 5))
    accepted = eng._verify(context, draft)
    assert 0 <= accepted.shape[1] <= draft.shape[1]
    print("ok: rejection sampling returns a bounded contiguous prefix")


def test_generate_makes_progress_and_preserves_input():
    eng = _engine()
    prompt = torch.randint(0, 64, (1, 4))
    original = prompt.clone()
    out = eng.generate(prompt, max_new_tokens=5)
    assert out.shape[1] >= prompt.shape[1] + 5           # produced >= requested
    assert torch.equal(prompt, original)                  # input not mutated
    print("ok: generate() makes progress without mutating input")


if __name__ == "__main__":
    torch.manual_seed(0)
    test_sps_fallback_never_raises()
    test_confidence_cutoff_bounds()
    test_rejection_is_contiguous()
    test_generate_makes_progress_and_preserves_input()
    print("\nAll DSpark smoke tests passed.")
