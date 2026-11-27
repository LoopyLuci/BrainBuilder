//! The Cluster/distributed-training data model (see `CLUSTER_PLAN.md` at the
//! repo root): device introspection, membership, identity and pairing, the
//! wire protocol types, gradient aggregation and per-epoch data sharding.
//! The libp2p transport lives in `runtime::distributed`; the Cluster Console
//! GUI in `gui/src-tauri/src/cluster_actor.rs`.
pub mod aggregator;
pub mod device_profile;
pub mod identity;
pub mod membership;
pub mod pairing;
pub mod protocol;
pub mod sharding;

pub use aggregator::GradientAggregator;
pub use device_profile::{DeviceProfile, GpuProfile, ResourceRequirement};
pub use identity::ClusterIdentity;
pub use membership::{ClusterId, ClusterMembership, NodeStatus};
pub use pairing::PairingRegistry;
pub use sharding::{ShardError, ShardPlan};
pub use protocol::{
    ClusterRequest, ClusterResponse, JobAnnouncement, JobRequest, JobResponse, ManagerHandoff, TensorPayload,
    WeightUpdate,
};
