import torch


def forward(ids, weight):
    """ids: (batch, seq) real-valued token indices (BrainBuilder's tensor
    exchange is float32-only end to end — see _bb_worker.py), weight:
    (vocab_size, embedding_dim) -> (batch, seq, embedding_dim). Indices are
    cast to long for the lookup; values must be whole numbers within
    [0, vocab_size), matching embedding.edn's declared port shapes."""
    return torch.nn.functional.embedding(ids.long(), weight)
