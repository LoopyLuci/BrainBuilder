// Phase 4 (pairing) + Phase 3b (data-parallel training) Cluster Console
// backend. A single background task owns the real `P2PCluster` swarm
// (mirrors the pattern already proven in core/tests/cluster_pairing.rs —
// event-driven, not polled), so Tauri commands never touch libp2p directly;
// they just send a `ClusterCommand` over a channel and await the reply.
// Distributed training reuses the exact gradient-exchange pipeline already
// proven end to end in core/tests/distributed_training_live.rs (real
// PyTorch gradients, real network transport, real GradientAggregator) —
// this file's job is wiring that into the app's own graph/registry/data
// pipeline (`scheduler::compile`, `data::source::load_dataset`,
// `PythonBridge`) instead of a synthetic test op.
//
// Blocking `PythonBridge` calls (real subprocess round trips) run inline in
// this actor's async task rather than via `spawn_blocking`, matching this
// codebase's existing precedent (`StandardTrainer::fit` does the same for
// local training) — a training step briefly delays this device's *own*
// swarm event processing (incoming pairing/job messages queue and are
// handled right after), which is an acceptable, honestly-documented
// tradeoff for a personal cluster, not a distributed-systems SLA product.
use brainbuilder_core::bbir::BBIRGraph;
use brainbuilder_core::cluster::protocol::{
    ClusterRequest, ClusterResponse, JobAnnouncement, JobRequest, JobResponse, ManagerHandoff, TensorPayload,
    WeightUpdate,
};
use brainbuilder_core::cluster::{
    ClusterId, ClusterIdentity, ClusterMembership, DeviceProfile, GradientAggregator, PairingRegistry,
};
use brainbuilder_core::component::validation::validate_graph;
use brainbuilder_core::data::metrics::{publish_metric, MetricPoint};
use brainbuilder_core::data::source::{load_dataset, DataIterator};
use brainbuilder_core::interop::dlpack_support::{tensor_from_vec_f32, tensor_to_vec_f32};
use brainbuilder_core::interop::python::PythonBridge;
use brainbuilder_core::runtime::distributed::{new_cluster_with_identity, DistributedBehaviourEvent, P2PCluster};
use brainbuilder_core::runtime::scheduler::{self, ExecutionPlan};
use brainbuilder_core::{AppContext, Tensor};
use libp2p::futures::StreamExt;
use libp2p::{gossipsub, mdns, request_response, swarm::SwarmEvent, Multiaddr, PeerId};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct NodeInfo {
    pub peer_id: String,
    pub display_name: String,
    pub os: String,
    pub cpu_cores: usize,
    pub ram_total_bytes: Option<u64>,
    pub is_self: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterStatus {
    pub self_peer_id: String,
    pub has_cluster: bool,
    pub cluster_id: Option<String>,
    pub is_manager: bool,
    pub nodes: Vec<NodeInfo>,
}

/// A job this device has heard gossiped, joinable via `join_training`.
#[derive(Debug, Clone, Serialize)]
pub struct JobInfo {
    pub job_id: String,
    pub job_name: String,
    pub host_display_name: String,
}

/// Live progress of this device's own hosted or joined training job, for the
/// Cluster panel to show without needing a separate polling protocol beyond
/// what `GetTrainingStatus` already provides.
#[derive(Debug, Clone, Serialize)]
pub struct TrainingStatus {
    pub job_id: String,
    pub role: String, // "host" | "client"
    pub step: u64,
    pub total_steps: u64,
}

enum ClusterCommand {
    GetStatus { reply: oneshot::Sender<ClusterStatus> },
    CreateCluster { display_name: String, reply: oneshot::Sender<ClusterStatus> },
    GeneratePairingCode { reply: oneshot::Sender<Result<String, String>> },
    JoinWithCode { code: String, display_name: String, reply: oneshot::Sender<Result<ClusterStatus, String>> },
    HostTraining { graph_json: String, expected_clients: usize, reply: oneshot::Sender<Result<String, String>> },
    ListJobs { reply: oneshot::Sender<Vec<JobInfo>> },
    JoinTraining { job_id: String, reply: oneshot::Sender<Result<(), String>> },
    GetTrainingStatus { reply: oneshot::Sender<Option<TrainingStatus>> },
}

#[derive(Clone)]
pub struct ClusterHandle {
    tx: mpsc::Sender<ClusterCommand>,
}

impl ClusterHandle {
    pub async fn status(&self) -> ClusterStatus {
        let (reply, rx) = oneshot::channel();
        self.tx.send(ClusterCommand::GetStatus { reply }).await.ok();
        rx.await.expect("cluster actor dropped")
    }

    pub async fn create_cluster(&self, display_name: String) -> ClusterStatus {
        let (reply, rx) = oneshot::channel();
        self.tx.send(ClusterCommand::CreateCluster { display_name, reply }).await.ok();
        rx.await.expect("cluster actor dropped")
    }

    pub async fn generate_pairing_code(&self) -> Result<String, String> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(ClusterCommand::GeneratePairingCode { reply }).await.ok();
        rx.await.map_err(|_| "cluster actor dropped".to_string())?
    }

    pub async fn join_with_code(&self, code: String, display_name: String) -> Result<ClusterStatus, String> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(ClusterCommand::JoinWithCode { code, display_name, reply }).await.ok();
        rx.await.map_err(|_| "cluster actor dropped".to_string())?
    }

    /// Compiles `graph_json` and starts hosting it as a data-parallel
    /// training job: `expected_clients` other devices must join before the
    /// first gradient-averaging round runs. Returns the new job's id.
    pub async fn host_training(&self, graph_json: String, expected_clients: usize) -> Result<String, String> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(ClusterCommand::HostTraining { graph_json, expected_clients, reply }).await.ok();
        rx.await.map_err(|_| "cluster actor dropped".to_string())?
    }

    pub async fn list_jobs(&self) -> Vec<JobInfo> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(ClusterCommand::ListJobs { reply }).await.ok();
        rx.await.unwrap_or_default()
    }

    pub async fn join_training(&self, job_id: String) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(ClusterCommand::JoinTraining { job_id, reply }).await.ok();
        rx.await.map_err(|_| "cluster actor dropped".to_string())?
    }

    pub async fn training_status(&self) -> Option<TrainingStatus> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(ClusterCommand::GetTrainingStatus { reply }).await.ok();
        rx.await.unwrap_or(None)
    }
}

