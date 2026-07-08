"""DSpark serve harness — run the speculative-decoding engine end to end.

This is the "serve mode" for the DSpark reference engine: it wires a backbone +
verifier into `SpeculativeEngine` and generates tokens speculatively. Training a
model and serving it speculatively are distinct concerns, so this stays a
separate entry point rather than being bolted into BrainBuilder's trainer.

Bring your own models by implementing the two tiny contracts:
  * backbone(ids) -> obj with `.hidden_states`  (batch, seq, hidden)
  * verifier(ids) -> obj with `.logits`          (batch, seq, vocab)
With no `--model` supplied, this runs against tiny random stand-ins so the loop
is demonstrable offline. Pass `--model` to load a real checkpoint instead (any
HuggingFace `transformers` causal-LM checkpoint — local path or hub id) and get
real generated text plus real speculative-acceptance behavior.

Usage:
    python -m dspark_system.serve --prompt-len 4 --new-tokens 8
    python -m dspark_system.serve --model gpt2 --prompt "Once upon a time" --new-tokens 32
    python -m dspark_system.serve --model /path/to/big-model --draft-model /path/to/small-model
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


class HFBackboneAdapter(nn.Module):
    """Adapts a real `transformers` causal-LM checkpoint to the engine's
    backbone contract: `.hidden_states` of shape (batch, seq, hidden). Uses
    the model's own last-layer hidden state, not its head — the drafter,
    Markov head, and confidence head all consume that representation."""

    def __init__(self, hf_model: nn.Module):
        super().__init__()
        self.model = hf_model

    def forward(self, ids):
        result = self.model(ids, output_hidden_states=True)
        out = type("O", (), {})()
        out.hidden_states = result.hidden_states[-1]
        return out


class HFVerifierAdapter(nn.Module):
    """Adapts a real `transformers` causal-LM checkpoint to the engine's
    verifier ("boss") contract: `.logits` of shape (batch, seq, vocab)."""

    def __init__(self, hf_model: nn.Module):
        super().__init__()
        self.model = hf_model

    def forward(self, ids):
        out = type("O", (), {})()
        out.logits = self.model(ids).logits
        return out


def _load_hf_checkpoint(path: str):
    """Loads a real HuggingFace `transformers` checkpoint (local directory or
    hub id). Import is lazy and the dependency stays optional (not needed for
    the default random-stand-in path or the smoke tests) — fail loudly with a
    clear instruction rather than a bare ImportError if it's missing."""
    try:
        from transformers import AutoModelForCausalLM
    except ImportError as e:
        raise SystemExit(
            "Loading a real checkpoint needs the `transformers` package "
            "(`pip install transformers`). It is intentionally not a hard "
            "dependency of dspark_system, which otherwise runs on plain torch."
        ) from e
    model = AutoModelForCausalLM.from_pretrained(path)
    model.eval()
    return model


def _load_tokenizer(path: str):
    from transformers import AutoTokenizer
    return AutoTokenizer.from_pretrained(path)


def build_engine(config: DSparkConfig, backbone=None, verifier=None) -> SpeculativeEngine:
    """Assemble a SpeculativeEngine, defaulting to random stand-in models so the
    harness runs offline. Pass real modules to serve a trained model."""
    backbone = backbone or _RandomBackbone(config.vocab_size, config.hidden_dim)
    verifier = verifier or _RandomVerifier(config.vocab_size, config.hidden_dim)
    return SpeculativeEngine(backbone, verifier, config)


