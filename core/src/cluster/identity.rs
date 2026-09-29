//! Phase 3: a Cluster's real, persistent identity. `libp2p::identity::Keypair`
//! is already how every swarm authenticates itself (the noise handshake in
//! `runtime::distributed::new_cluster`) — this just persists that keypair to
//! disk across restarts, instead of generating a fresh one every launch, so
//! "my Cluster" and "my Manager" have a stable identity a joining device can
//! actually trust over time.
use libp2p::identity::Keypair;
use libp2p::PeerId;
use std::io;
use std::path::Path;

pub struct ClusterIdentity {
    pub keypair: Keypair,
}

impl ClusterIdentity {
    pub fn generate() -> Self {
        Self { keypair: Keypair::generate_ed25519() }
    }

    pub fn peer_id(&self) -> PeerId {
        self.keypair.public().to_peer_id()
    }

    /// Loads a previously-saved identity from `path`, or generates and saves
    /// a new one if none exists yet (or the existing file is unreadable —
    /// treated as "no identity yet" rather than a hard error, since a
    /// zero-knowledge user should never see a crash here).
    pub fn load_or_create(path: &Path) -> io::Result<Self> {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(keypair) = Keypair::from_protobuf_encoding(&bytes) {
                return Ok(Self { keypair });
            }
        }
        let identity = Self::generate();
        let bytes = identity
            .keypair
            .to_protobuf_encoding()
            .map_err(|e| io::Error::other(format!("failed to encode cluster identity: {e}")))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)?;
        Ok(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_or_create_generates_a_real_keypair_when_none_exists() {
        let dir = std::env::temp_dir().join(format!("bb_identity_test_{}", uuid::Uuid::new_v4()));
        let path = dir.join("identity.key");
        let identity = ClusterIdentity::load_or_create(&path).unwrap();
        assert!(path.exists());
        let _ = identity.peer_id();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_or_create_returns_the_same_identity_across_restarts() {
        let dir = std::env::temp_dir().join(format!("bb_identity_test_{}", uuid::Uuid::new_v4()));
        let path = dir.join("identity.key");
        let first = ClusterIdentity::load_or_create(&path).unwrap();
        let second = ClusterIdentity::load_or_create(&path).unwrap();
        assert_eq!(first.peer_id(), second.peer_id(), "reloading must yield the same real PeerId");
        std::fs::remove_dir_all(&dir).ok();
    }
}