/// State for a job this device is hosting: coordinates gradient collection
/// and applies the averaged update, but doesn't contribute its own
/// gradients — keeping the host's single actor task from also having to
/// interleave its own `compute_gradients` calls with coordinating everyone
/// else's.
struct HostSession {
    job_id: Uuid,
    graph: BBIRGraph,
    plan: ExecutionPlan,
    weights: HashMap<String, Tensor>,
    optimizer_name: String,
    lr: f64,
    weight_decay: f64,
    grad_clip: f64,
    aggregator: GradientAggregator,
    losses_this_step: Vec<f32>,
    step: u64,
    total_steps: u64,
    expected_clients: usize,
    next_client_index: usize,
}

/// State for a job this device has joined as a compute contributor.
struct ClientSession {
    job_id: Uuid,
    host_peer: PeerId,
    plan: ExecutionPlan,
    weights: HashMap<String, Tensor>,
    loss_name: String,
    data: Box<dyn DataIterator>,
    next_expected_step: u64,
    total_steps: u64,
}

struct ActorState {
    swarm: P2PCluster,
    self_peer_id: PeerId,
    context: Arc<AppContext>,
    python: Arc<PythonBridge>,
    membership: Option<ClusterMembership>,
    registry: Option<PairingRegistry>,
    discovered: HashMap<PeerId, Multiaddr>,
    listen_addr: Option<Multiaddr>,
    pending_join: Option<(String, DeviceProfile, oneshot::Sender<Result<ClusterStatus, String>>)>,
    /// Jobs gossiped by any peer (including this device's own announcement),
    /// keyed by job id, alongside the announcing peer to join against.
    known_jobs: HashMap<Uuid, (JobAnnouncement, PeerId)>,
    host_session: Option<HostSession>,
    client_session: Option<ClientSession>,
    pending_job_join: Option<(Uuid, oneshot::Sender<Result<(), String>>)>,
}

