// Proves the full network path for data-parallel training, not just the math
// (distributed_gradients.rs) or the discovery/join handshake
// (distributed_job_protocol.rs) in isolation: three real, independent libp2p
// swarms (one host, two clients) exchange real gradient tensors over
// request-response, the host runs them through the real `GradientAggregator`,
// and broadcasts the averaged result back over gossipsub — and both clients
// actually receive the identical averaged weights.
use brainbuilder_core::cluster::protocol::{JobRequest, JobResponse, TensorPayload, WeightUpdate};
use brainbuilder_core::cluster::GradientAggregator;
use brainbuilder_core::runtime::distributed::{new_cluster, DistributedBehaviourEvent, P2PCluster};
use libp2p::futures::StreamExt;
use libp2p::{gossipsub, mdns, request_response, swarm::SwarmEvent, PeerId};
use std::time::Duration;

#[tokio::test]
async fn two_clients_submit_gradients_and_both_receive_the_averaged_weights() {
    let mut host = new_cluster().await.expect("host swarm failed to start");
    let mut client_a = new_cluster().await.expect("client_a swarm failed to start");
    let mut client_b = new_cluster().await.expect("client_b swarm failed to start");
    let host_peer_id = host.peer_id;

    let job_id = uuid::Uuid::new_v4();
    host.subscribe_to_job_weights(job_id).unwrap();
    client_a.subscribe_to_job_weights(job_id).unwrap();
    client_b.subscribe_to_job_weights(job_id).unwrap();

    let mut aggregator = GradientAggregator::new(0, 2);
    let mut sent_a = false;
    let mut sent_b = false;
    let mut client_a_connected_to_host = false;
    let mut client_b_connected_to_host = false;
    let mut client_a_got_update = false;
    let mut client_b_got_update = false;

    let grad_a = TensorPayload { shape: vec![2], data: vec![2.0, 4.0] };
    let grad_b = TensorPayload { shape: vec![2], data: vec![4.0, 8.0] };

    let result = tokio::time::timeout(Duration::from_secs(20), async {
        let mut retry = tokio::time::interval(Duration::from_millis(300));
        loop {
            if client_a_got_update && client_b_got_update {
                return;
            }
            tokio::select! {
                _ = retry.tick() => {
                    // A request-response send needs an established
                    // connection to route over — only fire once mdns
                    // discovery has actually connected each client to the
                    // host (tracked via ConnectionEstablished below), not
                    // just on a fixed timer.
                    if !sent_a && client_a_connected_to_host {
                        let mut g = std::collections::HashMap::new();
                        g.insert("w".to_string(), grad_a.clone());
                        client_a.send_gradients(host_peer_id, JobRequest::SubmitGradients { job_id, step: 0, loss: 0.0, gradients: g });
                        sent_a = true;
                    }
                    if !sent_b && client_b_connected_to_host {
                        let mut g = std::collections::HashMap::new();
                        g.insert("w".to_string(), grad_b.clone());
                        client_b.send_gradients(host_peer_id, JobRequest::SubmitGradients { job_id, step: 0, loss: 0.0, gradients: g });
                        sent_b = true;
                    }
                }
                event = host.swarm.select_next_some() => {
                    handle_discovery(&mut host, &event);
                    if let SwarmEvent::Behaviour(DistributedBehaviourEvent::JobProtocol(
                        request_response::Event::Message { peer, message: request_response::Message::Request { request, channel, .. }, .. },
                    )) = event {
                        if let JobRequest::SubmitGradients { job_id: req_job, step, loss: _, gradients } = request {
                            assert_eq!(req_job, job_id);
                            aggregator.submit(uuid_from_peer(&peer), step, gradients);
                            host.respond_to_join(channel, JobResponse::GradientsAcked).ok();
                            if aggregator.is_ready() {
                                let averaged = aggregator.average().expect("averaging failed");
                                let update = WeightUpdate { job_id, step, weights: averaged };
                                host.publish_weight_update(&update).ok();
                            }
                        }
                    }
                }
                event = client_a.swarm.select_next_some() => {
                    handle_discovery(&mut client_a, &event);
                    if let SwarmEvent::ConnectionEstablished { peer_id, .. } = &event {
                        if *peer_id == host_peer_id {
                            client_a_connected_to_host = true;
                        }
                    }
                    if let SwarmEvent::Behaviour(DistributedBehaviourEvent::Gossipsub(gossipsub::Event::Message { message, .. })) = &event {
                        let update: WeightUpdate = serde_json::from_slice(&message.data).expect("bad weight update payload");
                        assert_eq!(update.weights["w"].data, vec![3.0, 6.0]);
                        client_a_got_update = true;
                    }
                }
                event = client_b.swarm.select_next_some() => {
                    handle_discovery(&mut client_b, &event);
                    if let SwarmEvent::ConnectionEstablished { peer_id, .. } = &event {
                        if *peer_id == host_peer_id {
                            client_b_connected_to_host = true;
                        }
                    }
                    if let SwarmEvent::Behaviour(DistributedBehaviourEvent::Gossipsub(gossipsub::Event::Message { message, .. })) = &event {
                        let update: WeightUpdate = serde_json::from_slice(&message.data).expect("bad weight update payload");
                        assert_eq!(update.weights["w"].data, vec![3.0, 6.0]);
                        client_b_got_update = true;
                    }
                }
            }
        }
    })
    .await;

    assert!(result.is_ok(), "gradient network test timed out before both clients received the averaged weights");
}

fn handle_discovery(node: &mut P2PCluster, event: &SwarmEvent<DistributedBehaviourEvent>) {
    if let SwarmEvent::Behaviour(DistributedBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) = event {
        for (peer, addr) in peers.clone() {
            node.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
            node.swarm.dial(addr).ok();
        }
    }
}

/// The aggregator keys submissions by a stable client id; a peer's own
/// PeerId (already unique per swarm) is a fine stand-in for that id in this
/// test — real code would use each client's `DeviceProfile`-associated id.
fn uuid_from_peer(peer: &PeerId) -> uuid::Uuid {
    let bytes = peer.to_bytes();
    let mut arr = [0u8; 16];
    for (i, b) in bytes.iter().take(16).enumerate() {
        arr[i] = *b;
    }
    uuid::Uuid::from_bytes(arr)
}
