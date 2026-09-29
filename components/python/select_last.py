def forward(input):
    """input: (batch, seq, features) -> (batch, features), taking the last
    timestep — the standard "next-token prediction" read-out from a
    transformer's sequence output, matching select_last.edn's declared port
    shapes. A real (differentiable) slice, not a copy/detach."""
    return input[:, -1, :]