impl ActorState {
    fn status(&self) -> ClusterStatus {
        match &self.membership {
            Some(m) => ClusterStatus {
                self_peer_id: self.self_peer_id.to_string(),
                has_cluster: true,
                cluster_id: Some(m.cluster_id.to_string()),
                is_manager: m.manager == self.self_peer_id,
                nodes: m
                    .nodes()
                    .map(|(peer, profile, _status)| NodeInfo {
                        peer_id: peer.to_string(),
                        display_name: profile.display_name.clone(),
                        os: profile.os.clone(),
                        cpu_cores: profile.cpu_cores,
                        ram_total_bytes: profile.ram_total_bytes,
                        is_self: *peer == self.self_peer_id,
                    })
                    .collect(),
            },
            None => ClusterStatus {
                self_peer_id: self.self_peer_id.to_string(),
                has_cluster: false,
                cluster_id: None,
                is_manager: false,
                nodes: vec![],
            },
        }
    }

    fn handle_discovery(&mut self, peers: &[(PeerId, Multiaddr)]) {
        for (peer, addr) in peers {
            self.discovered.insert(*peer, addr.clone());
            self.swarm.swarm.behaviour_mut().gossipsub.add_explicit_peer(peer);
            self.swarm.swarm.dial(addr.clone()).ok();
        }
    }
}

fn tensors_to_payload(weights: &HashMap<String, Tensor>) -> HashMap<String, TensorPayload> {
    weights
        .iter()
        .filter_map(|(name, t)| tensor_to_vec_f32(t).ok().map(|(shape, data)| (name.clone(), TensorPayload { shape, data })))
        .collect()
}

fn payload_to_tensors(context: &AppContext, payload: &HashMap<String, TensorPayload>) -> HashMap<String, Tensor> {
    payload
        .iter()
        .map(|(name, p)| (name.clone(), tensor_from_vec_f32(&context.arena, &p.shape, &p.data)))
        .collect()
}

/// Real identity path: `<app_data_dir>/cluster_identity.key`. Loaded (or
/// generated once) at startup so this device's `PeerId` — and, if it becomes
/// a Manager, its `ClusterId` — survives app restarts, matching Phase 3's
/// design (`CLUSTER_PLAN.md`). `context` is the same `AppContext` (registry,
/// tensor arena) the app's own Orchestrator uses, so a distributed job
/// compiles against the identical component registry; `python` is a
/// dedicated worker for cluster-training calls, separate from the
/// Orchestrator's own, so a distributed job never contends with local
/// training for the same subprocess.
pub async fn spawn(identity_path: PathBuf, context: Arc<AppContext>) -> ClusterHandle {
    let identity = ClusterIdentity::load_or_create(&identity_path).expect("failed to load/create cluster identity");
    let self_peer_id = identity.peer_id();
    let swarm = new_cluster_with_identity(identity.keypair).await.expect("failed to start cluster swarm");
    let python = Arc::new(
        PythonBridge::new(context.arena.clone()).expect("failed to start cluster training's python worker"),
    );

    let (tx, mut rx) = mpsc::channel::<ClusterCommand>(32);
    let mut state = ActorState {
        swarm,
        self_peer_id,
        context,
        python,
        membership: None,
        registry: None,
        discovered: HashMap::new(),
        listen_addr: None,
        pending_join: None,
        known_jobs: HashMap::new(),
        host_session: None,
        client_session: None,
        pending_job_join: None,
    };

    tauri::async_runtime::spawn(async move {
        let mut announce_retry = tokio::time::interval(Duration::from_secs(2));
        loop {
            tokio::select! {
                cmd = rx.recv() => {
                    let Some(cmd) = cmd else { break };
                    handle_command(&mut state, cmd).await;
                }
                event = state.swarm.swarm.select_next_some() => {
                    handle_swarm_event(&mut state, event).await;
                }
                _ = announce_retry.tick() => {
                    if let Some(session) = &state.host_session {
                        let announcement = JobAnnouncement {
                            job_id: session.job_id,
                            job_name: session.graph.name.clone(),
                            host_display_name: "this device".to_string(),
                            min_requirements: Default::default(),
                        };
                        state.swarm.announce_job(&announcement).ok();
                    }
                }
            }
        }
    });

    ClusterHandle { tx }
}

