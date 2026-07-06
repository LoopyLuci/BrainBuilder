//! Phase 1 of the Cluster/distributed-training plan (see `CLUSTER_PLAN.md`
//! at the repo root): real device introspection and the membership data
//! model. Networking (Phase 2: libp2p gossipsub/request-response for job
//! announce+join, gradient exchange) and the Cluster Console GUI (Phase 4)
//! build on this but aren't implemented yet — this phase's job is to get
//! the foundation (especially GPU enumeration) right before anything is
//! built on top of it.
pub mod aggregator;
pub mod device_profile;
pub mod identity;
pub mod membership;
pub mod pairing;
pub mod protocol;

pub use aggregator::GradientAggregator;
pub use device_profile::{DeviceProfile, GpuProfile, ResourceRequirement};
pub use identity::ClusterIdentity;
pub use membership::{ClusterId, ClusterMembership, NodeStatus};
pub use pairing::PairingRegistry;
pub use protocol::{
    ClusterRequest, ClusterResponse, JobAnnouncement, JobRequest, JobResponse, ManagerHandoff, TensorPayload,
    WeightUpdate,
};
