"""DSpark serve harness — run the speculative-decoding engine end to end.

This is the "serve mode" for the DSpark reference engine: it wires a backbone +
verifier into `SpeculativeEngine` and generates tokens speculatively. Training a
model and serving it speculatively are distinct concerns, so this stays a
separate entry point rather than being bolted into BrainBuilder's trainer.

Bring your own models by implementing the two tiny contracts:
  * backbone(ids) -> obj with `.hidden_states`  (batch, seq, hidden)
  * verifier(ids) -> obj with `.logits`          (batch, seq, vocab)
A real deployment passes trained transformers here (e.g. a small draft model as
the backbone and the full model as the verifier). With no models supplied, this
runs against tiny random stand-ins so the loop is demonstrable offline.

Usage:
    python -m dspark_system.serve --prompt-len 4 --new-tokens 8
"""
import argparse

import torch
import torch.nn as nn

from .config import DSparkConfig
from .engine import SpeculativeEngine


class _RandomBackbone(nn.Module):
    """Stand-in backbone. Replace with a trained draft model in production."""
    def __init__(self, vocab, hidden):
        super().__init__()
        self.embed = nn.Embedding(vocab, hidden)

    def forward(self, ids):
        out = type("O", (), {})()
        out.hidden_states = self.embed(ids)
        return out


class _RandomVerifier(nn.Module):
    """Stand-in verifier ("Boss"). Replace with the full model in production."""
    def __init__(self, vocab, hidden):
        super().__init__()
        self.embed = nn.Embedding(vocab, hidden)
        self.proj = nn.Linear(hidden, vocab)

    def forward(self, ids):
        out = type("O", (), {})()
        out.logits = self.proj(self.embed(ids))
        return out


def build_engine(config: DSparkConfig, backbone=None, verifier=None) -> SpeculativeEngine:
    """Assemble a SpeculativeEngine, defaulting to random stand-in models so the
    harness runs offline. Pass real modules to serve a trained model."""
    backbone = backbone or _RandomBackbone(config.vocab_size, config.hidden_dim)
    verifier = verifier or _RandomVerifier(config.vocab_size, config.hidden_dim)
    return SpeculativeEngine(backbone, verifier, config)


def main() -> None:
    parser = argparse.ArgumentParser(description="Run the DSpark speculative-decoding engine.")
    parser.add_argument("--prompt-len", type=int, default=4)
    parser.add_argument("--new-tokens", type=int, default=8)
    parser.add_argument("--vocab", type=int, default=256)
    parser.add_argument("--hidden", type=int, default=64)
    parser.add_argument("--seed", type=int, default=0)
    args = parser.parse_args()

    torch.manual_seed(args.seed)
    config = DSparkConfig(vocab_size=args.vocab, hidden_dim=args.hidden, max_draft_len=6, min_draft_len=2, markov_rank=8)
    engine = build_engine(config)

    prompt = torch.randint(0, args.vocab, (1, args.prompt_len))
    generated = engine.generate(prompt, max_new_tokens=args.new_tokens)

    new = generated[0, args.prompt_len:].tolist()
    print(f"prompt tokens : {prompt[0].tolist()}")
    print(f"generated     : {new}")
    print(f"total length  : {generated.shape[1]} (>= {args.prompt_len + args.new_tokens})")


if __name__ == "__main__":
    main()
