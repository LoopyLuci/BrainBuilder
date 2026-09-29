// Phase 2 of the Cluster/distributed-training plan (`CLUSTER_PLAN.md`):
// real libp2p behaviours for job discovery (gossipsub) and join negotiation
// (request-response, CBOR-encoded). Verified against the real libp2p 0.54.1
// API via `cargo build` + `core/tests/distributed_job_protocol.rs`, which
// runs two real in-process swarms over loopback TCP — not mocked.
use crate::cluster::protocol::{
    ClusterRequest, ClusterResponse, JobAnnouncement, JobRequest, JobResponse, ManagerHandoff, WeightUpdate,
};
use crate::cluster::ClusterId;
use libp2p::{gossipsub, mdns, request_response, swarm::NetworkBehaviour, PeerId, StreamProtocol};
use std::error::Error;
use uuid::Uuid;

#[derive(NetworkBehaviour)]
pub struct DistributedBehaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub job_protocol: request_response::cbor::Behaviour<JobRequest, JobResponse>,
    pub pair_protocol: request_response::cbor::Behaviour<ClusterRequest, ClusterResponse>,
}

pub struct P2PCluster {
    pub peer_id: PeerId,
    pub swarm: libp2p::Swarm<DistributedBehaviour>,
    pub jobs_topic: gossipsub::IdentTopic,
}

const JOB_PROTOCOL: &str = "/brainbuilder/job/1.0.0";
const PAIR_PROTOCOL: &str = "/brainbuilder/pair/1.0.0";
const JOBS_TOPIC: &str = "brainbuilder/jobs/v1";

/// Ephemeral identity — real for the tests/scaffolding, but a Cluster device
/// wants `new_cluster_with_identity` (persisted keypair) so its `PeerId`
/// (and, for a Manager, its `ClusterId`) stays stable across restarts.
pub async fn new_cluster() -> Result<P2PCluster, Box<dyn Error>> {
    new_cluster_with_identity(libp2p::identity::Keypair::generate_ed25519()).await
}

pub async fn new_cluster_with_identity(keypair: libp2p::identity::Keypair) -> Result<P2PCluster, Box<dyn Error>> {
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default().nodelay(true),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_behaviour(|key| {
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .build()
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )
            .map_err(|e| std::io::Error::other(e))?;

            let job_protocol = request_response::cbor::Behaviour::new(
                [(StreamProtocol::new(JOB_PROTOCOL), request_response::ProtocolSupport::Full)],
                request_response::Config::default(),
            );
            let pair_protocol = request_response::cbor::Behaviour::new(
                [(StreamProtocol::new(PAIR_PROTOCOL), request_response::ProtocolSupport::Full)],
                request_response::Config::default(),
            );

            Ok(DistributedBehaviour {
                mdns: mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?,
                gossipsub,
                job_protocol,
                pair_protocol,
            })
        })?
        .build();

    let jobs_topic = gossipsub::IdentTopic::new(JOBS_TOPIC);
    swarm.behaviour_mut().gossipsub.subscribe(&jobs_topic)?;

    let peer_id = *swarm.local_peer_id();
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    Ok(P2PCluster { peer_id, swarm, jobs_topic })
}

impl P2PCluster {
    /// Broadcasts a job announcement to every peer subscribed to the jobs
    /// topic (real gossipsub — propagates across the mesh, not just direct
    /// peers).
    pub fn announce_job(&mut self, announcement: &JobAnnouncement) -> Result<(), Box<dyn Error>> {
        let bytes = serde_json::to_vec(announcement)?;
        self.swarm.behaviour_mut().gossipsub.publish(self.jobs_topic.clone(), bytes)?;
        Ok(())
    }

    /// Sends a direct (non-gossiped) join request to a specific host peer —
    /// this is where the host re-validates the client's resource profile,
    /// never trusting the gossip announcement or client self-report alone.
    pub fn send_join_request(&mut self, host: PeerId, request: JobRequest) -> request_response::OutboundRequestId {
        self.swarm.behaviour_mut().job_protocol.send_request(&host, request)
    }

