//! Phase 3: pairing codes — the zero-knowledge-user-facing half of Cluster
//! identity. The Manager generates a short-lived numeric code (displayed on
//! its Cluster Console, Phase 4); a joining device presents that code over
//! the network (`ClusterRequest::Pair` in `protocol.rs`) instead of a user
//! ever typing an address or exchanging a key by hand.
use libp2p::{Multiaddr, PeerId};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A real, one-time-use 6-digit code. Backed by `uuid`'s v4 generation (OS
/// CSPRNG via the `getrandom` crate, already a transitive dependency) rather
/// than a hand-rolled RNG.
fn generate_code() -> String {
    let n = uuid::Uuid::new_v4().as_u128() % 1_000_000;
    format!("{n:06}")
}

pub struct PairingSession {
    pub manager_peer_id: PeerId,
    pub manager_addr: Multiaddr,
    expires_at: Instant,
}

/// Held by the Manager. Real expiry (checked against `Instant::now()`, not
/// simulated) and real one-time-use semantics: a code is removed from the
/// registry the moment it's redeemed, so a code seen once (e.g. by someone
/// glancing at the Console) can't be replayed later.
pub struct PairingRegistry {
    sessions: HashMap<String, PairingSession>,
    validity: Duration,
}

impl PairingRegistry {
    pub fn new(validity: Duration) -> Self {
        Self { sessions: HashMap::new(), validity }
    }

    pub fn create_code(&mut self, manager_peer_id: PeerId, manager_addr: Multiaddr) -> String {
        self.prune_expired();
        let code = generate_code();
        self.sessions.insert(
            code.clone(),
            PairingSession { manager_peer_id, manager_addr, expires_at: Instant::now() + self.validity },
        );
        code
    }

    /// Consumes and returns the session for `code` if it exists and hasn't
    /// expired; `None` for an unknown, already-redeemed, or expired code —
    /// deliberately indistinguishable to the caller (a client shouldn't be
    /// able to distinguish "wrong code" from "code expired" by response
    /// shape alone).
    pub fn redeem(&mut self, code: &str) -> Option<PairingSession> {
        self.prune_expired();
        self.sessions.remove(code)
    }

    fn prune_expired(&mut self) {
        let now = Instant::now();
        self.sessions.retain(|_, s| s.expires_at > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_addr() -> Multiaddr {
        "/ip4/127.0.0.1/tcp/0".parse().unwrap()
    }

    #[test]
    fn a_freshly_created_code_redeems_successfully() {
        let mut registry = PairingRegistry::new(Duration::from_secs(300));
        let manager = PeerId::random();
        let code = registry.create_code(manager, fake_addr());
        assert_eq!(code.len(), 6);
        let session = registry.redeem(&code).expect("fresh code must redeem");
        assert_eq!(session.manager_peer_id, manager);
    }

    #[test]
    fn a_code_cannot_be_redeemed_twice() {
        let mut registry = PairingRegistry::new(Duration::from_secs(300));
        let code = registry.create_code(PeerId::random(), fake_addr());
        assert!(registry.redeem(&code).is_some());
        assert!(registry.redeem(&code).is_none(), "replaying a redeemed code must fail");
    }

    #[test]
    fn an_expired_code_fails_to_redeem() {
        let mut registry = PairingRegistry::new(Duration::from_millis(20));
        let code = registry.create_code(PeerId::random(), fake_addr());
        std::thread::sleep(Duration::from_millis(60));
        assert!(registry.redeem(&code).is_none(), "an expired code must not redeem");
    }

    #[test]
    fn an_unknown_code_fails_to_redeem() {
        let mut registry = PairingRegistry::new(Duration::from_secs(300));
        registry.create_code(PeerId::random(), fake_addr());
        assert!(registry.redeem("000000").is_none());
    }
}
