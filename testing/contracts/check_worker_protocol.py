#!/usr/bin/env python3
"""Cross-language contract drift-check for the Rust <-> Python worker protocol.

`core/src/interop/python.rs` talks to `components/python/_bb_worker.py` over a
line-delimited JSON request/response protocol keyed by an `"op"` field. There
is no compiler or test today that catches one side renaming/removing an op
without updating the other — it fails silently at *runtime* instead (Rust
gets back `{"ok": false, "error": "unknown op ..."}`, or the Python worker
raises `KeyError`/`ValueError` on a request it can't fully service).

This script parses both source files (stdlib-only, no build step needed) and
diffs the two op sets:
  * Rust-only ops  -> Rust can send a request the Python worker has no
    handler for. At request time this comes back as an explicit
    `{"ok": false, "error": "unknown op `...`"}` from the worker's dispatch
    loop (see `_bb_worker.py`'s `main()`), which the Rust side then has to
    surface/handle -- a silent-until-runtime failure that this check turns
    into a pre-merge failure instead.
  * Python-only ops -> dead handler code on the Python side, or a naming
    drift where Rust was renamed but the Python dict entry (and its handler)
    was not cleaned up.

Usage (from anywhere):
    python testing/contracts/check_worker_protocol.py

Exit code 0 and a one-line summary when the two sides agree; exit code 1 and
a detailed listing of the mismatched op(s) otherwise.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

# Compute paths relative to this file's location, not the caller's cwd, so
# this works no matter where it's invoked from.
_THIS_FILE = Path(__file__).resolve()
_REPO_ROOT = _THIS_FILE.parents[2]
RUST_PATH = _REPO_ROOT / "core" / "src" / "interop" / "python.rs"
PYTHON_PATH = _REPO_ROOT / "components" / "python" / "_bb_worker.py"


def parse_rust_ops(text: str) -> set[str]:
    """Every op the Rust side can send: string literals following `"op":`
    inside `serde_json::json!({...})` (or similar) request-construction
    blocks, e.g. `"op": "save_state_dict"`. We deliberately match the raw
    `"op"` key/value token pair rather than trying to parse the surrounding
    macro, since `json!` is not valid standalone JSON/Rust-AST-friendly
    syntax -- a regex on the literal text is the robust choice here.
    """
    # Matches:  "op": "some_name"   or   "op" : "some_name"
    # (allows optional whitespace around the colon, as real usage varies)
    pattern = re.compile(r'"op"\s*:\s*"([A-Za-z0-9_]+)"')
    return set(pattern.findall(text))


def parse_python_ops(text: str) -> set[str]:
    """Every op the Python worker actually dispatches: the keys of the
    `HANDLERS = {...}` dict literal that `main()`'s dispatch loop looks up
    via `HANDLERS.get(req.get("op"))`. We parse only the HANDLERS block
    (not every quoted string in the file) so unrelated string literals
    elsewhere (error messages, dict keys for LOSS_FNS/OPTIMIZERS, etc.)
    can't produce false-positive ops.
    """
    match = re.search(r'HANDLERS\s*=\s*\{(.*?)\n\}', text, re.DOTALL)
    if not match:
        raise RuntimeError(
            f"Could not find a `HANDLERS = {{...}}` dict literal in {PYTHON_PATH}. "
            "The dispatch-table parsing logic in this checker needs updating to "
            "match how ops are now registered."
        )
    body = match.group(1)
    # Matches the quoted key on each `"op_name": handler_fn,` line.
    key_pattern = re.compile(r'"([A-Za-z0-9_]+)"\s*:')
    return set(key_pattern.findall(body))


def main() -> int:
    if not RUST_PATH.is_file():
        print(f"ERROR: Rust interop source not found at {RUST_PATH}", file=sys.stderr)
        return 1
    if not PYTHON_PATH.is_file():
        print(f"ERROR: Python worker source not found at {PYTHON_PATH}", file=sys.stderr)
        return 1

    rust_ops = parse_rust_ops(RUST_PATH.read_text(encoding="utf-8"))
    python_ops = parse_python_ops(PYTHON_PATH.read_text(encoding="utf-8"))

    if not rust_ops:
        print(f"ERROR: found zero ops parsed from {RUST_PATH} -- parser is broken, not a clean protocol.",
              file=sys.stderr)
        return 1
    if not python_ops:
        print(f"ERROR: found zero ops parsed from {PYTHON_PATH} -- parser is broken, not a clean protocol.",
              file=sys.stderr)
        return 1

    rust_only = sorted(rust_ops - python_ops)
    python_only = sorted(python_ops - rust_ops)

    if not rust_only and not python_only:
        print(f"OK: {len(rust_ops)} ops in sync between Rust and Python worker protocol.")
        return 0

    lines = ["Worker protocol drift detected between:",
             f"  Rust : {RUST_PATH.relative_to(_REPO_ROOT)}",
             f"  Python: {PYTHON_PATH.relative_to(_REPO_ROOT)}",
             ""]

    if rust_only:
        lines.append(f"Rust-only ops ({len(rust_only)}) -- Rust can send these, but the Python worker "
                      "has no handler for them (dispatch falls through to "
                      "`{\"ok\": false, \"error\": \"unknown op ...\"}` at request time):")
        for op in rust_only:
            lines.append(f"    - {op!r}")
        lines.append(f"  Fix: add a handler for it to HANDLERS in {PYTHON_PATH.name}, "
                      "or remove the Rust call site if it's stale.")
        lines.append("")

    if python_only:
        lines.append(f"Python-only ops ({len(python_only)}) -- registered in HANDLERS but the Rust side "
                      "never sends a request for them (dead handler code, or a naming drift "
                      "where Rust was renamed without updating Python):")
        for op in python_only:
            lines.append(f"    - {op!r}")
        lines.append(f"  Fix: remove the stale handler from HANDLERS in {PYTHON_PATH.name}, "
                      f"or rename it to match the current Rust op name in {RUST_PATH.name}.")

    print("\n".join(lines), file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
