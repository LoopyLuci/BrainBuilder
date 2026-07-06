import torch


def forward(input):
    """Elementwise GELU activation, matching gelu.edn's declared port shapes."""
    return torch.nn.functional.gelu(input)