async fn handle_command(state: &mut ActorState, cmd: ClusterCommand) {
    match cmd {
        ClusterCommand::GetStatus { reply } => {
            reply.send(state.status()).ok();
        }
        ClusterCommand::CreateCluster { display_name, reply } => {
            if state.membership.is_none() {
                let profile = DeviceProfile::capture(display_name);
                state.membership = Some(ClusterMembership::new(state.self_peer_id, profile));
                state.registry = Some(PairingRegistry::new(Duration::from_secs(300)));
                let cluster_id = ClusterId::from_manager(state.self_peer_id);
                state.swarm.subscribe_to_cluster_control(cluster_id).ok();
            }
            reply.send(state.status()).ok();
        }
        ClusterCommand::GeneratePairingCode { reply } => {
            let result = match (&mut state.registry, &state.membership) {
                (Some(registry), Some(m)) if m.manager == state.self_peer_id => {
                    let addr = state
                        .listen_addr
                        .clone()
                        .unwrap_or_else(|| "/ip4/0.0.0.0/tcp/0".parse().unwrap());
                    Ok(registry.create_code(state.self_peer_id, addr))
                }
                (_, Some(_)) => Err("only the Cluster's Manager can generate a pairing code".to_string()),
                (_, None) => Err("create a Cluster first".to_string()),
            };
            reply.send(result).ok();
        }
        ClusterCommand::JoinWithCode { code, display_name, reply } => {
            if state.discovered.is_empty() {
                reply.send(Err("no other BrainBuilder devices found on this network yet".to_string())).ok();
                return;
            }
            let profile = DeviceProfile::capture(display_name);
            let peers: Vec<PeerId> = state.discovered.keys().copied().collect();
            for peer in peers {
                state.swarm.send_pairing_request(peer, ClusterRequest::Pair { code: code.clone(), profile: profile.clone() });
            }
            state.pending_join = Some((code, profile, reply));
        }
        ClusterCommand::HostTraining { graph_json, expected_clients, reply } => {
            reply.send(start_hosting(state, graph_json, expected_clients).await).ok();
        }
        ClusterCommand::ListJobs { reply } => {
            let jobs = state
                .known_jobs
                .values()
                .map(|(a, _)| JobInfo {
                    job_id: a.job_id.to_string(),
                    job_name: a.job_name.clone(),
                    host_display_name: a.host_display_name.clone(),
                })
                .collect();
            reply.send(jobs).ok();
        }
        ClusterCommand::JoinTraining { job_id, reply } => {
            let result = start_joining(state, job_id, reply).await;
            if let Err((reply, msg)) = result {
                reply.send(Err(msg)).ok();
            }
        }
        ClusterCommand::GetTrainingStatus { reply } => {
            let status = state
                .host_session
                .as_ref()
                .map(|s| TrainingStatus {
                    job_id: s.job_id.to_string(),
                    role: "host".to_string(),
                    step: s.step,
                    total_steps: s.total_steps,
                })
                .or_else(|| {
                    state.client_session.as_ref().map(|s| TrainingStatus {
                        job_id: s.job_id.to_string(),
                        role: "client".to_string(),
                        step: s.next_expected_step,
                        total_steps: s.total_steps,
                    })
                });
            reply.send(status).ok();
        }
    }
}

/// Compiles `graph_json`, lazily initializes its weights once (a throwaway
/// local `compute_gradients_step`, discarding the gradient — the only thing
/// wanted is the real `torch.randn`-initialized weight values every joining
/// client will start from identically), and starts announcing the job.
async fn start_hosting(state: &mut ActorState, graph_json: String, expected_clients: usize) -> Result<String, String> {
    if state.host_session.is_some() {
        return Err("already hosting a training job — wait for it to finish first".to_string());
    }
    let graph: BBIRGraph = serde_json::from_str(&graph_json).map_err(|e| e.to_string())?;
    {
        let registry = state.context.registry.read().map_err(|_| "registry lock poisoned".to_string())?;
        validate_graph(&graph, &registry).map_err(|e| e.to_string())?;
    }
    let plan = scheduler::compile(&graph, &state.context).map_err(|e| e.to_string())?;
    let training = graph.training.clone().ok_or_else(|| "graph has no training config".to_string())?;
    let lr = training.hyperparams.get("lr").and_then(|v| v.as_f64()).unwrap_or(0.01);
    let weight_decay = training.hyperparams.get("weight_decay").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let grad_clip = training.hyperparams.get("grad_clip").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let total_steps = plan.epochs.max(1) as u64;

    let mut data = load_dataset(&training).await.map_err(|e| e.to_string())?;
    let batch = data
        .next()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "host's dataset is empty".to_string())?;
    let inputs = plan.prepare_inputs(batch).map_err(|e| e.to_string())?;
    let mut weights: HashMap<String, Tensor> = HashMap::new();
    plan.compute_gradients_step(inputs, &mut weights, &training.loss, state.python.as_ref())
        .map_err(|e| e.to_string())?;

    let job_id = Uuid::new_v4();
    state.swarm.subscribe_to_job_weights(job_id).map_err(|e| e.to_string())?;
    state.host_session = Some(HostSession {
        job_id,
        graph,
        plan,
        weights,
        optimizer_name: training.optimizer.clone(),
        lr,
        weight_decay,
        grad_clip,
        aggregator: GradientAggregator::new(0, expected_clients),
        losses_this_step: Vec::new(),
        step: 0,
        total_steps,
        expected_clients,
        next_client_index: 0,
    });
    Ok(job_id.to_string())
}

