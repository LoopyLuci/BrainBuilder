import torch


def forward(input, p=0.1):
    """Elementwise dropout, matching dropout.edn's declared port shapes.
    Known limitation: there is no train/eval-mode signal plumbed from the
    scheduler into component hyperparameters yet, so this always drops units
    at rate `p` — including during a `predict` forward pass. Real eval-mode
    behavior (dropout disabled at inference) needs that mode flag threaded
    through `ExecutionPlan::forward` first; not silently faked here."""
    return torch.nn.functional.dropout(input, p=p, training=True)
