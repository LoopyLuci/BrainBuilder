def forward(x, w):
    """Elementwise scale: y = x * w. Matches the Arrow bridge's convention of
    one 1-D (length = batch size) tensor per dataset column — the honest
    building block for the current tabular-data pipeline, as opposed to
    `linear`, which needs a proper (batch, features) 2-D matrix the bridge
    doesn't produce yet (a real reshape design question for later)."""
    return x * w
