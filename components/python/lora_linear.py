import torch


def forward(input, weight, lora_a, lora_b, alpha=8.0):
    """LoRA fine-tuning (Hu et al.): `weight` is the frozen base layer
    (out_features, in_features) — `:trainable false` in lora_linear.edn keeps
    the optimizer from ever touching it. `lora_a` (rank, in_features) and
    `lora_b` (out_features, rank) are the only trainable parameters; `lora_b`
    starts at zero (`:zero-init true`) so training begins as an exact no-op
    on the frozen base, matching the reference LoRA initialization. Scaling
    by `alpha / rank` (rather than a bare `alpha`) is the paper's convention
    so the adapter's effective magnitude doesn't blow up as rank grows."""
    base = torch.nn.functional.linear(input, weight)
    rank = lora_a.shape[0]
    delta = torch.nn.functional.linear(torch.nn.functional.linear(input, lora_a), lora_b)
    return base + delta * (alpha / rank)
