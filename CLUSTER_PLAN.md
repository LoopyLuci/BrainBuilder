# BrainBuilder Distributed Training + Personal Cluster — Design Plan

## Context

You asked for two related but distinct systems:

1. **Job-based distributed training**: any machine can host or join a training
   job; the host sets resource requirements and per-client visibility/settings;
   clients choose what they contribute.
2. **The Cluster**: a persistent, always-on personal mesh of *all* of a user's
   devices (desktop, laptop, phone, tablet — "unlimited"), one elected
   **Manager**, everything else a **Node**, managed from a **Cluster Console**
   dashboard simple enough for a zero-knowledge user to set up flawlessly.

This is a genuinely large distributed-systems product — realistically
multiple weeks of work, not a single pass. This document is the honest,
concrete plan: what's built on already (the nervous system + libp2p
scaffold), the real architecture for both systems, and a phased delivery
order. I built **Phase 1** now (real code, tested on your actual two
machines' topology where testable); everything past that is scoped and
ordered, not pretended into existence.

**Reality check on "phone/tablet" and "unlimited devices" and "next 100
years"**: a phone/tablet Node needs a *different* runtime entirely (no Rust
subprocess model, no local Python/Racket/JVM) — realistically a thin
observer/control client (view cluster status, adjust settings) rather than a
compute Node, at least initially. "Unlimited" and "next 100 years" aren't
engineering requirements I can size against — I'm treating them as "the
architecture must not have an artificial device-count ceiling or a
dashboard that needs redesigning for features we add next year," which is a
real, buildable constraint, not a slogan.

---

## Part 1: The Cluster (persistent device mesh)

### Roles
- **Manager**: exactly one per Cluster at a time. Owns the Cluster's identity
  (a keypair-derived Cluster ID), the membership list, and the Console UI.
  Any device *can* be a Manager; it's a role, not special hardware — this
  matters for personal use (your desktop is naturally the Manager; if it's
  off, the laptop can take over — see Manager failover below).
- **Node**: any device joined to the Cluster. Advertises its own
  `DeviceProfile` (CPU cores, RAM, GPU(s) + VRAM, OS) and current
  availability. Executes work the Manager assigns, subject to the Node
  owner's own resource-allocation settings (see Part 2 — a Node's owner
  decides what it contributes, the Manager doesn't unilaterally seize
  resources).

### Identity & pairing (zero-knowledge-user requirement)
- Cluster identity = an Ed25519 keypair generated on first Manager setup
  (libp2p already uses this identity model — `PeerId` is already how
  `distributed.rs` addresses peers).
  - "**Create a Cluster**" on the Manager generates a short pairing code
    (6-digit, like a Wi-Fi/Bluetooth pairing flow) valid for ~5 minutes.
  - "**Join a Cluster**" on any other device: enter the code (or scan a QR
    code rendered by the Manager's Console — trivial to add once there's a
    Console webview). No manual key exchange, no config files, no IP
    addresses typed in.
- Transport: mDNS (already in `distributed.rs`) for same-LAN discovery — the
  common personal case (desktop + laptop on home Wi-Fi). For devices on
  different networks (laptop on the road, phone on cellular), the real next
  step is a libp2p relay/rendezvous point — see Phase 3.

