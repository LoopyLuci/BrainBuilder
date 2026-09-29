// Proves Phase 2's networking is real: two actual libp2p swarms (host +
// client), each with its own TCP listener, discover each other via mDNS,
// the host gossips a real JobAnnouncement, and the client sends a real
// request-response Join request that the host accepts. Nothing here is
// mocked — both swarms run real noise-encrypted, yamux-multiplexed TCP
// connections on loopback.
use brainbuilder_core::cluster::device_profile::DeviceProfile;
use brainbuilder_core::cluster::protocol::{JobAnnouncement, JobRequest, JobResponse};
use brainbuilder_core::runtime::distributed::{new_cluster, DistributedBehaviourEvent};
use libp2p::futures::StreamExt;
use libp2p::{gossipsub, mdns, request_response, swarm::SwarmEvent};
use std::time::Duration;

#[tokio::test]
async fn two_real_swarms_discover_announce_and_join() {
    let mut host = new_cluster().await.expect("host swarm failed to start");
    let mut client = new_cluster().await.expect("client swarm failed to start");
    let host_peer_id = host.peer_id;

    let job_id = uuid::Uuid::new_v4();
    let announcement = JobAnnouncement {
        job_id,
        job_name: "integration-test-job".to_string(),
        host_display_name: "test-host".to_string(),
        min_requirements: Default::default(),
    };

    // Wait for real mDNS discovery to connect the two swarms, then have the
    // host announce the job and the client join it — driven by polling both
    // swarms' real event loops concurrently, with a hard timeout so a
    // protocol regression fails the test instead of hanging CI forever.
    let result = tokio::time::timeout(Duration::from_secs(20), async {
        let mut announced = false;
        let mut _client_saw_announcement = false;
        let mut client_joined = false;
        let mut host_accepted = false;

        // gossipsub needs at least one heartbeat cycle after the connection
        // to finish its mesh GRAFT handshake before a publish will find any
        // peers. Retrying via a *separate, concurrently-polled* timer branch
        // (rather than blocking inside an event handler with `sleep().await`)
        // matters: blocking there would stop the swarm's own event loop from
        // being polled, which is what actually drives the gossipsub
        // subscription exchange in the first place — the exact bug this
        // structure fixes.
        let mut announce_retry = tokio::time::interval(Duration::from_millis(300));

        loop {
            if client_joined && host_accepted {
                return;
            }
            tokio::select! {
                _ = announce_retry.tick() => {
                    if !announced && host.announce_job(&announcement).is_ok() {
                        announced = true;
                    }
                }
                event = host.swarm.select_next_some() => {
                    match event {
                        SwarmEvent::Behaviour(DistributedBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                            for (peer, addr) in peers {
                                host.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                                host.swarm.dial(addr).ok();
                            }
                        }
                        SwarmEvent::Behaviour(DistributedBehaviourEvent::JobProtocol(
                            request_response::Event::Message { message: request_response::Message::Request { request, channel, .. }, .. },
                        )) => {
                            match request {
                                JobRequest::Join { job_id: req_job_id, .. } => {
                                    assert_eq!(req_job_id, job_id);
                                    let accepted = JobResponse::Accepted {
                                        graph: brainbuilder_core::bbir::BBIRGraph {
                                            schema_version: 1,
                                            graph_id: "g".to_string(),
                                            name: "test-graph".to_string(),
                                            nodes: vec![],
                                            edges: vec![],
                                            training: None,
                                        },
                                        initial_weights: Default::default(),
                                        client_index: 0,
                                        total_clients: 1,
                                        total_steps: 1,
                                    };
                                    host.respond_to_join(channel, accepted).ok();
                                }
                                JobRequest::SubmitGradients { .. } => unreachable!("not exercised by this test"),
                            }
                        }
                        SwarmEvent::Behaviour(DistributedBehaviourEvent::JobProtocol(
                            request_response::Event::ResponseSent { .. },
                        )) => {
                            host_accepted = true;
                        }
                        _ => {}
                    }
                }
                event = client.swarm.select_next_some() => {
                    match event {
                        SwarmEvent::Behaviour(DistributedBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                            for (peer, addr) in peers {
                                client.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                                client.swarm.dial(addr).ok();
                            }
                        }
                        SwarmEvent::Behaviour(DistributedBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                            message, ..
                        })) => {
                            let received: JobAnnouncement =
                                serde_json::from_slice(&message.data).expect("bad announcement payload");
                            assert_eq!(received.job_id, job_id);
                            _client_saw_announcement = true;

                            let profile = DeviceProfile::capture("test-client");
                            client.send_join_request(
                                host_peer_id,
                                JobRequest::Join {
                                    job_id: received.job_id,
                                    profile,
                                    offered_cpu_cores: 2,
                                    offered_ram_bytes: 1024 * 1024 * 1024,
                                },
                            );
                        }
                        SwarmEvent::Behaviour(DistributedBehaviourEvent::JobProtocol(
                            request_response::Event::Message { message: request_response::Message::Response { response, .. }, .. },
                        )) => {
                            assert!(matches!(response, JobResponse::Accepted { .. }));
                            client_joined = true;
                        }
                        _ => {}
                    }
                }
            }
        }
    })
    .await;

    assert!(result.is_ok(), "distributed job protocol test timed out — discovery/announce/join never completed");
}
