import torch


def forward(input, p=0.1, training=True):
    """Elementwise dropout, matching dropout.edn's declared port shapes.
    `training` defaults to True so every existing train_step/compute_gradients
    call (which never sets it) keeps dropping units exactly as before.
    `ExecutionPlan::forward` — the inference-only path behind Predict, batch
    predict, the local serve endpoint, and feature importance — explicitly
    passes `training=False`, so real predictions are deterministic instead of
    randomly dropping units the way a training step does."""
    return torch.nn.functional.dropout(input, p=p, training=training)
