import torch


def forward(input, weight, bias, eps=1e-5):
    """input: (batch, features), weight/bias: (features,) -> layer-normalizes
    over the last dimension, matching layernorm.edn's declared port shapes.
    `weight`'s own shape (not the `features` hyperparam) drives torch's
    `normalized_shape` so this stays correct even if the hyperparam and the
    actual initialized weight ever disagree."""
    return torch.nn.functional.layer_norm(input, weight.shape, weight, bias, eps)
