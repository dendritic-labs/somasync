# SomaSync

[![Crates.io](https://img.shields.io/crates/v/somasync)](https://crates.io/crates/somasync)
[![Documentation](https://docs.rs/somasync/badge.svg)](https://docs.rs/somasync)
[![License](https://img.shields.io/crates/l/somasync)](LICENSE)
[![Build Status](https://github.com/dendritic-labs/somasync/workflows/CI/badge.svg)](https://github.com/dendritic-labs/somasync/actions)

**Neural-Inspired Distributed Mesh Networking Library**

SomaSync is a high-performance, self-organizing mesh networking library that enables distributed systems to communicate efficiently through adaptive neural-like pathways.

## Features

- **Neural-Inspired Architecture**: Nodes communicate like neurons via synaptic connections
- **Self-Organizing Mesh**: Automatic peer discovery and topology optimization  
- **Gossip Protocols**: Efficient data propagation with anti-entropy guarantees
- **Message Signing**: Ed25519 cryptographic signatures for threat intel authenticity
- **Enterprise Gossip**: Optimized protocols for 10K+ node networks with adaptive algorithms
- **Self-Healing**: Automatic failure detection and network recovery
- **High Performance**: Zero-copy serialization and connection pooling
- **Production Ready**: Comprehensive error handling and monitoring

## Quick Start

Add SomaSync to your `Cargo.toml`:

```toml
[dependencies]
somasync = "0.1"
tokio = { version = "1.0", features = ["full"] }
# For message signing (optional)
ed25519-dalek = { version = "2.1", features = ["serde"] }
rand = "0.8"
```

Create a simple mesh network:

```rust
use somasync::{SynapseNodeBuilder, SynapseEvent, MessageType, Peer};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and configure a new node
    let (mut node, mut message_rx, mut event_rx) = SynapseNodeBuilder::new()
        .with_node_id("node-1".to_string())
        .with_bind_address("127.0.0.1:8080".parse()?)
        .build();

    // Add a peer
    let peer = Peer::new("node-2".to_string(), "127.0.0.1:8081".parse()?);
    node.add_peer(peer).await?;

    // Start the node (in background)
    tokio::spawn(async move {
        node.start().await.unwrap();
    });

    // Send a message to the network
    let (tx_node, _, _) = SynapseNodeBuilder::new()
        .with_node_id("sender".to_string())
        .build();
    
    tx_node.broadcast_message(
        MessageType::Data("Hello, neural mesh!".to_string())
    ).await?;

    Ok(())
}
```

## Architecture

SomaSync consists of several key components:

### Core Components

- **SynapseNode**: Main coordination component that manages all networking
- **PeerManager**: Handles peer discovery, connection management, and health monitoring
- **GossipProtocol**: Implements efficient message dissemination across the mesh
- **MeshNetwork**: Provides routing and topology management
- **Message System**: Handles message serialization, batching, and delivery

### Neural-Inspired Design

The architecture is inspired by neural networks:

- **Nodes** act as neurons
- **Connections** act as synapses  
- **Messages** flow along optimized pathways
- **Routing** adapts based on network conditions and usage patterns

## Examples

### Basic Mesh Network

```rust
use somasync::{SynapseNodeBuilder, Peer};

// Create two nodes
let (mut node1, _, _) = SynapseNodeBuilder::new()
    .with_node_id("alpha".to_string())
    .build();

let (mut node2, _, _) = SynapseNodeBuilder::new()
    .with_node_id("beta".to_string())
    .add_bootstrap_peer("127.0.0.1:8080".parse()?)
    .build();

// Connect them as peers
let peer_beta = Peer::new("beta".to_string(), "127.0.0.1:8081".parse()?);
node1.add_peer(peer_beta).await?;
```

### Message Broadcasting

```rust
use somasync::MessageType;

// Send high-priority message
node.broadcast_message(
    MessageType::Alert {
        level: "critical".to_string(),
        message: "System overload detected".to_string(),
        details: None,
    }
).await?;

// Send structured data
let mut data = std::collections::HashMap::new();
data.insert("temperature".to_string(), "85.2".to_string());
data.insert("unit".to_string(), "celsius".to_string());

node.broadcast_message(MessageType::Structured(data)).await?;
```

### Event Handling

```rust
use somasync::SynapseEvent;

// Handle incoming messages and events
tokio::spawn(async move {
    while let Some(message) = message_rx.recv().await {
        println!("Received: {:?}", message);
    }
});

tokio::spawn(async move {
    while let Some(event) = event_rx.recv().await {
        match event {
            SynapseEvent::PeerConnected { peer_id, address } => {
                println!("New peer connected: {} at {}", peer_id, address);
            }
            SynapseEvent::MessageReceived { from, message } => {
                println!("Message from {}: {:?}", from, *message);
            }
            _ => {}
        }
    }
});
```

### Message Signing for Security

```rust
use somasync::{Message, MessageType};
use ed25519_dalek::SigningKey;
use rand::RngCore;

// Generate a signing key for message authenticity
let mut csprng = rand::rngs::OsRng;
let mut secret_bytes = [0u8; 32];
csprng.fill_bytes(&mut secret_bytes);
let signing_key = SigningKey::from_bytes(&secret_bytes);

// Create and sign a threat intelligence message
let threat_message = Message::new(
    MessageType::Alert {
        level: "critical".to_string(),
        message: "APT detected in network".to_string(),
        details: Some(threat_details),
    },
    "threat-intel-node".to_string(),
)
.with_priority(255) // High priority
.sign_with_ed25519(&signing_key); // Cryptographic signature

// Recipients can verify message authenticity
match threat_message.verify_signature() {
    Ok(true) => println!("Message verified - authentic threat intel"),
    Ok(false) => println!("Message signature invalid"),
    Err(e) => println!("Verification failed: {}", e),
}
```

### Enterprise Gossip for Scale

```rust
use somasync::{EnterpriseGossipConfig, GossipProtocol};

// Configure enterprise gossip for 10K+ node networks
let enterprise_config = EnterpriseGossipConfig::for_threat_intel()
    .with_adaptive_fanout(20, 100)    // Scale fanout 20-100 based on network
    .with_priority_routing()           // Route critical threats first
    .with_smart_routing()              // Reputation-based peer selection
    .with_bandwidth_limit(50_000_000); // 50MB/s bandwidth limit

// Create enterprise gossip protocol
let (gossip_protocol, outbound_rx) = GossipProtocol::new_enterprise(
    "enterprise-node-001".to_string(),
    enterprise_config,
    message_tx,
);

// Enterprise protocols automatically:
// - Adapt fanout based on network size and health
// - Prioritize critical threat intelligence
// - Manage bandwidth for enterprise networks
// - Track peer reputation for optimal routing
```

## Configuration

SomaSync supports extensive configuration for both basic and enterprise deployments:

### Basic Configuration

```rust
use somasync::{SynapseNodeBuilder, GossipConfig};
use std::time::Duration;

let gossip_config = GossipConfig {
    fanout: 5,
    gossip_interval: Duration::from_secs(15),
    max_message_age: Duration::from_secs(1800),
    max_stored_messages: 10_000,
    ..Default::default()
};

let (node, _, _) = SynapseNodeBuilder::new()
    .with_node_id("configured-node".to_string())
    .with_gossip_config(gossip_config)
    .with_auto_discovery(true)
    .build();
```

### Enterprise Configuration

```rust
use somasync::EnterpriseGossipConfig;

// Threat intelligence network configuration
let enterprise_config = EnterpriseGossipConfig::for_threat_intel()
    .with_adaptive_fanout(10, 50)     // Scale based on network conditions
    .with_priority_routing()           // Critical alerts get priority
    .with_smart_routing()              // Reputation-based peer selection
    .with_bandwidth_limit(100_000_000) // 100MB/s limit
    .with_batch_size(100)             // Batch messages for efficiency
    .with_epidemic_tuning(0.1, 0.9);  // Fine-tune gossip parameters

// Large-scale SOC network configuration  
let large_scale_config = EnterpriseGossipConfig::for_large_networks()
    .with_adaptive_fanout(50, 200)    // Higher fanout for large networks
    .with_bandwidth_limit(1_000_000_000); // 1GB/s for high-throughput
```

## Monitoring

Get real-time network statistics:

```rust
let stats = node.get_stats().await;
println!("Node: {}", stats.node_id);
println!("Uptime: {}s", stats.uptime);
println!("Messages processed: {}", stats.messages_processed);
println!("Healthy peers: {}", stats.peer_stats.healthy_peers);
```

## Testing

Run the test suite:

```bash
cargo test
```

Run with coverage:

```bash
cargo test --all-features
```

## Contributing

**Development Status**: This project is currently in a development freeze while core features are stabilized for production deployment. The main branch is protected and new contributions are temporarily paused.

For future contribution opportunities and guidelines, please see [CONTRIBUTING.md](CONTRIBUTING.md).

## 📄 License

This project is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🔬 Research & Inspiration

SomaSync draws inspiration from:

- Neural network architectures and synaptic communication
- Gossip protocols and epidemic algorithms  
- Self-organizing systems and emergent behavior
- Distributed hash tables and peer-to-peer networks

## Roadmap

### Completed
- [x] **Message Signing**: Ed25519 cryptographic signatures for message authenticity
- [x] **Enterprise Gossip**: Optimized protocols for 10K+ node networks
- [x] **Adaptive Algorithms**: Dynamic fanout, priority routing, bandwidth management

### In Progress
- [ ] Basic network-level flood protection
- [ ] Monitoring and observability APIs
- [ ] Reputation system and fault detection

### Future
- [ ] WebRTC transport support
- [ ] Advanced routing algorithms (DHT, Kademlia)
- [ ] Network simulation and testing tools
- [ ] Performance benchmarking suite
- [ ] Integration with existing P2P frameworks

---

Made by [Dendritic Labs](https://github.com/dendritic-labs)