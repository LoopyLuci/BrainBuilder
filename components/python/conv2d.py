import torch


def forward(x, weight):
    """x: (batch, in_ch, h, w), weight: (out_ch, in_ch, kh, kw) -> valid
    (no padding) 2D convolution, matching conv2d.edn's declared port shapes."""
    return torch.nn.functional.conv2d(x, weight)
