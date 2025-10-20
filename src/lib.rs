//! # SomaSync - Neural-Inspired Distributed Mesh Networking
//!
//! SomaSync provides neural-inspired distributed mesh networking capabilities with gossip protocols
//! for building resilient, self-organizing networks. Perfect for distributed threat intelligence,
//! real-time data sharing, and coordinated system responses.
//!
//! ## Core Features
//!
//! - **Neural-Inspired Architecture**: Nodes communicate like neurons via synaptic connections
//! - **Mesh Networking**: Self-organizing peer discovery and connection management
//! - **Gossip Protocols**: Efficient data propagation with anti-entropy guarantees
//! - **Self-Healing**: Automatic failure detection and network recovery
//! - **High Performance**: Zero-copy serialization and connection pooling
//! - **Production Ready**: Comprehensive error handling and monitoring
//!
//! ## Examples
//!
//! ### Basic Node Setup
//!
//! ```rust,no_run
//! use somasync::{SynapseNodeBuilder, MessageType};
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create and configure a new node
//!     let (mut node, _message_rx, _event_rx) = SynapseNodeBuilder::new()
//!         .with_node_id("node-1".to_string())
//!         .build()?;
//!     
//!     // Start the node
//!     node.start().await?;
//!     
//!     // Send a message to the mesh
//!     node.broadcast_message(MessageType::Data("Hello, neural mesh!".to_string())).await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Message Handling with Receivers
//!
//! ```rust,no_run
//! use somasync::{SynapseNodeBuilder, MessageType, SynapseEvent};
//! use tokio::time::{sleep, Duration};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let (mut node, mut message_rx, mut event_rx) = SynapseNodeBuilder::new()
//!         .with_node_id("receiver-node".to_string())
//!         .with_bind_address("127.0.0.1:8080".parse()?)
//!         .build()?;
//!
//!     node.start().await?;
//!
//!     // Spawn task to handle incoming messages
//!     let message_handler = tokio::spawn(async move {
//!         while let Some(message) = message_rx.recv().await {
//!             println!("Received message: {:?}", message.message_type);
//!         }
//!     });
//!
//!     // Spawn task to handle node events
//!     let event_handler = tokio::spawn(async move {
//!         while let Some(event) = event_rx.recv().await {
//!             match event {
//!                 SynapseEvent::PeerConnected { peer_id, address } => {
//!                     println!("Peer connected: {} at {}", peer_id, address);
//!                 }
//!                 SynapseEvent::MessageReceived { from, message } => {
//!                     println!("Message {} received from {}", message.id, from);
//!                 }
//!                 _ => {}
//!             }
//!         }
//!     });
//!
//!     // Keep the node running
//!     sleep(Duration::from_secs(30)).await;
//!     Ok(())
//! }
//! ```
//!
//! ### Message Signing for Security
//!
//! ```rust,no_run
//! use somasync::{SynapseNodeBuilder, MessageType, Message};
//! use ed25519_dalek::SigningKey;
//! use rand::{rngs::OsRng, RngCore};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Generate signing keys
//!     let mut csprng = OsRng;
//!     let mut secret_bytes = [0u8; 32];
//!     csprng.fill_bytes(&mut secret_bytes);
//!     let signing_key = SigningKey::from_bytes(&secret_bytes);
//!
//!     let (mut node, _message_rx, _event_rx) = SynapseNodeBuilder::new()
//!         .with_node_id("secure-node".to_string())
//!         .build()?;
//!
//!     node.start().await?;
//!
//!     // Create and sign a message with threat intel data
//!     let threat_data = serde_json::json!({
//!         "threat_type": "malware",
//!         "ioc": "192.168.1.100",
//!         "confidence": 0.95,
//!         "timestamp": "2025-10-20T12:00:00Z"
//!     });
//!
//!     let mut message = Message::new(
//!         MessageType::Structured(threat_data.as_object().unwrap().iter()
//!             .map(|(k, v)| (k.clone(), v.to_string()))
//!             .collect()),
//!         "threat-detector-1".to_string()
//!     );
//!
//!     // Sign the message for authenticity
//!     let message = message.sign_with_ed25519(&signing_key);
//!
//!     // Broadcast the signed threat intelligence
//!     node.broadcast_message(message.message_type).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Enterprise Gossip for Large Networks
//!
//! ```rust,no_run
//! use somasync::{SynapseNodeBuilder, EnterpriseGossipConfig, MessageType, Peer};
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Configure enterprise gossip for high-scale threat intel sharing
//!     let enterprise_config = EnterpriseGossipConfig::for_threat_intel();
//!
//!     let (mut node, mut message_rx, _event_rx) = SynapseNodeBuilder::new()
//!         .with_node_id("enterprise-hub".to_string())
//!         .with_bind_address("0.0.0.0:9090".parse()?)
//!         .with_gossip_config(enterprise_config.base)
//!         .build()?;
//!
//!     node.start().await?;
//!
//!     // Add bootstrap peers for enterprise network
//!     let bootstrap_peers: Vec<std::net::SocketAddr> = vec![
//!         "10.0.1.100:9090".parse()?,
//!         "10.0.2.100:9090".parse()?,
//!         "10.0.3.100:9090".parse()?,
//!     ];
//!
//!     for peer_addr in bootstrap_peers {
//!         let peer = Peer::new(peer_addr.to_string(), peer_addr);
//!         node.add_peer(peer).await?;
//!     }
//!
//!     // Broadcast high-priority threat alert
//!     let mut alert_data = HashMap::new();
//!     alert_data.insert("alert_level".to_string(), "critical".to_string());
//!     alert_data.insert("attack_vector".to_string(), "lateral_movement".to_string());
//!     alert_data.insert("affected_systems".to_string(), "database_cluster".to_string());
//!
//!     let alert_message = MessageType::Alert {
//!         level: "critical".to_string(),
//!         message: "Advanced persistent threat detected".to_string(),
//!         details: Some(alert_data),
//!     };
//!
//!     node.broadcast_message(alert_message).await?;
//!
//!     println!("Enterprise threat intelligence hub started");
//!     Ok(())
//! }
//! ```
//!
//! ### Production Logging and Monitoring
//!
//! ```rust,no_run
//! use somasync::{
//!     SynapseNodeBuilder, MessageType, CorrelationId,
//!     init_logging, production_config, SecurityEvent
//! };
//! use tracing::{info, warn};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize production logging with JSON output
//!     let log_config = production_config();
//!     init_logging(&log_config)?;
//!
//!     // Create correlation ID for this session
//!     let correlation_id = CorrelationId::new();
//!     
//!     info!(
//!         correlation_id = %correlation_id.short(),
//!         environment = "production",
//!         "Starting SomaSync threat intelligence node"
//!     );
//!
//!     let (mut node, mut message_rx, mut event_rx) = SynapseNodeBuilder::new()
//!         .with_node_id("prod-intel-node".to_string())
//!         .with_bind_address("0.0.0.0:8080".parse()?)
//!         .build()?;
//!
//!     // Start node (automatically logs with correlation ID and performance metrics)
//!     node.start().await?;
//!
//!     info!(
//!         correlation_id = %correlation_id.short(),
//!         node_id = "prod-intel-node",
//!         "Node started successfully with structured logging"
//!     );
//!
//!     // Handle incoming messages with structured logging
//!     let correlation_id_clone = correlation_id.clone();
//!     tokio::spawn(async move {
//!         while let Some(message) = message_rx.recv().await {
//!             info!(
//!                 correlation_id = %correlation_id_clone.short(),
//!                 message_id = %message.id,
//!                 message_type = ?message.message_type,
//!                 source = %message.source,
//!                 "Received threat intelligence message"
//!             );
//!
//!             // Log security event for message processing
//!             warn!(
//!                 target: "somasync::security",
//!                 correlation_id = %correlation_id_clone.short(),
//!                 event = ?SecurityEvent::SignatureVerification {
//!                     message_id: message.id.to_string(),
//!                     node_id: "prod-intel-node".to_string(),
//!                     success: message.signature.is_some(),
//!                     algorithm: "ed25519".to_string(),
//!                 },
//!                 "Message signature verification completed"
//!             );
//!         }
//!     });
//!
//!     // Broadcast threat intelligence with automatic logging
//!     let threat_alert = MessageType::Alert {
//!         level: "high".to_string(),
//!         message: "Suspicious network activity detected".to_string(),
//!         details: None,
//!     };
//!
//!     // This will automatically log performance metrics and security events
//!     node.broadcast_message(threat_alert).await?;
//!
//!     info!(
//!         correlation_id = %correlation_id.short(),
//!         "Production logging demonstration completed"
//!     );
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod gossip;
pub mod logging;
pub mod mesh;
pub mod message;
pub mod node;
pub mod peer;

// Re-export main types for convenience
pub use error::SynapseError;
pub use gossip::{EnterpriseGossipConfig, GossipConfig, GossipProtocol, GossipStats, NetworkStats};
pub use logging::{
    development_config, init_logging, production_config, CorrelationId, LogConfig,
    PerformanceMetrics, PerformanceTimer, SecurityEvent, SpanExt,
};
pub use mesh::{MeshConfig, MeshNetwork, MeshStats, NetworkTopology, Route};
pub use message::{priority, ttl, Message, MessageBatch, MessageEnvelope, MessageType};
pub use node::{SynapseConfig, SynapseEvent, SynapseNode, SynapseNodeBuilder, SynapseStats};
pub use peer::{DiscoveryConfig, DiscoveryMethod, Peer, PeerManager, PeerState, PeerStats};

/// Result type alias for SomaSync operations
pub type Result<T> = std::result::Result<T, SynapseError>;
