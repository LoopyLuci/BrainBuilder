# Cluster & Distributed Training

`core/src/cluster/` implements two related but distinct systems, per the design doc
`CLUSTER_PLAN.md` at the repo root:

1. **The Cluster** — a persistent personal device mesh with an elected Manager and
   member Nodes, managed via a Console UI.
2. **Job-based distributed training** — ad hoc, host/client, and doesn't require a
   persistent Cluster to exist first.

> `core/src/cluster/mod.rs`'s own module doc comment is stale — it describes only
> "Phase 1" (device introspection + membership model) as done, while `CLUSTER_PLAN.md`
> documents Phases 1 through 5 as complete, with explicit call-outs of what's *not* yet
> built per phase. This doc follows `CLUSTER_PLAN.md` as the more current source.

## Implemented pieces

| File | What it does |
|---|---|
| `device_profile.rs` | `DeviceProfile::capture` does real CPU/RAM introspection and real GPU enumeration via wgpu (gated on the `wgpu` feature). `meets(&ResourceRequirement)` fails closed on missing data |
| `membership.rs` | `ClusterId(PeerId)` — the Cluster's identity **is** its Manager's real libp2p `PeerId`, not a separate UUID. Custom `Serialize`/`Deserialize` route through `Vec<u8>` sequence encoding rather than `serialize_bytes` because a real decode mismatch was found under the CBOR codec `request_response` uses on the wire. `NodeStatus`: `Online`/`Busy`/`Offline` |
| `protocol.rs` | The full wire protocol: `JobAnnouncement` (gossiped), `JobRequest::{Join, SubmitGradients}` / `JobResponse::{Accepted, Rejected, GradientsAcked}` (request-response), `TensorPayload` (flattened shape + `Vec<f32>` — the network analogue of `PythonBridge`'s scratch-file tensors, see [Interop](interop.md)), `WeightUpdate` (gossiped per-job), `ClusterRequest::Pair`/`ClusterResponse::{Paired, Rejected}` (pairing), `ManagerHandoff` (gossiped control message) |
| `aggregator.rs` | `GradientAggregator` — host-side, pure/no networking. `submit` rejects a client's gradients for a stale `step`. `average` does real element-wise averaging, failing (not silently corrupting) on a shape/length mismatch across clients |
| `identity.rs` | `ClusterIdentity::load_or_create` persists an Ed25519 keypair to disk so `PeerId`/`ClusterId` survive restarts |
| `pairing.rs` | `PairingRegistry` issues real, wall-clock-expiring 6-digit pairing codes |
| `runtime/distributed.rs` | `DistributedBehaviour` composes four real libp2p behaviours: `mdns` (LAN discovery), `gossipsub` (job announcements + weight updates), and two `request_response::cbor` behaviours (`job_protocol` on `/brainbuilder/job/1.0.0`, `pair_protocol` on `/brainbuilder/pair/1.0.0`). `new_cluster_with_identity` builds the swarm over real TCP+noise+yamux, subscribes to the `brainbuilder/jobs/v1` gossipsub topic, and listens on an ephemeral port |

Training a job across clients reuses the [PythonBridge's](interop.md)
`compute_gradients`/`apply_averaged_gradients` split: each client computes gradients
locally, submits them as a `TensorPayload`, the host's `GradientAggregator` averages
them, and the averaged gradients are applied as one optimizer step — real distributed
data-parallel training, not a simulated progress bar.

## GUI side

The [Cluster panel](gui-panels.md#cluster-panel) polls `get_cluster_status` and lets a
user create or join a cluster, generate a pairing code, and host or join a distributed
training job. Live loss during a distributed job reuses the same `metrics-update`
event the [Training panel](gui-panels.md#training-panel) listens to — no separate UI
for distributed progress.

## Explicitly not yet built (per `CLUSTER_PLAN.md`)

- **Cross-network transport.** Only LAN mDNS discovery works today; there's no
  relay/rendezvous mechanism for devices off the same LAN.
- **Phone/tablet devices are read-only.** They connect as HTTP "observer" clients
  (`observer_server.rs`, via `axum`) with no Node/compute role — supporting an actual
  compute role on those devices would need a genuinely different runtime, not built.
- **No job-creation/job-list UI** in the Cluster Console beyond the Cluster half itself
  (the wire protocol for jobs exists; the UI for browsing/creating them doesn't).
- **No Manager-handoff UI trigger** — the wire mechanism (`ManagerHandoff`) works, no
  button calls it yet.
- **No QR-code pairing** — numeric code only.
- **No per-job data-sharding/scheduling logic** — which client trains which rows is
  currently assumed pre-assigned, not computed.
- **Gossipsub/request-response isn't yet wired into the nervous system** as a scoped
  `ComponentRuntime` for remote-peer capability enforcement — see
  [the sandbox](runtime-and-devices.md#the-nervous-system-sandbox) for what *is*
  enforced today (subprocesses, not network peers).
- **No gradient compression or async/stale-gradient training** for weak links.

See `CLUSTER_PLAN.md` at the repo root for the full phase-by-phase design log.
