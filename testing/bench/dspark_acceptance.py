#!/usr/bin/env python3
"""DSpark speculative-decoding acceptance-rate regression benchmark.

Even with the random stand-in backbone/verifier from `dspark_system.serve`
(no trained weights), `SpeculativeEngine.generate()`'s accept/reject
mechanics are fully deterministic given a torch seed. That makes "tokens
produced per boss-model call" a cheap, seed-stable structural metric: it
doesn't tell you anything about generation *quality*, but it tells you
whether rejection sampling is still doing SOMETHING useful mechanically.

Why this bound is real, not arbitrary (measured, not guessed -- see below):
every boss call happens through exactly one of `_boss_step` (always
exactly 1 token) or `_verify` (0..valid tokens, and 0 costs a boss call too,
via the subsequent `_boss_step` fallback). With a `vocab_size=256` random
boss/drafter, an accepted draft run is rare per `_verify` call, so measured
tokens-per-boss-call clusters tightly in the 0.80-1.00 range across seeds
(empirically: min 0.80, max 1.00, avg ~0.93 over 10 fixed seeds -- see the
`per_seed` numbers this script prints). That narrow, seed-stable band is
exactly what makes it a useful regression signal:
  * An always-reject bug in `_verify` (e.g. `keep` hard-coded to 0, or the
    confidence cutoff `valid` always 0) forces every step through
    `_boss_step` alone, which is INDISTINGUISHABLE from the low end of the
    normal band by this metric alone at small N -- so this benchmark's real
    job is catching things that push the metric OUTSIDE the observed
    healthy band, in particular:
  * A broken accept path that starts accepting every drafted token
    regardless of boss agreement (an always-accept bug) inflates
    tokens-per-boss-call sharply above the observed band (each `_verify`
    call would then yield up to `max_draft_len` tokens instead of 0-1),
    and/or violates `_verify`'s own contiguous-prefix invariant.
  * A counting/engine bug that stops incrementing `boss_calls` entirely, or
    a `generate()` bug that stops advancing `current`, either divides by a
    near-zero call count (metric explodes) or never terminates -- both are
    easy to catch with a floor.

This benchmark asserts tokens_per_boss_call stays within
[MIN_TOKENS_PER_BOSS_CALL, MAX_TOKENS_PER_BOSS_CALL] for every one of N
fixed seeds, with a fixed `DSparkConfig`. The band is set with headroom
around the measured 0.80-1.00 range (0.5 to 2.0) so ordinary run-to-run
determinism holds it comfortably inside, while a structural break (metric
collapsing toward 0, or an always-accept bug driving it well past 2x) trips
the check.

Usage (from repo root, so `dspark_system` is importable):
    PYTHONPATH=. python testing/bench/dspark_acceptance.py
  or, equivalently on Windows PowerShell:
    $env:PYTHONPATH="."; python testing/bench/dspark_acceptance.py

Exits 0 with a small JSON summary on stdout (and writes the same JSON to
`testing/bench/last_result.json`) when all seeds are within bounds; exits 1
with the actual per-seed numbers if any seed violates the bound.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

_THIS_FILE = Path(__file__).resolve()
_REPO_ROOT = _THIS_FILE.parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

import torch  # noqa: E402

from dspark_system.config import DSparkConfig  # noqa: E402
from dspark_system.serve import build_engine  # noqa: E402

SEEDS = list(range(10))
PROMPT_LEN = 4
NEW_TOKENS = 24
# Bounds chosen with headroom around the measured healthy range (0.80-1.00
# across 10 fixed seeds with the CONFIG below) -- see module docstring.
MIN_TOKENS_PER_BOSS_CALL = 0.5
MAX_TOKENS_PER_BOSS_CALL = 2.0

CONFIG = DSparkConfig(
    vocab_size=256,
    hidden_dim=64,
    max_draft_len=6,
    min_draft_len=2,
    markov_rank=8,
)

RESULT_PATH = _THIS_FILE.parent / "last_result.json"


def run_seed(seed: int) -> dict:
    torch.manual_seed(seed)
    engine = build_engine(CONFIG)
    prompt = torch.randint(0, CONFIG.vocab_size, (1, PROMPT_LEN))

    out = engine.generate(prompt, max_new_tokens=NEW_TOKENS)

    tokens_produced = out.shape[1] - prompt.shape[1]
    boss_calls = engine.boss_calls
    tokens_per_boss_call = tokens_produced / boss_calls if boss_calls else float("nan")

    return {
        "seed": seed,
        "tokens_produced": tokens_produced,
        "boss_calls": boss_calls,
        "tokens_per_boss_call": tokens_per_boss_call,
    }


def main() -> int:
    per_seed = [run_seed(s) for s in SEEDS]

    violations = [
        r for r in per_seed
        if not (MIN_TOKENS_PER_BOSS_CALL <= r["tokens_per_boss_call"] <= MAX_TOKENS_PER_BOSS_CALL)
    ]

    avg_tpbc = sum(r["tokens_per_boss_call"] for r in per_seed) / len(per_seed)
    summary = {
        "seeds": SEEDS,
        "prompt_len": PROMPT_LEN,
        "new_tokens": NEW_TOKENS,
        "config": {
            "vocab_size": CONFIG.vocab_size,
            "hidden_dim": CONFIG.hidden_dim,
            "max_draft_len": CONFIG.max_draft_len,
            "min_draft_len": CONFIG.min_draft_len,
            "markov_rank": CONFIG.markov_rank,
        },
        "tokens_per_boss_call_bounds": [MIN_TOKENS_PER_BOSS_CALL, MAX_TOKENS_PER_BOSS_CALL],
        "avg_tokens_per_boss_call": avg_tpbc,
        "per_seed": per_seed,
        "pass": not violations,
    }

    print(json.dumps(summary, indent=2))
    RESULT_PATH.write_text(json.dumps(summary, indent=2), encoding="utf-8")

    if violations:
        print(
            f"\nFAIL: {len(violations)}/{len(SEEDS)} seed(s) fell outside "
            f"tokens_per_boss_call bounds [{MIN_TOKENS_PER_BOSS_CALL}, {MAX_TOKENS_PER_BOSS_CALL}] "
            "(speculative accept/reject mechanics look structurally broken -- e.g. "
            "an always-reject or always-accept bug in `_verify`, or a broken "
            "confidence cutoff/boss-call counter):",
            file=sys.stderr,
        )
        for r in violations:
            print(f"    seed={r['seed']}: tokens_per_boss_call={r['tokens_per_boss_call']:.4f} "
                  f"(tokens_produced={r['tokens_produced']}, boss_calls={r['boss_calls']})",
                  file=sys.stderr)
        return 1

    print(f"\nOK: {len(SEEDS)} seeds within bounds "
          f"(avg tokens_per_boss_call={avg_tpbc:.4f}, "
          f"bounds=[{MIN_TOKENS_PER_BOSS_CALL}, {MAX_TOKENS_PER_BOSS_CALL}]).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
