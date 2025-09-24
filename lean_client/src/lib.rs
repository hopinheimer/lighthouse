mod cli;
mod environment;
pub use self::environment::{LeanEnvironment, LeanEnvironmentBuilder};
use clap::{Command, crate_version};
use clap_utils::get_color_style;
use lean_network::{LeanNetworkService, LeanNetworkBehaviourEvent};
use libp2p::swarm::SwarmEvent;
use libp2p::{identify, mdns};
use gossipsub;
use std::time::Duration;
use tracing::{Level, info, span, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct ProductionLeanClient {}

impl ProductionLeanClient {
    pub async fn new() -> Result<Self, String> {
        // Initialize tracing subscriber if not already initialized
        let _ = tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_thread_ids(false)
                    .with_level(true),
            )
            .with(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("lean_client=info".parse().unwrap()),
            )
            .try_init();

        let span = span!(Level::TRACE, "my_span");
        let _enter = span.enter();

        info!("Starting lean consensus client");

        // Initialize and start the gossipsub network service
        let mut network_service = LeanNetworkService::new().await
            .map_err(|e| format!("Failed to create network service: {}", e))?;

        // Start listening on port 9000
        network_service.start_listening(9000).await
            .map_err(|e| format!("Failed to start listening: {}", e))?;

        info!("Network service started, listening on port 9000");

        // Start the event loop
        let mut message_counter = 0u64;
        let mut message_interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                // Send a test message every 30 seconds
                _ = message_interval.tick() => {
                    message_counter += 1;
                    let test_message = format!("Test message #{} from lean client", message_counter);
                    match network_service.publish_message(test_message.as_bytes().to_vec()).await {
                        Ok(msg_id) => info!("Published message {} with ID: {:?}", message_counter, msg_id),
                        Err(e) => error!("Failed to publish message: {:?}", e),
                    }
                }

                // Handle network events
                event = network_service.next_event() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!("Listening on {}", address);
                        }
                        SwarmEvent::Behaviour(event) => {
                            match event {
                                LeanNetworkBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                                    propagation_source: peer_id,
                                    message_id: id,
                                    message,
                                }) => {
                                    info!(
                                        "Got message: '{}' with id: {:?} from peer: {:?}",
                                        String::from_utf8_lossy(&message.data),
                                        id,
                                        peer_id
                                    );
                                }
                                LeanNetworkBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed {
                                    peer_id, topic, ..
                                }) => {
                                    info!("Peer {:?} subscribed to topic {:?}", peer_id, topic);
                                }
                                LeanNetworkBehaviourEvent::Mdns(mdns::Event::Discovered(list)) => {
                                    for (peer_id, _multiaddr) in list {
                                        info!("mDNS discovered a new peer: {}", peer_id);
                                    }
                                }
                                LeanNetworkBehaviourEvent::Mdns(mdns::Event::Expired(list)) => {
                                    for (peer_id, _multiaddr) in list {
                                        info!("mDNS discover peer has expired: {}", peer_id);
                                    }
                                }
                                LeanNetworkBehaviourEvent::Identify(identify::Event::Received {
                                    peer_id,
                                    info,
                                    ..
                                }) => {
                                    info!("Received identify info from {:?}: {:?}", peer_id, info);
                                }
                                _ => {}
                            }
                        }
                        SwarmEvent::ConnectionEstablished {
                            peer_id, endpoint, ..
                        } => {
                            info!("Established connection to {:?} via {:?}", peer_id, endpoint);
                        }
                        SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                            info!("Connection to {:?} closed: {:?}", peer_id, cause);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

// TODO: change to derive
pub fn cli_app() -> Command {
    Command::new("lean_node")
        .display_order(0)
        .visible_aliases(["l", "ln", "lean"])
        .version(crate_version!())
        .author("Sigma Prime <contact@sigmaprime.io")
        .about("Lean client for the Ethereum Lean Consensus")
        .styles(get_color_style())
}
