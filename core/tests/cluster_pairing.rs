// Proves Phase 3's pairing + Manager-handoff wire protocol is real, not just
// the local data model: two independent libp2p swarms — a "Manager" and a
// "joining device" — exchange a real pairing code over request-response, the
// Manager adds the joiner to its real `ClusterMembership`, hands off Manager
// status to it, and broadcasts that handoff over gossipsub, which the joiner
// (a genuinely separate process/swarm) actually receives over the network.
use brainbuilder_core::cluster::protocol::{ClusterRequest, ClusterResponse, ManagerHandoff};
use brainbuilder_core::cluster::{ClusterId, ClusterMembership, DeviceProfile, PairingRegistry};
use brainbuilder_core::runtime::distributed::{new_cluster, DistributedBehaviourEvent};
use libp2p::futures::StreamExt;
use libp2p::{gossipsub, mdns, request_response, swarm::SwarmEvent};
use std::time::Duration;

#[tokio::test]
async fn a_joining_device_pairs_with_the_manager_and_receives_a_real_handoff() {
    let mut manager = new_cluster().await.expect("manager swarm failed to start");
    let mut joiner = new_cluster().await.expect("joiner swarm failed to start");
    let manager_peer_id = manager.peer_id;
    let joiner_peer_id = joiner.peer_id;

    let cluster_id = ClusterId::from_manager(manager_peer_id);
    let mut membership = ClusterMembership::new(manager_peer_id, DeviceProfile::capture("manager-desktop"));
    let mut registry = PairingRegistry::new(Duration::from_secs(300));
    let code = registry.create_code(manager_peer_id, "/ip4/127.0.0.1/tcp/0".parse().unwrap());

    manager.subscribe_to_cluster_control(cluster_id).unwrap();
    joiner.subscribe_to_cluster_control(cluster_id).unwrap();

    let mut joiner_connected = false;
    let mut pairing_sent = false;
    let mut joiner_paired = false;
    let mut joiner_saw_handoff = false;

    let result = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if joiner_saw_handoff && joiner_paired {
                return;
            }
            tokio::select! {
                event = manager.swarm.select_next_some() => {
                    if let SwarmEvent::Behaviour(DistributedBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) = &event {
                        for (peer, addr) in peers.clone() {
                            manager.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            manager.swarm.dial(addr).ok();
                        }
                    }
                    if let SwarmEvent::Behaviour(DistributedBehaviourEvent::PairProtocol(
                        request_response::Event::Message { peer, message: request_response::Message::Request { request, channel, .. }, .. },
                    )) = event {
                        let ClusterRequest::Pair { code: presented, profile } = request;
                        match registry.redeem(&presented) {
                            Some(_) => {
                                membership.add_node(peer, profile);
                                manager.respond_to_pairing(channel, ClusterResponse::Paired {
                                    cluster_id,
                                    manager: manager_peer_id.to_bytes(),
                                }).ok();
                                // Personal-recovery scenario: the Manager
                                // immediately hands off to the newly-paired
                                // device and broadcasts it so every other
                                // Node (and the joiner itself) re-points.
                                membership.transfer_manager(peer).expect("just-added node must be a member");
                                manager.publish_manager_handoff(&ManagerHandoff {
                                    cluster_id,
                                    new_manager: peer.to_bytes(),
                                }).ok();
                            }
                            None => {
                                manager.respond_to_pairing(channel, ClusterResponse::Rejected {
                                    reason: "invalid or expired pairing code".to_string(),
                                }).ok();
                            }
                        }
                    }
                }
                event = joiner.swarm.select_next_some() => {
                    if let SwarmEvent::Behaviour(DistributedBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) = &event {
                        for (peer, addr) in peers.clone() {
                            joiner.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            joiner.swarm.dial(addr).ok();
                        }
                    }
                    if let SwarmEvent::ConnectionEstablished { peer_id, .. } = &event {
                        if *peer_id == manager_peer_id {
                            joiner_connected = true;
                        }
                    }
                    if joiner_connected && !pairing_sent {
                        joiner.send_pairing_request(manager_peer_id, ClusterRequest::Pair {
                            code: code.clone(),
                            profile: DeviceProfile::capture("joiner-laptop"),
                        });
                        pairing_sent = true;
                    }
                    if let SwarmEvent::Behaviour(DistributedBehaviourEvent::PairProtocol(
                        request_response::Event::Message { message: request_response::Message::Response { response, .. }, .. },
                    )) = &event {
                        match response {
                            ClusterResponse::Paired { cluster_id: got_id, .. } => {
                                assert_eq!(*got_id, cluster_id);
                                joiner_paired = true;
                            }
                            ClusterResponse::Rejected { reason } => panic!("pairing was rejected: {reason}"),
                        }
                    }
                    if let SwarmEvent::Behaviour(DistributedBehaviourEvent::Gossipsub(gossipsub::Event::Message { message, .. })) = &event {
                        // Response (pairing ack) and this gossip broadcast
                        // travel over independent protocols/streams, so
                        // libp2p doesn't guarantee which arrives first —
                        // only that both real messages do arrive.
                        let handoff: ManagerHandoff = serde_json::from_slice(&message.data).expect("bad handoff payload");
                        assert_eq!(handoff.cluster_id, cluster_id);
                        assert_eq!(handoff.new_manager, joiner_peer_id.to_bytes());
                        joiner_saw_handoff = true;
                    }
                }
            }
        }
    })
    .await;

    assert!(result.is_ok(), "pairing/handoff test timed out");
    assert!(joiner_paired, "joiner must have received a real Paired response");
    assert_eq!(membership.manager, joiner_peer_id, "manager's local membership must reflect the handoff");
    assert_eq!(membership.node_count(), 2, "manager must have added the joiner as a real member");
}
