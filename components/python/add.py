def forward(a, b):
    """Elementwise sum — the residual/skip-connection primitive, matching
    add.edn's declared port shapes."""
    return a + b