async fn start_joining(
    state: &mut ActorState,
    job_id: String,
    reply: oneshot::Sender<Result<(), String>>,
) -> Result<(), (oneshot::Sender<Result<(), String>>, String)> {
    let job_uuid: Uuid = match job_id.parse() {
        Ok(u) => u,
        Err(_) => return Err((reply, "invalid job id".to_string())),
    };
    let Some(host_peer) = state.known_jobs.get(&job_uuid).map(|(_a, p)| *p) else {
        return Err((reply, "job not found — it may not have been discovered yet".to_string()));
    };
    if state.client_session.is_some() {
        return Err((reply, "already joined a training job".to_string()));
    }
    let profile = DeviceProfile::capture("this device");
    state.swarm.send_join_request(
        host_peer,
        JobRequest::Join { job_id: job_uuid, profile, offered_cpu_cores: 0, offered_ram_bytes: 0 },
    );
    state.pending_job_join = Some((job_uuid, reply));
    Ok(())
}

/// Computes and submits this device's gradients for `client_session`'s
/// current step, cycling back to the start of the local dataset if it's
/// been exhausted (the common personal-scale case: one batch covers the
/// whole shard, so every step reuses it).
fn client_compute_and_submit(state: &mut ActorState) -> Result<(), String> {
    let host_peer;
    let job_id;
    let step;
    let payload;
    let loss;
    {
        let session = state.client_session.as_mut().ok_or("no active client session")?;
        let batch = match session.data.next().map_err(|e| e.to_string())? {
            Some(b) => b,
            None => {
                session.data.reset().map_err(|e| e.to_string())?;
                session.data.next().map_err(|e| e.to_string())?.ok_or("client's dataset is empty")?
            }
        };
        let inputs = session.plan.prepare_inputs(batch).map_err(|e| e.to_string())?;
        let (computed_loss, gradients) = session
            .plan
            .compute_gradients_step(inputs, &mut session.weights, &session.loss_name, state.python.as_ref())
            .map_err(|e| e.to_string())?;
        payload = tensors_to_payload(&gradients);
        loss = computed_loss;
        host_peer = session.host_peer;
        job_id = session.job_id;
        step = session.next_expected_step;
    }
    state.swarm.send_gradients(host_peer, JobRequest::SubmitGradients { job_id, step, loss, gradients: payload });
    Ok(())
}

