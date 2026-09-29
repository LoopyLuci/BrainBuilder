# Component Synthesis

`core/src/synthesis/mod.rs` generates brand-new components — an EDN descriptor plus a
Python kernel — from a plain-English description, growing the
[component library](component-library.md) open-endedly instead of merely assembling
graphs from a fixed set. Everything synthesized is untrusted until it clears a
three-gate "gauntlet."

## Prompting

`build_synthesis_prompt(existing_names)` shows the model the exact EDN dialect and
kernel shape it must produce, and forbids reusing an existing component name.
`build_repair_request(description, previous_json, failure)` builds a self-repair turn
that echoes the model's own failed output plus the exact validator error, giving the
model one chance to fix a specific, named problem rather than guessing again from
scratch.

## The gauntlet

### Gate 1 — Parse

`parse_synthesis_output` deserializes the model's JSON into `RawSynthesis{name,
descriptor_edn, python_code, smoke_test}`: rejects an empty name, rejects if the name
is already taken in the registry, `edn_rs::from_str`s the descriptor into a
`ComponentDescriptor`, and requires `descriptor.name == parsed.name`.

### Gate 2 — Structure

`validate_descriptor_self(&descriptor)` reuses the system's own
[self-consistency checks](component-system.md#validation-core-srccomponentvalidationrs).
Then an entry-fn check: resolve the descriptor's implementation entry (default
`"forward"`) and require the kernel source literally contains `"def {entry}"` — a cheap
guard before paying for a sandboxed run. Finally the smoke test must supply one input
shape per declared input port and a non-empty expected output shape.

### Gate 3 — Sandboxed smoke test

`run_smoke_test` writes the kernel and an auto-built driver script
(`build_smoke_driver`, which constructs `torch.randn`/`torch.randint` synthetic tensors
per declared dtype, calls `kernel.{entry}(*args)`, and prints the output shape as JSON)
into a throwaway temp directory. It runs via
`Supervisor::run_checked_named("synthesis-smoke", …)` with
`Capabilities::none().allow_read(&dir).with_timeout(30s).with_memory_limit(1 GiB)` —
read-only access to that one temp dir, no network, a wall-clock and memory ceiling.
`PYTHONPATH` entries are additionally read-granted so `import torch` resolves. This
routes through exactly the same
[nervous-system sandbox](runtime-and-devices.md#the-nervous-system-sandbox) that
Racket/Clojure and the Python worker use — synthesized code gets no special exemption.

`interpret_smoke_output` compares the printed shape to the expected shape: a non-zero
exit is reported as "kernel crashed in the sandbox" with stderr; a shape mismatch is
reported explicitly. The temp dir is best-effort cleaned up afterward.

## Install

`install_synthesized` — only to be called after a passing smoke test — writes
`components_dir/python/<sanitized-name>.py` and `components_dir/<sanitized-name>.edn`,
returning the descriptor path for hot-registration.

## Tauri commands

- **`synthesize_component(description, selector, state)`** builds the prompt off the
  live registry (dropping the lock before the network `.await`), then loops up to
  `MAX_ATTEMPTS = 2`: call the provider, run gates 1–2, and if that passes run gate 3.
  On any failure with an attempt remaining, it builds a repair request and retries; on
  final failure it returns a descriptive error. On success it returns
  descriptor/kernel/smoke_test/smoke-report/attempt-count as JSON — **nothing is
  installed yet at this point**.
- **`install_synthesized_component(component_json, state)`** never trusts the round
  trip: re-parses and re-validates against the *current* registry (catching a name
  taken since synthesis), re-runs the sandboxed smoke test, refuses install if it
  fails, then calls `synthesis::install_synthesized` and
  `orchestrator.install_component(&edn_path)` to hot-register the component into the
  palette with no restart.
- **`install_component(path, state)`** is the older, general "install a component from
  a local file path" command, backed by `Orchestrator::install_component`.

## GUI

The [Synthesize panel](gui-panels.md#llm--synthesis--agent-gui-side-only) is the
frontend: a description textarea plus a provider/model selector, showing a smoke-test
pass/fail chip, an "auto-repaired" chip if more than one attempt was needed, and the
generated Python source. "Add to canvas" / "Palette only" call
`install_synthesized_component`.

## Why this design

A synthesized component is treated exactly like an unvetted third-party dependency: it
must structurally validate, and it must actually run correctly on real (if tiny)
tensors inside a sandbox with no filesystem or network access beyond what it strictly
needs, before it's ever installed where a real training run could execute it
unsandboxed via the normal [Python worker](interop.md#python-worker-componentspython_bb_workerpy)
path. See the [Live Verification Checklist](VERIFICATION.md#2-component-synthesis--train-phase-b)
for the manual pass/fail criteria used to verify this end to end.
