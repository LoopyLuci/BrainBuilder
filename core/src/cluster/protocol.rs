//! Phase 2 wire protocol (see `CLUSTER_PLAN.md`): job announcements
//! (gossipsub broadcast) and join negotiation (request-response), real
//! libp2p behaviours, tested in `core/tests/distributed_job_protocol.rs`
//! against two real in-process swarms over loopback TCP.
use crate::cluster::device_profile::{DeviceProfile, ResourceRequirement};
use crate::cluster::membership::ClusterId;
use serde::{Deserialize, Serialize};

/// Gossiped on the `brainbuilder/jobs/v1` topic so any peer on the mesh can
/// discover a joinable job without a central directory. Deliberately small
/// (graph name only, not the full `BBIRGraph`) — the full spec is fetched via
/// request-response only once a peer actually wants to consider joining.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAnnouncement {
    pub job_id: uuid::Uuid,
    pub job_name: String,
    pub host_display_name: String,
    pub min_requirements: ResourceRequirement,
}

/// Request-response protocol messages, exchanged directly between a
/// prospective client and the job's host (not gossiped — this is where
/// `min_requirements` gets *re-checked* against the client's real profile,
/// never trusting gossip/self-report for anything access-gating).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobRequest {
    /// Ask to join a job, offering a real captured profile and how much of
    /// it the client is willing to contribute (host's `JobClientPermissions`
    /// determines whether these client-chosen values are honored as-is or
    /// clamped — Phase 2 always honors them; clamping is a real Phase 3+
    /// refinement once `JobClientPermissions` exists on the wire too).
    Join { job_id: uuid::Uuid, profile: DeviceProfile, offered_cpu_cores: usize, offered_ram_bytes: u64 },
    /// A client submits the gradients it computed (via
    /// `PythonBridge::compute_gradients`) on its data shard for this step,
    /// plus its own local loss so the host can publish a real (averaged)
    /// metric point for the GUI's training dashboard — the point of
    /// data-parallel training is that no single device's loss alone
    /// describes the whole model's progress. The host accumulates these
    /// from every joined client before averaging and applying an update —
    /// see `GradientAggregator`.
    SubmitGradients {
        job_id: uuid::Uuid,
        step: u64,
        loss: f32,
        gradients: std::collections::HashMap<String, TensorPayload>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobResponse {
    /// `graph`/`initial_weights` let the client run the *exact* same
    /// computation the host will average against — the host lazily
    /// initializes each parameter port once (real `torch.randn`) before
    /// accepting anyone, so every client starts from identical weights
    /// instead of each independently rolling its own random init (which
    /// would make step 0's "average" meaningless — two different models).
    /// `client_index`/`total_clients` (0-based, fixed at accept time for
    /// Phase 2's fixed-roster model) is how a client picks its data shard
    /// (`batch_index % total_clients == client_index`) without every client
    /// needing to coordinate partitioning themselves.
    Accepted {
        graph: crate::bbir::BBIRGraph,
        initial_weights: std::collections::HashMap<String, TensorPayload>,
        client_index: usize,
        total_clients: usize,
        /// How many gradient-exchange rounds this job runs — derived by the
        /// host from its `TrainingConfig`'s `epochs` hyperparameter, so
        /// every client knows when to stop without a separate "job done"
        /// message.
        total_steps: u64,
    },
    Rejected { reason: String },
    GradientsAcked,
}

/// A tensor flattened for wire transport — real shape + real f32 data, the
/// same representation `PythonBridge` already uses for its scratch-file
/// protocol (`core/src/interop/python.rs`), just serialized instead of
/// written to disk since this crosses a network socket, not a process
/// boundary on the same machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorPayload {
    pub shape: Vec<i64>,
    pub data: Vec<f32>,
}

/// Gossiped on a per-job topic (`brainbuilder/jobs/<job_id>/weights`) so
/// every joined client picks up the host's post-average weights for the next
/// step, without the host needing to track per-client delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightUpdate {
    pub job_id: uuid::Uuid,
    pub step: u64,
    pub weights: std::collections::HashMap<String, TensorPayload>,
}

/// Phase 3 pairing: exchanged directly (request-response, not gossiped)
/// between a joining device and the Manager it's pairing with, over the
/// `/brainbuilder/pair/1.0.0` protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterRequest {
    Pair { code: String, profile: DeviceProfile },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterResponse {
    Paired { cluster_id: ClusterId, manager: libp2p_peer_id_bytes::PeerIdBytes },
    Rejected { reason: String },
}

/// Gossiped on `brainbuilder/cluster/<cluster_id>/control` so every Node
/// re-points at a new Manager the moment a handoff happens, without needing
/// a real consensus protocol — acceptable for a personal, single-owner
/// Cluster where every device already trusts every other device it paired
/// with directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerHandoff {
    pub cluster_id: ClusterId,
    pub new_manager: libp2p_peer_id_bytes::PeerIdBytes,
}

/// `PeerId` doesn't derive `Serialize`/`Deserialize` under our enabled
/// libp2p features, so wire messages carry it as its own canonical byte
/// encoding (`PeerId::to_bytes`/`from_bytes`) instead.
pub mod libp2p_peer_id_bytes {
    pub type PeerIdBytes = Vec<u8>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_announcement_round_trips_through_json() {
        let announcement = JobAnnouncement {
            job_id: uuid::Uuid::new_v4(),
            job_name: "test-job".to_string(),
            host_display_name: "desktop".to_string(),
            min_requirements: ResourceRequirement { min_cpu_cores: Some(4), ..Default::default() },
        };
        let json = serde_json::to_string(&announcement).unwrap();
        let back: JobAnnouncement = serde_json::from_str(&json).unwrap();
        assert_eq!(back.job_id, announcement.job_id);
        assert_eq!(back.min_requirements.min_cpu_cores, Some(4));
    }
}