async fn handle_swarm_event(state: &mut ActorState, event: SwarmEvent<DistributedBehaviourEvent>) {
    match event {
        SwarmEvent::NewListenAddr { address, .. } => {
            state.listen_addr = Some(address);
        }
        SwarmEvent::Behaviour(DistributedBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
            state.handle_discovery(&peers);
        }
        SwarmEvent::Behaviour(DistributedBehaviourEvent::PairProtocol(request_response::Event::Message {
            peer,
            message: request_response::Message::Request { request, channel, .. },
            ..
        })) => {
            let ClusterRequest::Pair { code, profile } = request;
            let response = match (&mut state.registry, &mut state.membership) {
                (Some(registry), Some(membership)) => match registry.redeem(&code) {
                    Some(_) => {
                        membership.add_node(peer, profile);
                        ClusterResponse::Paired {
                            cluster_id: membership.cluster_id,
                            manager: state.self_peer_id.to_bytes(),
                        }
                    }
                    None => ClusterResponse::Rejected { reason: "invalid or expired pairing code".to_string() },
                },
                _ => ClusterResponse::Rejected { reason: "this device is not hosting a cluster".to_string() },
            };
            state.swarm.respond_to_pairing(channel, response).ok();
        }
        SwarmEvent::Behaviour(DistributedBehaviourEvent::PairProtocol(request_response::Event::Message {
            message: request_response::Message::Response { response, .. },
            ..
        })) => {
            if let Some((_code, profile, _)) = &state.pending_join {
                if let ClusterResponse::Paired { cluster_id, manager } = &response {
                    if let Ok(manager_peer) = PeerId::from_bytes(manager) {
                        let mut membership = ClusterMembership::new(manager_peer, profile.clone());
                        membership.add_node(state.self_peer_id, DeviceProfile::capture("this device"));
                        state.swarm.subscribe_to_cluster_control(*cluster_id).ok();
                        state.membership = Some(membership);
                        if let Some((_, _, reply)) = state.pending_join.take() {
                            reply.send(Ok(state.status())).ok();
                        }
                    }
                }
            }
        }
        SwarmEvent::Behaviour(DistributedBehaviourEvent::JobProtocol(request_response::Event::Message {
            peer,
            message: request_response::Message::Request { request, channel, .. },
            ..
        })) => {
            match request {
                JobRequest::Join { job_id, .. } => {
                    let response = match &mut state.host_session {
                        Some(session) if session.job_id == job_id && session.next_client_index < session.expected_clients => {
                            let client_index = session.next_client_index;
                            session.next_client_index += 1;
                            JobResponse::Accepted {
                                graph: session.graph.clone(),
                                initial_weights: tensors_to_payload(&session.weights),
                                client_index,
                                total_clients: session.expected_clients,
                                total_steps: session.total_steps,
                            }
                        }
                        Some(session) if session.job_id == job_id => {
                            JobResponse::Rejected { reason: "job already has enough clients".to_string() }
                        }
                        _ => JobResponse::Rejected { reason: "unknown job".to_string() },
                    };
                    state.swarm.respond_to_join(channel, response).ok();
                }
                JobRequest::SubmitGradients { job_id, step, loss, gradients } => {
                    state.swarm.respond_to_join(channel, JobResponse::GradientsAcked).ok();
                    let _ = peer;
                    handle_submitted_gradients(state, job_id, step, loss, gradients);
                }
            }
        }
        SwarmEvent::Behaviour(DistributedBehaviourEvent::JobProtocol(request_response::Event::Message {
            message: request_response::Message::Response { response, .. },
            ..
        })) => {
            handle_job_response(state, response).await;
        }
        SwarmEvent::Behaviour(DistributedBehaviourEvent::Gossipsub(gossipsub::Event::Message { message, .. })) => {
            if let Ok(handoff) = serde_json::from_slice::<ManagerHandoff>(&message.data) {
                if let (Some(membership), Ok(new_manager)) =
                    (&mut state.membership, PeerId::from_bytes(&handoff.new_manager))
                {
                    if membership.cluster_id == handoff.cluster_id {
                        membership.transfer_manager(new_manager).ok();
                    }
                }
            } else if let Ok(announcement) = serde_json::from_slice::<JobAnnouncement>(&message.data) {
                if let Some(source) = message.source {
                    state.known_jobs.insert(announcement.job_id, (announcement, source));
                }
            } else if let Ok(update) = serde_json::from_slice::<WeightUpdate>(&message.data) {
                handle_weight_update(state, update);
            }
        }
        _ => {}
    }
}

fn handle_submitted_gradients(
    state: &mut ActorState,
    job_id: Uuid,
    step: u64,
    loss: f32,
    gradients: HashMap<String, TensorPayload>,
) {
    let Some(session) = &mut state.host_session else { return };
    if session.job_id != job_id {
        return;
    }
    if !session.aggregator.submit(Uuid::new_v4(), step, gradients) {
        return; // stale step — dropped, matches GradientAggregator's own guard
    }
    session.losses_this_step.push(loss);
    if !session.aggregator.is_ready() {
        return;
    }

    let averaged = match session.aggregator.average() {
        Ok(a) => a,
        Err(_) => return, // shape mismatch across clients — nothing sound to apply
    };
    let avg_grad_tensors = payload_to_tensors(&state.context, &averaged);
    let updated = session.plan.apply_averaged_gradients_step(
        &session.weights,
        &avg_grad_tensors,
        &session.optimizer_name,
        session.lr,
        session.weight_decay,
        session.grad_clip,
        state.python.as_ref(),
    );
    let Ok(updated) = updated else { return };
    session.weights.extend(updated.clone());

    let avg_loss = if session.losses_this_step.is_empty() {
        0.0
    } else {
        session.losses_this_step.iter().sum::<f32>() / session.losses_this_step.len() as f32
    };
    publish_metric(MetricPoint {
        epoch: session.step as usize,
        step: session.step as usize,
        loss: avg_loss,
        // Early stopping (standard_trainer.rs) isn't wired into distributed
        // training's gradient-averaging loop — every round always runs.
        stopped_early: false,
        // lr_decay_epochs (standard_trainer.rs) isn't wired into distributed
        // training either — every round uses the same configured lr.
        current_lr: session.lr as f32,
    });

    let update = WeightUpdate { job_id, step: session.step, weights: tensors_to_payload(&updated) };
    state.swarm.publish_weight_update(&update).ok();

    session.step += 1;
    session.losses_this_step.clear();
    if session.step >= session.total_steps {
        state.host_session = None;
    } else {
        session.aggregator = GradientAggregator::new(session.step, session.expected_clients);
    }
}

