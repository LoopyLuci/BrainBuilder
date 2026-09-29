// The glue test: proves the three independently-verified pieces
// (`distributed_gradients.rs`'s gradient math, `distributed_gradient_network.rs`'s
// wire transport, `distributed_job_protocol.rs`'s discovery/join) actually
// compose into one live, multi-step distributed training run. Two real
// `PythonBridge` subprocess workers ("clients") each train on a disjoint data
// shard; a third ("host") applies the averaged gradient. All three are wired
// together over real libp2p swarms, not simulated in-process math. Ignored by
// default (needs `torch` importable) — run with `cargo test -- --ignored`.
use brainbuilder_core::cluster::protocol::{JobRequest, JobResponse, TensorPayload, WeightUpdate};
use brainbuilder_core::cluster::GradientAggregator;
use brainbuilder_core::component::descriptor::DataType;
use brainbuilder_core::interop::arena::SharedArena;
use brainbuilder_core::interop::python::PythonBridge;
use brainbuilder_core::runtime::cpu_backend::CpuDevice;
use brainbuilder_core::runtime::device::Device;
use brainbuilder_core::runtime::distributed::{new_cluster, DistributedBehaviourEvent, P2PCluster};
use brainbuilder_core::runtime::scheduler::ExecutableOp;
use libp2p::futures::StreamExt;
use libp2p::{gossipsub, mdns, request_response, swarm::SwarmEvent};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

const W_LEN: usize = 2;
const MAX_STEPS: u64 = 60;
const TRUE_W: f32 = 3.0;
const LR: f64 = 0.05;

fn tensor_from(values: &[f32]) -> brainbuilder_core::Tensor {
    let device = CpuDevice;
    let t = device.alloc(&[values.len()], DataType::Float32);
    unsafe {
        let dst = (*t.0).dl_tensor.data as *mut f32;
        std::ptr::copy_nonoverlapping(values.as_ptr(), dst, values.len());
    }
    t
}

fn read(tensor: &brainbuilder_core::Tensor, len: usize) -> Vec<f32> {
    unsafe { std::slice::from_raw_parts((*tensor.0).dl_tensor.data as *const f32, len).to_vec() }
}

fn to_payload(t: &brainbuilder_core::Tensor) -> TensorPayload {
    TensorPayload { shape: vec![W_LEN as i64], data: read(t, W_LEN) }
}

fn scale_ops() -> Vec<ExecutableOp> {
    vec![ExecutableOp {
        node_id: "n1".into(),
        component: "bb_scale_component".into(),
        language: "python".into(),
        entry: "forward".into(),
        inputs: vec!["x".into(), "w".into()],
        outputs: vec!["y".into()],
        param_inputs: vec!["w".into()],
        trainable_inputs: vec!["w".into()],
        zero_init_inputs: vec![],
        hyperparams: serde_json::json!({}),
        resolved_param_shapes: Default::default(),
    }]
}

fn handle_discovery(node: &mut P2PCluster, event: &SwarmEvent<DistributedBehaviourEvent>) {
    if let SwarmEvent::Behaviour(DistributedBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) = event {
        for (peer, addr) in peers.clone() {
            node.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
            node.swarm.dial(addr).ok();
        }
    }
}

