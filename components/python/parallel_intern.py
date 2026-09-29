import torch


def forward(input, heads_weight):
    """DSpark Parallel Intern (drafter). Predicts a block of `draft_len` future
    tokens in a single pass — the "Intern" that speeds generation by drafting
    many tokens at once for the "Boss" to verify in parallel. Each of the
    `draft_len` positions has its own projection head; here they are stacked
    into one weight tensor and applied in a single einsum. Matches
    parallel_intern.edn's port shapes.

    input:        (batch, features)
    heads_weight: (draft_len, vocab, features)
    -> output:    (batch, draft_len, vocab)  per-position logits

    Predicting the whole block simultaneously is what causes "suffix decay"
    (later positions are guessed with less context) — pair this with
    low_rank_markov_head + confidence_head to counter it.
    """
    if input.shape[-1] != heads_weight.shape[-1]:
        raise ValueError(
            f"parallel_intern: input features={input.shape[-1]} does not match "
            f"heads_weight's features={heads_weight.shape[-1]}"
        )
    # b=batch, d=draft_len, v=vocab, f=features
    return torch.einsum("bf,dvf->bdv", input, heads_weight)
