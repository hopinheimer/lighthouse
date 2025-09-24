use futures::prelude::*;
use gossipsub::{IdentTopic as Topic, Message, MessageAuthenticity, MessageId, ValidationMode};
use libp2p::swarm::{NetworkBehaviour, Swarm, SwarmEvent};
use libp2p::{PeerId, SwarmBuilder, identify, identity::Keypair, mdns, noise, tcp, yamux};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tracing::info;

#[derive(NetworkBehaviour)]
pub struct LeanNetworkBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub identify: identify::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
}

pub struct LeanNetworkService {
    pub swarm: Swarm<LeanNetworkBehaviour>,
    pub topic: Topic,
}

impl LeanNetworkService {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Create a keypair for the node
        let local_key = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        info!("Local peer id: {:?}", local_peer_id);

        // Create a custom message ID function that uses the hash of the message content
        let message_id_fn = |message: &Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            MessageId::from(s.finish().to_string())
        };

        // Set up the gossipsub configuration
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(ValidationMode::Strict)
            .message_id_fn(message_id_fn)
            .build()
            .expect("Valid config");

        // Create a gossipsub instance with authentication disabled
        let gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )?;

        // Create identify behavior
        let identify = identify::Behaviour::new(identify::Config::new(
            "/ipfs/0.1.0".into(),
            local_key.public(),
        ));

        // Create mDNS behavior for local peer discovery
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;

        // Create the network behavior
        let behaviour = LeanNetworkBehaviour {
            gossipsub,
            identify,
            mdns,
        };

        // Create the swarm
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_quic()
            .with_behaviour(|_key| behaviour)?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        // Create a topic for lean consensus
        let topic = Topic::new("lean-consensus");

        Ok(LeanNetworkService { swarm, topic })
    }

    pub async fn start_listening(&mut self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        // Listen on all interfaces on the specified port
        self.swarm
            .listen_on(format!("/ip4/0.0.0.0/tcp/{}", port).parse()?)?;

        // Subscribe to the topic
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.topic)?;
        info!("Subscribed to topic: {}", self.topic);

        Ok(())
    }

    pub async fn publish_message(
        &mut self,
        message: Vec<u8>,
    ) -> Result<MessageId, gossipsub::PublishError> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.topic.clone(), message)
    }

    pub async fn next_event(&mut self) -> SwarmEvent<LeanNetworkBehaviourEvent> {
        self.swarm.select_next_some().await
    }
}