#[tokio::test]
#[ignore]
async fn two_real_workers_train_over_the_network_and_converge() {
    let fixtures_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    std::env::set_var("PYTHONPATH", &fixtures_dir);

    let mut host = new_cluster().await.expect("host swarm failed to start");
    let mut client_a = new_cluster().await.expect("client_a swarm failed to start");
    let mut client_b = new_cluster().await.expect("client_b swarm failed to start");
    let host_peer_id = host.peer_id;

    let job_id = uuid::Uuid::new_v4();
    host.subscribe_to_job_weights(job_id).unwrap();
    client_a.subscribe_to_job_weights(job_id).unwrap();
    client_b.subscribe_to_job_weights(job_id).unwrap();

    let host_bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();
    let client_a_bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();
    let client_b_bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();
    let ops = scale_ops();

    // Two disjoint shards of the same y = 3x distribution.
    let shard_a_x = [1.0f32, 2.0];
    let shard_b_x = [3.0f32, 4.0];
    let shard_a_target = tensor_from(&shard_a_x.iter().map(|x| x * TRUE_W).collect::<Vec<_>>());
    let shard_b_target = tensor_from(&shard_b_x.iter().map(|x| x * TRUE_W).collect::<Vec<_>>());

    let initial_w = vec![0.1f32; W_LEN];
    let mut host_weight = tensor_from(&initial_w);
    let mut client_a_weight = tensor_from(&initial_w);
    let mut client_b_weight = tensor_from(&initial_w);

    let mut host_aggregator = GradientAggregator::new(0, 2);
    let mut client_a_connected = false;
    let mut client_b_connected = false;
    let mut client_a_sent_step0 = false;
    let mut client_b_sent_step0 = false;
    let mut client_a_expected_step = 1u64;
    let mut client_b_expected_step = 1u64;
    let mut host_step = 0u64;
    let mut host_done = false;
    let mut losses: Vec<f32> = Vec::new();

    let compute_and_send = |bridge: &PythonBridge,
                             node: &mut P2PCluster,
                             weight: &brainbuilder_core::Tensor,
                             shard_x: brainbuilder_core::Tensor,
                             target: &brainbuilder_core::Tensor,
                             step: u64|
     -> f32 {
        let mut inputs = HashMap::new();
        inputs.insert("x".to_string(), shard_x);
        inputs.insert("w".to_string(), weight.clone());
        let (loss, grads) = bridge
            .compute_gradients(&ops, inputs, "y", target, "mse", &["w".to_string()])
            .expect("compute_gradients failed");
        let mut payload = HashMap::new();
        payload.insert("w".to_string(), to_payload(grads.get("w").unwrap()));
        node.send_gradients(host_peer_id, JobRequest::SubmitGradients { job_id, step, loss, gradients: payload });
        loss
    };

    let result = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if host_done {
                return;
            }
            tokio::select! {
                event = host.swarm.select_next_some() => {
                    handle_discovery(&mut host, &event);
                    if let SwarmEvent::Behaviour(DistributedBehaviourEvent::JobProtocol(
                        request_response::Event::Message { peer: _, message: request_response::Message::Request { request, channel, .. }, .. },
                    )) = event {
                        if let JobRequest::SubmitGradients { job_id: req_job, step, loss: _, gradients } = request {
                            assert_eq!(req_job, job_id);
                            host.respond_to_join(channel, JobResponse::GradientsAcked).ok();
                            // Keyed by a fresh id per submission (this test only
                            // cares about count, not per-client identity).
                            host_aggregator.submit(uuid::Uuid::new_v4(), step, gradients);
                            if host_aggregator.is_ready() {
                                let averaged = host_aggregator.average().expect("averaging failed");
                                let mut avg_grads = HashMap::new();
                                let payload = averaged.get("w").unwrap();
                                avg_grads.insert("w".to_string(), tensor_from(&payload.data));
                                let mut weights = HashMap::new();
                                weights.insert("w".to_string(), host_weight.clone());
                                let updated = host_bridge
                                    .apply_averaged_gradients(&weights, &avg_grads, "sgd", LR, 0.0, 0.0)
                                    .expect("apply_averaged_gradients failed");
                                host_weight = updated.get("w").unwrap().clone();
                                host_step += 1;

                                if host_step >= MAX_STEPS {
                                    host_done = true;
                                } else {
                                    host_aggregator = GradientAggregator::new(host_step, 2);
                                    let update = WeightUpdate { job_id, step: host_step, weights: {
                                        let mut m = HashMap::new();
                                        m.insert("w".to_string(), to_payload(&host_weight));
                                        m
                                    }};
                                    host.publish_weight_update(&update).ok();
                                }
                            }
                        }
                    }
                }
                event = client_a.swarm.select_next_some() => {
                    handle_discovery(&mut client_a, &event);
                    if let SwarmEvent::ConnectionEstablished { peer_id, .. } = &event {
                        if *peer_id == host_peer_id { client_a_connected = true; }
                    }
                    if client_a_connected && !client_a_sent_step0 {
                        let loss = compute_and_send(&client_a_bridge, &mut client_a, &client_a_weight, tensor_from(&shard_a_x), &shard_a_target, 0);
                        losses.push(loss);
                        client_a_sent_step0 = true;
                    }
                    if let SwarmEvent::Behaviour(DistributedBehaviourEvent::Gossipsub(gossipsub::Event::Message { message, .. })) = &event {
                        let update: WeightUpdate = serde_json::from_slice(&message.data).expect("bad weight update");
                        if update.step == client_a_expected_step {
                            client_a_weight = tensor_from(&update.weights["w"].data);
                            let loss = compute_and_send(&client_a_bridge, &mut client_a, &client_a_weight, tensor_from(&shard_a_x), &shard_a_target, update.step);
                            losses.push(loss);
                            client_a_expected_step += 1;
                        }
                    }
                }
                event = client_b.swarm.select_next_some() => {
                    handle_discovery(&mut client_b, &event);
                    if let SwarmEvent::ConnectionEstablished { peer_id, .. } = &event {
                        if *peer_id == host_peer_id { client_b_connected = true; }
                    }
                    if client_b_connected && !client_b_sent_step0 {
                        let loss = compute_and_send(&client_b_bridge, &mut client_b, &client_b_weight, tensor_from(&shard_b_x), &shard_b_target, 0);
                        losses.push(loss);
                        client_b_sent_step0 = true;
                    }
                    if let SwarmEvent::Behaviour(DistributedBehaviourEvent::Gossipsub(gossipsub::Event::Message { message, .. })) = &event {
                        let update: WeightUpdate = serde_json::from_slice(&message.data).expect("bad weight update");
                        if update.step == client_b_expected_step {
                            client_b_weight = tensor_from(&update.weights["w"].data);
                            let loss = compute_and_send(&client_b_bridge, &mut client_b, &client_b_weight, tensor_from(&shard_b_x), &shard_b_target, update.step);
                            losses.push(loss);
                            client_b_expected_step += 1;
                        }
                    }
                }
            }
        }
    })
    .await;

    assert!(result.is_ok(), "live distributed training run timed out after {MAX_STEPS} steps");

    let first_loss = losses[0];
    let last_loss = *losses.last().unwrap();
    assert!(
        last_loss < first_loss * 0.1,
        "expected loss to collapse over the live distributed run: first={first_loss}, last={last_loss}"
    );

    let learned_w = read(&host_weight, W_LEN);
    for w_i in learned_w {
        assert!(
            (w_i - TRUE_W).abs() < 0.2,
            "expected host's final averaged weight ~{TRUE_W}, got {w_i}"
        );
    }
}