def build_engine_from_checkpoint(model_path: str, draft_model_path: str | None, config_overrides: dict) -> tuple[SpeculativeEngine, object]:
    """Loads a real checkpoint as the verifier ("boss"), and either a second,
    smaller checkpoint or the same checkpoint as the backbone ("intern") — a
    single model can speculatively decode against itself (self-speculation),
    which is still a real, useful configuration. Returns (engine, tokenizer)."""
    boss = _load_hf_checkpoint(model_path)
    intern = _load_hf_checkpoint(draft_model_path) if draft_model_path else boss
    hidden_dim = boss.config.hidden_size
    vocab_size = boss.config.vocab_size

    if draft_model_path and intern.config.hidden_size != hidden_dim:
        raise SystemExit(
            f"--draft-model `{draft_model_path}` has hidden_size={intern.config.hidden_size}, "
            f"but --model `{model_path}` has hidden_size={hidden_dim}. The drafter, Markov, and "
            "confidence heads are sized from the boss model's hidden_size, so a mismatched draft "
            "model would fail with an opaque tensor-shape error deep inside the engine — pick a "
            "draft model with the same hidden size, or omit --draft-model to self-speculate."
        )
    if draft_model_path and intern.config.vocab_size != vocab_size:
        raise SystemExit(
            f"--draft-model `{draft_model_path}` has vocab_size={intern.config.vocab_size}, "
            f"but --model `{model_path}` has vocab_size={vocab_size}. Rejection sampling compares "
            "token ids directly, so a mismatched tokenizer/vocab would silently produce nonsense "
            "acceptances instead of a clean error — pick a draft model that shares the boss's "
            "tokenizer/vocab, or omit --draft-model to self-speculate."
        )

    config = DSparkConfig(vocab_size=vocab_size, hidden_dim=hidden_dim, **config_overrides)
    engine = SpeculativeEngine(HFBackboneAdapter(intern), HFVerifierAdapter(boss), config)
    tokenizer = _load_tokenizer(model_path)
    return engine, tokenizer


def main() -> None:
    parser = argparse.ArgumentParser(description="Run the DSpark speculative-decoding engine.")
    parser.add_argument("--model", type=str, default=None,
                         help="Real checkpoint (local path or HF hub id) to serve as the verifier/boss. "
                              "Omit to run the offline demo with random stand-in models.")
    parser.add_argument("--draft-model", type=str, default=None,
                         help="Optional smaller checkpoint to serve as the drafter/intern backbone. "
                              "Defaults to --model itself (self-speculation) when omitted.")
    parser.add_argument("--prompt", type=str, default=None,
                         help="Text prompt, tokenized with --model's tokenizer. Only valid with --model.")
    parser.add_argument("--prompt-len", type=int, default=4, help="Random-prompt length (ignored with --prompt).")
    parser.add_argument("--new-tokens", type=int, default=8)
    parser.add_argument("--vocab", type=int, default=256, help="Vocab size for the random stand-in demo.")
    parser.add_argument("--hidden", type=int, default=64, help="Hidden size for the random stand-in demo.")
    parser.add_argument("--max-draft-len", type=int, default=6)
    parser.add_argument("--min-draft-len", type=int, default=2)
    parser.add_argument("--markov-rank", type=int, default=8)
    parser.add_argument("--seed", type=int, default=0)
    args = parser.parse_args()

    if args.prompt is not None and args.model is None:
        raise SystemExit("--prompt requires --model (it needs a real tokenizer).")

    torch.manual_seed(args.seed)

    if args.model:
        engine, tokenizer = build_engine_from_checkpoint(
            args.model, args.draft_model,
            dict(max_draft_len=args.max_draft_len, min_draft_len=args.min_draft_len, markov_rank=args.markov_rank),
        )
        if args.prompt is not None:
            prompt = tokenizer(args.prompt, return_tensors="pt").input_ids
        else:
            prompt = torch.randint(0, engine.config.vocab_size, (1, args.prompt_len))

        generated = engine.generate(prompt, max_new_tokens=args.new_tokens)
        new = generated[0, prompt.shape[1]:].tolist()
        print(f"prompt tokens : {prompt[0].tolist()}")
        print(f"generated     : {new}")
        if args.prompt is not None:
            print(f"generated text: {tokenizer.decode(new, skip_special_tokens=True)!r}")
        print(f"total length  : {generated.shape[1]} (>= {prompt.shape[1] + args.new_tokens})")
        return

    config = DSparkConfig(vocab_size=args.vocab, hidden_dim=args.hidden, max_draft_len=args.max_draft_len,
                           min_draft_len=args.min_draft_len, markov_rank=args.markov_rank)
    engine = build_engine(config)

    prompt = torch.randint(0, args.vocab, (1, args.prompt_len))
    generated = engine.generate(prompt, max_new_tokens=args.new_tokens)

    new = generated[0, args.prompt_len:].tolist()
    print(f"prompt tokens : {prompt[0].tolist()}")
    print(f"generated     : {new}")
    print(f"total length  : {generated.shape[1]} (>= {args.prompt_len + args.new_tokens})")


if __name__ == "__main__":
    main()
