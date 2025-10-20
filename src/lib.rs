//! # Synapse - Distributed Mesh Networking Library
//!
//! Synapse provides neural-inspired distributed mesh networking capabilities with gossip protocols
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
//! ## Quick Start
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

pub mod error;
pub mod gossip;
pub mod mesh;
pub mod message;
pub mod node;
pub mod peer;

// Re-export main types for convenience
pub use error::SynapseError;
pub use gossip::{EnterpriseGossipConfig, GossipConfig, GossipProtocol, GossipStats, NetworkStats};
pub use mesh::{MeshConfig, MeshNetwork, MeshStats, NetworkTopology, Route};
pub use message::{priority, ttl, Message, MessageBatch, MessageEnvelope, MessageType};
pub use node::{SynapseConfig, SynapseEvent, SynapseNode, SynapseNodeBuilder, SynapseStats};
pub use peer::{DiscoveryConfig, DiscoveryMethod, Peer, PeerManager, PeerState, PeerStats};

/// Result type alias for synapse operations
pub type Result<T> = std::result::Result<T, SynapseError>;
