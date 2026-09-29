import torch


def forward(input, compress_weight, expand_weight):
    """DSpark Low-Rank Markov Head. Cures "suffix decay" in a parallel drafter
    by biasing the next-token logits from the immediately preceding hidden
    state through a tight low-rank bottleneck: compress (features -> rank),
    then expand (rank -> vocab). Matches low_rank_markov_head.edn's port shapes.

    input:           (batch, features)
    compress_weight: (rank, features)
    expand_weight:   (vocab, rank)
    -> output:       (batch, vocab)  logit bias

    The low-rank factorization is what keeps this a negligible add to latency:
    it never materializes a full (features x vocab) matrix.
    """
    if input.shape[-1] != compress_weight.shape[-1]:
        raise ValueError(
            f"low_rank_markov_head: input features={input.shape[-1]} does not match "
            f"compress_weight's in_features={compress_weight.shape[-1]}"
        )
    if compress_weight.shape[0] != expand_weight.shape[-1]:
        raise ValueError(
            f"low_rank_markov_head: compress_weight rank={compress_weight.shape[0]} does not match "
            f"expand_weight's rank={expand_weight.shape[-1]} — the low-rank bottleneck must agree "
            "on both ends"
        )
    compressed = torch.nn.functional.linear(input, compress_weight)  # (batch, rank)
    return torch.nn.functional.linear(compressed, expand_weight)      # (batch, vocab)
