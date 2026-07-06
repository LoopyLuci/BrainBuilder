import torch


def forward(input, weight):
    """input: (batch, in_features), weight: (out_features, in_features) ->
    (batch, out_features), matching linear.edn's declared port shapes."""
    return torch.nn.functional.linear(input, weight)