### Cluster Console
A dashboard (new GUI surface, `gui/src/cluster/`) that only exists on the
Manager (Nodes get a lightweight "connected to cluster X, contributing Y" 
status view, not the full Console — this is what keeps "zero-knowledge user"
real: they see complexity only if they're the one managing it):
- Device list: every Node, live status (online/offline/busy), its
  `DeviceProfile`, and current allocation toggle (Part 2).
- Job list: running/queued/completed jobs, per-job device assignment.
- Manager handoff: "Make \<device\> the Manager" — transfers the Cluster's
  authoritative state (membership + job history) to another Node and
  re-points all Nodes at it. Needed for personal use: your desktop shouldn't
  be a single point of failure for using your laptop solo.
- Settings: default resource-contribution policy, pairing-code generation,
  audit log (every provenance-relevant event — this plugs directly into the
  already-real `ProvenanceStore`).

### Data model (new crate module `core/src/cluster/`)
```rust
struct DeviceProfile {
    peer_id: PeerId,
    display_name: String,       // user-editable, e.g. "Desktop", "Laptop"
    os: String,
    cpu_cores: usize,           // system_resources::logical_cpu_count()
    ram_total_bytes: u64,       // system_resources::total_physical_memory_bytes()
    gpus: Vec<GpuProfile>,      // name, vram_bytes, backend (cuda/wgpu/metal)
}

struct ClusterMembership {
    cluster_id: ClusterId,      // derived from Manager's keypair
    manager: PeerId,
    nodes: HashMap<PeerId, (DeviceProfile, NodeStatus)>,
}
```

---

## Part 2: Job-based distributed training

### Job lifecycle
1. **Host creates a Job** from their Cluster Console (or standalone, without
   a persistent Cluster — a Job can exist ad hoc between any two BrainBuilder
   instances that discover each other, same as today's `distributed.rs`
   mDNS peer, no Cluster required — Cluster and Job are independent features
   that compose).
2. **`JobSpec`** (host-authored):
   ```rust
   struct JobSpec {
       graph: BBIRGraph,                     // reuse — already real
       min_requirements: ResourceRequirement, // floor to even join
       visible_to_clients: JobVisibility,      // what clients can see
       client_settings: JobClientPermissions,  // what clients can change
   }
   struct ResourceRequirement {
       min_cpu_cores: Option<usize>,
       min_ram_bytes: Option<u64>,
       min_vram_bytes: Option<u64>,
       require_gpu_backend: Option<GpuBackend>, // e.g. must have CUDA, or any
   }
   struct JobVisibility {
       show_loss_curve: bool,
       show_other_clients: bool,
       show_graph_structure: bool,   // hide proprietary architectures from clients
   }
   struct JobClientPermissions {
       client_can_set_batch_share: bool,   // how much of each batch a client trains
       client_can_set_max_vram: bool,
       client_can_leave_anytime: bool,     // vs. host-locked for the job's duration
   }
   ```
3. **Discovery**: the Job is advertised over the same libp2p mDNS/gossip
   already scaffolded; a client sees a Job list (name, host, min
   requirements) before deciding to join.
4. **Client joins**: client's `DeviceProfile` is checked against
   `min_requirements` locally (a rejected join never even contacts the host
   with unusable capacity) and remotely (host re-validates — never trust the
   client's self-report for anything security-relevant). Client then sets
   their *own* contribution within whatever `JobClientPermissions` allows
   (e.g. "use up to 8GB VRAM, up to 4 CPU threads").
5. **Data-parallel training** (the actual distributed-training mechanism):
   - Manager/host partitions each batch across joined clients proportional to
     their declared contribution.
   - Each client runs a normal local `train_step` (the exact same, already-real
     `PythonBridge::train_step` — no new training code needed) on its shard.
   - Gradients (not raw data) are sent back to the host and averaged
     (synchronous all-reduce to start — simplest correct approach; async/
     stale-gradient schemes are a real future optimization, not needed for
     correctness first).
   - Host applies the averaged gradient via the optimizer, broadcasts updated
     weights back to clients for the next step.
   - This reuses 100% of the existing real training path — the only new work
     is the network transport for tensors + gradients and the aggregation
     step.
6. **Toggling visibility mid-job**: since `JobVisibility`/`JobClientPermissions`
   are just state the host can update and broadcast, "toggle what clients
   see" is a live config push, not a re-negotiation — cheap to implement once
   the control-plane message types exist.

### Where this plugs into what's already built
- **Transport**: `core/src/runtime/distributed.rs`'s libp2p `Swarm` is the
  literal foundation — it currently only does mDNS peer discovery; Phase 2
  adds gossipsub (job announcements) + request-response (join requests,
  gradient exchange) behaviours, both real libp2p protocols, not custom ones.
- **Security**: a remote peer is fundamentally the same trust problem as a
  local component — it runs code-adjacent operations (accepts a graph,
  computes gradients) on your machine. It becomes a `ComponentRuntime` in the
  nervous system, with `Capabilities` scoped per-Job (no filesystem access
  beyond a job-specific scratch dir, no ability to read the host's other
  data) rather than trusted implicitly. This is the direct payoff of having
  built the nervous system first.
- **Resource limits**: `system_resources::resolve_memory_limit_bytes` (just
  built) is exactly the mechanism `ResourceRequirement`/client contribution
  caps need — real hardware queries, not guesses.

---

## Phased delivery order

**Phase 1 — done.** `core/src/cluster/device_profile.rs` — real
`DeviceProfile` capture (CPU/RAM via `system_resources`, GPU via `wgpu`'s
adapter enumeration), verified against your actual desktop (24 logical
cores, 63.9GiB RAM, real RX 7900 XTX found across Vulkan/DX12/GL). Plus
`ClusterMembership` (add/remove nodes, Manager handoff), tested.

**Phase 2 — done.** Real libp2p `gossipsub` (job announcements) +
`request-response`/CBOR (join negotiation) behaviours in `distributed.rs`.
Proven in `core/tests/distributed_job_protocol.rs`: two independent,
real libp2p swarms (separate keypairs, separate TCP listeners) discover each
other via mDNS, exchange a real noise-encrypted/yamux-multiplexed
connection, the host gossips a `JobAnnouncement`, and the client sends a
real `JobRequest::Join` that the host accepts — no mocks, this is the actual
wire protocol two real machines would use. **Not yet built**: this doesn't
wire to the nervous system as a `ComponentRuntime` yet (remote-peer
capability scoping), and it doesn't move any tensors/gradients — it's the
discovery+negotiation layer, which is Phase 2's correct scope. Gradient
exchange needs a `PythonBridge` worker protocol extension (separate
`compute_gradients`/`apply_averaged_gradients` ops, since today's
`train_step` computes loss→backward→optimizer.step() as one atomic worker
call and never exposes a raw gradient to Rust) — that's the next concrete,
bounded piece of work, not yet started.

**Gradient exchange — done.** `PythonBridge::compute_gradients` and
`PythonBridge::apply_averaged_gradients` (`core/src/interop/python.rs`), backed
by new `compute_gradients`/`apply_averaged_gradients` ops in
`components/python/_bb_worker.py`. This splits the previously-atomic
`train_step` (forward→loss→backward→optimizer.step()) into two real worker
calls: clients run forward+backward on their own data shard and return a raw
gradient (no optimizer step), a host averages gradients from all clients and
applies exactly one optimizer step. Proven in
`core/tests/distributed_gradients.rs`: two independent `PythonBridge`
subprocess workers each compute a gradient on a disjoint 2-sample shard of
`y = 3x`, a third worker applies the averaged gradient, and — over 200 real
steps — this converges to the true weight (`3.0 ± 0.05`) exactly like the
non-distributed `train_step` path does. **Network transport — done.** `JobRequest::SubmitGradients` (direct
request-response) carries real `TensorPayload`s from client to host;
`GradientAggregator` (`core/src/cluster/aggregator.rs`) accumulates them
per-step and produces a real element-wise average (fails closed on a shape
mismatch rather than silently corrupting the average); `WeightUpdate`
broadcasts the result back over a per-job gossipsub topic
(`brainbuilder/jobs/<job_id>/weights`). Proven in
`core/tests/distributed_gradient_network.rs`: three independent real libp2p
swarms (host + 2 clients) — two clients each send a real gradient tensor to
the host over request-response, the host averages them with
`GradientAggregator` and publishes the result, and **both clients
independently receive and verify the identical averaged weights** over the
network. This is the actual wire path two real machines (your desktop +
NixOS box) would use — not simulated in one process like
`distributed_gradients.rs`.

**Not yet built**: the per-job data sharding/scheduling logic that decides
which client trains which rows each step (currently each client's shard is
assumed pre-assigned), and wiring this transport to actually invoke
`PythonBridge::compute_gradients`/`apply_averaged_gradients` end-to-end in one
running process (today the network test and the gradient-math test are
proven separately; gluing them together is straightforward given both are
independently verified, but hasn't been done as one live example).

**Phase 3 — done.** Real keypair-derived Cluster identity
(`core/src/cluster/identity.rs`): `ClusterIdentity::load_or_create` generates
an Ed25519 keypair on first run and persists it to disk, so a Cluster's
`ClusterId` (now the Manager's real `PeerId`, not a random UUID — see
`membership.rs`) is stable across restarts. Real pairing codes
(`core/src/cluster/pairing.rs`): `PairingRegistry` issues one-time-use,
real-wall-clock-expiring 6-digit codes; a joining device presents one over a
dedicated `/brainbuilder/pair/1.0.0` request-response protocol
(`ClusterRequest::Pair`/`ClusterResponse`); Manager handoff is a real
gossipsub broadcast (`ManagerHandoff` on `brainbuilder/cluster/<id>/control`).
Proven in `core/tests/cluster_pairing.rs`: two independent real libp2p
swarms — a "Manager" and a "joining device" — exchange a real pairing code,
the Manager adds the joiner to its `ClusterMembership`, hands off Manager
status, and broadcasts it; the joiner (a genuinely separate process/swarm)
receives and verifies the handoff over the real network. This test also
caught and fixed a real bug: `ClusterId`'s original `serialize_bytes`-based
Serialize impl decoded incorrectly under the CBOR codec `request_response`
uses on the wire (works fine over JSON, which is why earlier gossipsub-only
messages never hit it) — fixed by routing through `Vec<u8>`'s normal
sequence-based (de)serialization instead.

**Also done (glue): `core/tests/distributed_training_live.rs`** — ties
together Phase 2's transport, the gradient-aggregation math, and
`PythonBridge::compute_gradients`/`apply_averaged_gradients` into one live,
multi-step (60-step) run: two real subprocess workers each train on a
disjoint shard over real libp2p swarms, converging to the true weight exactly
like the single-machine and in-process-only tests already proved separately.

**Phase 4 — v1 done.** A real Cluster Console: `gui/src-tauri/src/cluster_actor.rs`
runs the live `P2PCluster` swarm as a background Tauri-managed actor (real
identity loaded from `<app data dir>/cluster_identity.key`, so this device's
`PeerId` survives restarts), driven by the exact protocol already proven in
`core/tests/cluster_pairing.rs` — no mocked data. Four Tauri commands
(`get_cluster_status`, `create_cluster`, `generate_pairing_code`,
`join_cluster_with_code`) back a real UI (`gui/src/cluster/ClusterConsole.tsx`):
create-or-join flow, a live (3s-polled) device list, and pairing-code
generation for the Manager. Verified: backend (`cargo build`) and frontend
(`tsc --noEmit`) both compile clean, and the full Tauri app boots and runs
without panicking (`npx tauri dev`, confirmed via process list — the native
window can't be screenshotted with available tooling, so this is a startup/
stability check, not a full interactive walkthrough of every button).

Fixed a real, unrelated pre-existing bug found while verifying this: the
components directory path resolved relative to the wrong cwd when launched
via `tauri dev` (whose cwd is `gui/src-tauri`, not `gui/`), so the app
crashed on startup before the Cluster actor even got a chance to run — now
tries both `../components` and `../../components` and uses whichever exists.

**Not yet built** (explicitly out of this v1's scope): Manager handoff has
no UI trigger yet (the wire mechanism from Phase 3 works, tested
independently); no job-creation/job-list UI (Part 2's `JobSpec` isn't wired
to any Tauri command yet — Phase 4 focused on the Cluster half, not Jobs);
no QR-code pairing (numeric code entry only); "join" only tries currently
mDNS-discovered peers at the moment `Join` is clicked, so a peer discovered
a moment later isn't retried within that single click (a full retry loop is
a natural follow-up, not a fundamental limitation).

**Phase 5 — thin mobile/tablet observer client: done (v1, LAN-only).** A
phone/tablet doesn't get a Rust subprocess runtime or a Node role — that's a
different device class, not a smaller version of the desktop app. What it
gets instead: a real HTTP server (`gui/src-tauri/src/observer_server.rs`,
axum) run by the Manager, serving a live, auto-refreshing, mobile-styled HTML
page (`GET /`) plus a JSON status endpoint (`GET /api/status`) backed by the
exact same `ClusterHandle::status()` the desktop Console uses — not a mock or
a separate data path. The Manager's real LAN IP is auto-detected (the
standard UDP-routing-table trick, no external calls made) and shown in the
desktop Console under "Phone / Tablet Access." Verified for real: launched
the full app (`npx tauri dev`), confirmed the server is actually listening
(`netstat`), and `curl`'d both `/api/status` (returned real live JSON
matching this device's actual `PeerId`) and `/` (returned the real served
HTML) — not asserted from source reading alone.

**Not yet built**: cross-network transport (relay/rendezvous for devices not
on the same LAN — a laptop on the road, or a phone off Wi-Fi), gradient
compression, async/stale-gradient training for weak links, and — the bigger
step up from this v1 — a phone/tablet ever acting as a Node (contributing
compute) rather than a read-only observer, which would need a genuinely
different runtime (e.g. a WASM or native mobile execution path), not just
more polish on this HTTP view.

## What I'd like your sign-off on before Phase 2
Phase 1 is unopinionated (it's just real device introspection). Phase 2
starts making real protocol/security decisions I'd rather confirm first:
- Gradient averaging: synchronous (simple, correct, stalls on the slowest
  client) vs. bounded-async (faster, more complex, needs staleness handling)
  — I'd start synchronous and revisit.
- Should a client's identity be persisted/trusted across jobs (so "my
  laptop" doesn't need to re-prove itself every time), or re-verified every
  join? I'd default to persisted *within a Cluster* (Part 1 identity) and
  re-verified for ad hoc Jobs outside a Cluster.