    pub fn respond_to_join(
        &mut self,
        channel: request_response::ResponseChannel<JobResponse>,
        response: JobResponse,
    ) -> Result<(), JobResponse> {
        self.swarm.behaviour_mut().job_protocol.send_response(channel, response)
    }

    /// A client sends the gradients it computed on its shard for this step
    /// directly to the host (not gossiped — the host alone decides when
    /// enough clients have reported in via `GradientAggregator`).
    pub fn send_gradients(&mut self, host: PeerId, request: JobRequest) -> request_response::OutboundRequestId {
        self.swarm.behaviour_mut().job_protocol.send_request(&host, request)
    }

    /// Each job gets its own gossipsub topic for weight broadcasts, derived
    /// from the job's UUID — every joined client subscribes once (on join)
    /// and then just receives the host's post-average weights each step
    /// without the host needing to track per-client delivery/backpressure.
    pub fn weight_topic(job_id: Uuid) -> gossipsub::IdentTopic {
        gossipsub::IdentTopic::new(format!("brainbuilder/jobs/{job_id}/weights"))
    }

    pub fn subscribe_to_job_weights(&mut self, job_id: Uuid) -> Result<(), Box<dyn Error>> {
        self.swarm.behaviour_mut().gossipsub.subscribe(&Self::weight_topic(job_id))?;
        Ok(())
    }

    /// Host broadcasts the averaged, post-step weights to every subscribed
    /// client — the counterpart to `announce_job`'s gossip channel, but
    /// scoped to one job's topic instead of the global job directory.
    pub fn publish_weight_update(&mut self, update: &WeightUpdate) -> Result<(), Box<dyn Error>> {
        let bytes = serde_json::to_vec(update)?;
        self.swarm.behaviour_mut().gossipsub.publish(Self::weight_topic(update.job_id), bytes)?;
        Ok(())
    }

    /// A joining device sends the pairing code it was shown (out of band —
    /// read off the Manager's Console, Phase 4) directly to the Manager,
    /// over `/brainbuilder/pair/1.0.0`.
    pub fn send_pairing_request(
        &mut self,
        manager: PeerId,
        request: ClusterRequest,
    ) -> request_response::OutboundRequestId {
        self.swarm.behaviour_mut().pair_protocol.send_request(&manager, request)
    }

    pub fn respond_to_pairing(
        &mut self,
        channel: request_response::ResponseChannel<ClusterResponse>,
        response: ClusterResponse,
    ) -> Result<(), ClusterResponse> {
        self.swarm.behaviour_mut().pair_protocol.send_response(channel, response)
    }

    fn control_topic(cluster_id: ClusterId) -> gossipsub::IdentTopic {
        gossipsub::IdentTopic::new(format!("brainbuilder/cluster/{cluster_id}/control"))
    }

    pub fn subscribe_to_cluster_control(&mut self, cluster_id: ClusterId) -> Result<(), Box<dyn Error>> {
        self.swarm.behaviour_mut().gossipsub.subscribe(&Self::control_topic(cluster_id))?;
        Ok(())
    }

    /// Broadcasts a Manager handoff to every Node in the Cluster. There's no
    /// real consensus here — the outgoing (or, in the personal-recovery
    /// case, any) Manager just announces it, and every Node updates its
    /// local view on receipt. That's the right tradeoff for a
    /// single-owner personal Cluster (every device already directly trusts
    /// every other device it paired with) rather than a multi-tenant system
    /// that would need real leader election.
    pub fn publish_manager_handoff(&mut self, handoff: &ManagerHandoff) -> Result<(), Box<dyn Error>> {
        let bytes = serde_json::to_vec(handoff)?;
        self.swarm.behaviour_mut().gossipsub.publish(Self::control_topic(handoff.cluster_id), bytes)?;
        Ok(())
    }
}
