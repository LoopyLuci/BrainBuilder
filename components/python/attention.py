import torch


def forward(input, q_weight, k_weight, v_weight, out_weight, num_heads=4):
    """Real multi-head self-attention (Vaswani et al.): input (batch, seq,
    features) is projected to Q/K/V by q_weight/k_weight/v_weight (each
    (features, features)), split into `num_heads` heads, scaled-dot-product
    attention is computed per head, heads are concatenated back, and
    out_weight projects the result — matching attention.edn's declared port
    shapes. `features` must be evenly divisible by `num_heads`."""
    batch, seq, features = input.shape
    if features % num_heads != 0:
        raise ValueError(f"attention: features={features} not divisible by num_heads={num_heads}")
    head_dim = features // num_heads

    def project_and_split(x, weight):
        projected = torch.nn.functional.linear(x, weight)
        return projected.view(batch, seq, num_heads, head_dim).transpose(1, 2)

    q = project_and_split(input, q_weight)
    k = project_and_split(input, k_weight)
    v = project_and_split(input, v_weight)

    scores = torch.matmul(q, k.transpose(-2, -1)) / (head_dim ** 0.5)
    attn_weights = torch.nn.functional.softmax(scores, dim=-1)
    context = torch.matmul(attn_weights, v)

    context = context.transpose(1, 2).contiguous().view(batch, seq, features)
    return torch.nn.functional.linear(context, out_weight)
