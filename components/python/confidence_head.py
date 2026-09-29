import torch


def forward(input, hidden_weight, score_weight):
    """DSpark Confidence Head. Emits a 0..1 probability that a drafted token
    will be accepted by the verifier ("Boss"). The speculative engine uses this
    to terminate a draft early the moment confidence dips below its threshold,
    which is what lifts draft acceptance rates dramatically. Matches
    confidence_head.edn's port shapes.

    input:         (batch, features)
    hidden_weight: (proj, features)
    score_weight:  (1, proj)
    -> output:     (batch,)  acceptance probability in [0, 1]
    """
    if input.shape[-1] != hidden_weight.shape[-1]:
        raise ValueError(
            f"confidence_head: input features={input.shape[-1]} does not match "
            f"hidden_weight's in_features={hidden_weight.shape[-1]}"
        )
    if hidden_weight.shape[0] != score_weight.shape[-1]:
        raise ValueError(
            f"confidence_head: hidden_weight's proj dim={hidden_weight.shape[0]} does not match "
            f"score_weight's in_features={score_weight.shape[-1]}"
        )
    hidden = torch.nn.functional.silu(torch.nn.functional.linear(input, hidden_weight))
    score = torch.nn.functional.linear(hidden, score_weight)  # (batch, 1)
    return torch.sigmoid(score).squeeze(-1)                    # (batch,)