/// Handles a `JobResponse`. `Accepted` needs to load this device's local
/// dataset (async — DataFusion I/O), so this whole function is `async`;
/// `handle_swarm_event` awaits it like everything else in the actor loop.
async fn handle_job_response(state: &mut ActorState, response: JobResponse) {
    match response {
        JobResponse::Accepted { graph, initial_weights, client_index: _, total_clients: _, total_steps } => {
            let Some((job_id, reply)) = state.pending_job_join.take() else { return };
            let training = match graph.training.clone() {
                Some(t) => t,
                None => {
                    reply.send(Err("host's graph has no training config".to_string())).ok();
                    return;
                }
            };
            let plan = match scheduler::compile(&graph, &state.context) {
                Ok(p) => p,
                Err(e) => {
                    reply.send(Err(format!("failed to compile the host's graph locally: {e}"))).ok();
                    return;
                }
            };
            // Real limitation, not a silent gap: a client trains on its OWN
            // locally-configured dataset (same `data_source` shape/columns
            // the host used, but a different file/rows — that's the whole
            // point of data parallelism). It's the user's responsibility to
            // point each device at its own real data shard; there's no
            // automatic single-file network resharding.
            let data = match load_dataset(&training).await {
                Ok(d) => d,
                Err(e) => {
                    reply.send(Err(format!("failed to load this device's local dataset: {e}"))).ok();
                    return;
                }
            };
            // `known_jobs` only grows (gossiped announcements are never
            // evicted) and `start_joining` only sets `pending_job_join`
            // after finding this exact `job_id` here, so this should always
            // hit — but a future eviction policy shouldn't turn a stale
            // entry into a panic, so this fails the join cleanly instead.
            let Some(host_peer) = state.known_jobs.get(&job_id).map(|(_, peer)| *peer) else {
                reply.send(Err("lost track of this job's host — try joining again".to_string())).ok();
                return;
            };
            if let Err(e) = state.swarm.subscribe_to_job_weights(job_id) {
                reply.send(Err(e.to_string())).ok();
                return;
            }
            let weights = payload_to_tensors(&state.context, &initial_weights);
            state.client_session = Some(ClientSession {
                job_id,
                host_peer,
                plan,
                weights,
                loss_name: training.loss.clone(),
                data,
                next_expected_step: 0,
                total_steps,
            });
            reply.send(Ok(())).ok();
            // `client_compute_and_submit` calls `.next()` itself and reports
            // a clean "dataset is empty" error there if this device's local
            // shard has nothing in it — no separate empty-check needed here.
            if let Err(e) = client_compute_and_submit(state) {
                log::error!("distributed training: failed to compute/submit step 0's gradients: {e}");
                state.client_session = None;
            }
        }
        JobResponse::Rejected { reason } => {
            if let Some((_, reply)) = state.pending_job_join.take() {
                reply.send(Err(reason)).ok();
            }
        }
        JobResponse::GradientsAcked => {}
    }
}

fn handle_weight_update(state: &mut ActorState, update: WeightUpdate) {
    let Some(session) = &mut state.client_session else { return };
    if session.job_id != update.job_id || update.step != session.next_expected_step {
        return;
    }
    session.weights.extend(payload_to_tensors(&state.context, &update.weights));
    session.next_expected_step += 1;
    if session.next_expected_step >= session.total_steps {
        state.client_session = None;
        return;
    }
    if let Err(e) = client_compute_and_submit(state) {
        log::error!("distributed training: failed to compute/submit next step's gradients: {e}");
        state.client_session = None;
    }
}
