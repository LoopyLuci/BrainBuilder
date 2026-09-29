"""The DSpark drafter heads, as torch modules. These mirror the trainable
BrainBuilder components of the same names; kept here too so the serving engine
is self-contained.
"""
import torch
import torch.nn as nn


class ParallelIntern(nn.Module):
    """Multi-head parallel drafter: predicts `k` future tokens in one pass, one
    projection head per future position."""

    def __init__(self, hidden_dim, vocab_size, max_draft_len):
        super().__init__()
        self.heads = nn.ModuleList([nn.Linear(hidden_dim, vocab_size) for _ in range(max_draft_len)])

    def forward(self, hidden_state, draft_len):
        active = min(draft_len, len(self.heads))
        per_pos = [self.heads[i](hidden_state).unsqueeze(1) for i in range(active)]
        return torch.cat(per_pos, dim=1)  # (batch, draft_len, vocab)


class LowRankMarkovHead(nn.Module):
    """Suffix-decay corrector: a low-rank (compress -> expand) bias on the next
    token, conditioned only on the immediately preceding hidden state."""

    def __init__(self, hidden_dim, vocab_size, rank):
        super().__init__()
        self.compress = nn.Linear(hidden_dim, rank, bias=False)
        self.expand = nn.Linear(rank, vocab_size, bias=False)

    def forward(self, prev_hidden_state):
        return self.expand(self.compress(prev_hidden_state))  # (batch, vocab)


class ConfidenceHead(nn.Module):
    """Scores each drafted position's probability of acceptance, in [0, 1]."""

    def __init__(self, hidden_dim):
        super().__init__()
        self.scorer = nn.Sequential(
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.SiLU(),
            nn.Linear(hidden_dim // 2, 1),
            nn.Sigmoid(),
        )

    def forward(self, hidden_states):
        return self.scorer(hidden_states).squeeze(-1)  # (batch, seq_len)
