"""The speculative orchestrator: marries hardware state (SPS) with drafting,
confidence-gated early termination, and rejection-sampling verification into a
single generation loop.

Design notes:
  * Backbone/verifier models are injected (dependency injection), so existing
    model code is never modified — the engine only composes.
  * All state is local to `generate`; inputs are never mutated.
  * A backbone is expected to return an object with `.hidden_states`
    (batch, seq, hidden); a verifier returns an object with `.logits`
    (batch, seq, vocab). Tiny stand-ins in the tests satisfy this contract.
"""
import torch
import torch.nn as nn

from .config import DSparkConfig
from .dynamic_hardware import SPSManager
from .neural_components import ParallelIntern, LowRankMarkovHead, ConfidenceHead


class SpeculativeEngine:
    def __init__(self, intern_backbone: nn.Module, boss_model: nn.Module, config: DSparkConfig):
        self.config = config
        self.intern_backbone = intern_backbone
        self.boss_model = boss_model

        self.drafter = ParallelIntern(config.hidden_dim, config.vocab_size, config.max_draft_len)
        self.markov = LowRankMarkovHead(config.hidden_dim, config.vocab_size, config.markov_rank)
        self.confidence = ConfidenceHead(config.hidden_dim)
        self.sps = SPSManager(
            config.min_draft_len, config.max_draft_len,
            config.gpu_critical_load, config.gpu_idle_load,
        )

        # Instrumentation only — not part of the generation contract. Counts
        # every real boss-model forward pass (`_boss_step` fallback tokens and
        # `_verify` rejection-sampling checks), so callers (e.g. a regression
        # benchmark) can derive tokens-produced-per-boss-call as a cheap,
        # seed-stable proxy for "is speculation still doing something useful."
        self.boss_calls = 0

    @torch.no_grad()
    def generate(self, input_ids: torch.Tensor, max_new_tokens: int) -> torch.Tensor:
        current = input_ids
        produced = 0

        while produced < max_new_tokens:
            batch = current.shape[0]

            # 1. Hardware dictates how far the intern may draft.
            draft_len = self.sps.draft_length(active_requests=batch)

            # 2. Backbone features for the last position.
            hidden = self.intern_backbone(current).hidden_states[:, -1, :]  # (batch, hidden)

            # 3. Parallel draft + Markov suffix-decay correction.
            draft_logits = self.drafter(hidden, draft_len)                  # (batch, d, vocab)
            markov_bias = self.markov(hidden).unsqueeze(1)                  # (batch, 1, vocab)
            draft_ids = torch.argmax(draft_logits + markov_bias, dim=-1)    # (batch, d)

            # 4. Confidence-gated early termination.
            draft_hidden = hidden.unsqueeze(1).expand(-1, draft_ids.shape[1], -1)
            confidences = self.confidence(draft_hidden)                     # (batch, d)
            valid = self._confidence_cutoff(confidences)

            if valid == 0:
                current = torch.cat([current, self._boss_step(current)], dim=1)
                produced += 1
                continue

            # 5. Verify the surviving draft in one boss pass (rejection sampling).
            accepted = self._verify(current, draft_ids[:, :valid])
            if accepted.shape[1] == 0:
                current = torch.cat([current, self._boss_step(current)], dim=1)
                produced += 1
            else:
                current = torch.cat([current, accepted], dim=1)
                produced += accepted.shape[1]

        return current

    def _confidence_cutoff(self, confidences: torch.Tensor) -> int:
        """First position whose (batch-min) confidence drops below threshold.
        Returns a length in [0, draft_len]."""
        per_pos = torch.min(confidences, dim=0).values  # protect the whole batch
        for i, score in enumerate(per_pos):
            if score < self.config.confidence_threshold:
                return i
        return confidences.shape[1]

    def _verify(self, context: torch.Tensor, draft_ids: torch.Tensor) -> torch.Tensor:
        """The boss checks the whole draft in parallel; accept the longest
        contiguous prefix it agrees with (a mismatch at t+i rejects t+i..t+k)."""
        self.boss_calls += 1
        full = torch.cat([context, draft_ids], dim=1)
        logits = self.boss_model(full).logits[:, context.shape[1] - 1:-1, :]
        boss_ids = torch.argmax(logits, dim=-1)
        match = (boss_ids == draft_ids)
        contiguous = torch.cumprod(match.int(), dim=1).sum(dim=1)  # per-batch prefix length
        keep = int(contiguous.min().item())                        # safe across the batch
        return draft_ids[:, :keep]

    def _boss_step(self, current: torch.Tensor) -> torch.Tensor:
        """One ordinary autoregressive token, when speculation yields nothing."""
        self.boss_calls += 1
        logits = self.boss_model(current).logits[:, -1:, :]
        return torch.argmax(logits, dim=-1)
