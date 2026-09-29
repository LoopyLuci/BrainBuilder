//! Membership data model (Phase 1 — see `CLUSTER_PLAN.md`). Phase 2 (libp2p
//! gossipsub/request-response) wired the network transport; Phase 3
//! (`identity.rs`, `pairing.rs`) makes `ClusterId` real: it's the Manager's
//! own `PeerId`, derived from a keypair persisted to disk
//! (`ClusterIdentity::load_or_create`), so "my Cluster" has a stable identity
//! across restarts instead of a fresh random id every launch.
use super::device_profile::DeviceProfile;
use libp2p::PeerId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClusterId(pub PeerId);

impl ClusterId {
    /// A Cluster's identity *is* its Manager's real, keypair-derived
    /// `PeerId` — not a separately-generated random id — so redeeming a
    /// pairing code or verifying "is this really my cluster" reduces to
    /// ordinary peer authentication libp2p already does at the transport
    /// layer (noise handshake), rather than a second identity system.
    pub fn from_manager(manager: PeerId) -> Self {
        Self(manager)
    }
}

impl std::fmt::Display for ClusterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// `PeerId`'s own (de)serialize support depends on cargo features we don't
// otherwise need, so this serializes as the same raw bytes
// `PeerId::to_bytes`/`from_bytes` already use for multihash encoding. Goes
// through `Vec<u8>`'s own (sequence-based) Serialize/Deserialize rather than
// `serialize_bytes`/`deserialize_bytes` — the latter's "borrowed byte string"
// hint isn't handled consistently by every format (observed a real decode
// mismatch under the CBOR codec `request_response` uses over the wire).
impl Serialize for ClusterId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.to_bytes().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ClusterId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        PeerId::from_bytes(&bytes).map(ClusterId).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    Online,
    Busy,
    Offline,
}

#[derive(Debug, Clone)]
pub struct ClusterMembership {
    pub cluster_id: ClusterId,
    pub manager: PeerId,
    nodes: HashMap<PeerId, (DeviceProfile, NodeStatus)>,
}

impl ClusterMembership {
    /// A brand-new Cluster with just its Manager as the sole member.
    pub fn new(manager: PeerId, manager_profile: DeviceProfile) -> Self {
        let mut nodes = HashMap::new();
        nodes.insert(manager, (manager_profile, NodeStatus::Online));
        Self { cluster_id: ClusterId::from_manager(manager), manager, nodes }
    }

    pub fn add_node(&mut self, peer: PeerId, profile: DeviceProfile) {
        self.nodes.insert(peer, (profile, NodeStatus::Online));
    }

    pub fn remove_node(&mut self, peer: &PeerId) {
        self.nodes.remove(peer);
    }

    pub fn set_status(&mut self, peer: &PeerId, status: NodeStatus) {
        if let Some(entry) = self.nodes.get_mut(peer) {
            entry.1 = status;
        }
    }

    pub fn node(&self, peer: &PeerId) -> Option<&(DeviceProfile, NodeStatus)> {
        self.nodes.get(peer)
    }

    pub fn nodes(&self) -> impl Iterator<Item = (&PeerId, &DeviceProfile, NodeStatus)> {
        self.nodes.iter().map(|(peer, (profile, status))| (peer, profile, *status))
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Transfers Manager status to `new_manager`, which must already be a
    /// member. Real Manager-handoff wiring (broadcasting this to every Node
    /// so they re-point at the new Manager) is Phase 3 — this is the
    /// state-model half of it.
    pub fn transfer_manager(&mut self, new_manager: PeerId) -> Result<(), String> {
        if !self.nodes.contains_key(&new_manager) {
            return Err(format!("{new_manager} is not a member of this cluster"));
        }
        self.manager = new_manager;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::DeviceProfile;

    fn fake_peer() -> PeerId {
        PeerId::random()
    }

    #[test]
    fn new_cluster_has_exactly_its_manager() {
        let manager = fake_peer();
        let membership = ClusterMembership::new(manager, DeviceProfile::capture("desktop"));
        assert_eq!(membership.node_count(), 1);
        assert_eq!(membership.manager, manager);
        assert_eq!(membership.cluster_id, ClusterId::from_manager(manager));
    }

    #[test]
    fn cluster_id_round_trips_through_json() {
        let id = ClusterId::from_manager(fake_peer());
        let json = serde_json::to_string(&id).unwrap();
        let back: ClusterId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn add_and_remove_node_round_trips() {
        let manager = fake_peer();
        let mut membership = ClusterMembership::new(manager, DeviceProfile::capture("desktop"));
        let laptop = fake_peer();
        membership.add_node(laptop, DeviceProfile::capture("laptop"));
        assert_eq!(membership.node_count(), 2);
        membership.remove_node(&laptop);
        assert_eq!(membership.node_count(), 1);
    }

    #[test]
    fn transfer_manager_rejects_a_non_member() {
        let manager = fake_peer();
        let mut membership = ClusterMembership::new(manager, DeviceProfile::capture("desktop"));
        let stranger = fake_peer();
        assert!(membership.transfer_manager(stranger).is_err());
        assert_eq!(membership.manager, manager, "manager must not change on a rejected transfer");
    }

    #[test]
    fn transfer_manager_succeeds_for_an_existing_node() {
        let manager = fake_peer();
        let mut membership = ClusterMembership::new(manager, DeviceProfile::capture("desktop"));
        let laptop = fake_peer();
        membership.add_node(laptop, DeviceProfile::capture("laptop"));
        assert!(membership.transfer_manager(laptop).is_ok());
        assert_eq!(membership.manager, laptop);
    }
}
