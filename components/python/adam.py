def step(params, grads, lr=0.001, betas=(0.9, 0.999), eps=1e-8, weight_decay=0.0):
    """One Adam update, treating this call as step t=1. `adam.edn`'s
    declared interface (params, grads -> updated_params) carries no
    optimizer state (m, v, t) across calls, so true multi-step Adam
    momentum/variance tracking isn't representable through it as-is —
    BrainBuilder's actual training loop (`PythonBridge::train_step`) uses
    `torch.optim.Adam` directly instead, which does track state correctly.
    This function is a real, correct single-step Adam update for standalone
    use of the `adam` component, not a drop-in stateful optimizer.
    """
    p = params.detach().clone()
    g = grads.detach()
    if weight_decay:
        g = g + weight_decay * p
    m_hat = g  # (1 - beta1) * g / (1 - beta1), step 1
    v_hat = g * g  # (1 - beta2) * g^2 / (1 - beta2), step 1
    return p - lr * m_hat / (v_hat.sqrt() + eps)
