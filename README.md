# SomaSync 🧠

[![CraAdd this to your `Cargo.toml`:

```toml
[dependencies]
somasync = "0.1"o](https://img.shields.io/crates/v/somasync)](https://crates.io/crates/somasync)
[![Documentation](https://docs.rs/somasync/badge.svg)](https://docs.rs/somasync)
[![License](https://img.shields.io/crates/l/somasync)](LICENSE)

**Neural-Inspired Distributed Mesh Networking Library**

SomaSync is a high-performance, self-organizing mesh networking library that enables distributed systems to communicate efficiently through adaptive neural-like pathways.

## ✨ Features

- **🧠 Neural-Inspired Architecture**: Nodes communicate like neurons via synaptic connections
- **🌐 Self-Organizing Mesh**: Automatic peer discovery and topology optimization  
- **💬 Gossip Protocols**: Efficient data propagation with anti-entropy guarantees
- **🔄 Self-Healing**: Automatic failure detection and network recovery
- **⚡ High Performance**: Zero-copy serialization and connection pooling
- **🛡️ Production Ready**: Comprehensive error handling and monitoring

## 🚀 Quick Start

Add Synapse to your `Cargo.toml`:

```toml
[dependencies]
synapse = "0.1"
tokio = { version = "1.0", features = ["full"] }
```

Create a simple mesh network:

```rust
```rust
use somasync::{SynapseNodeBuilder, SynapseEvent, MessageType};
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

## 🏗️ Architecture

Synapse consists of several key components:

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

## 📖 Examples

### Basic Mesh Network

```rust
use synapse::{SynapseNodeBuilder, Peer};

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
use synapse::{MessageType, priority, ttl};

// Send high-priority message with custom TTL
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
                println!("Message from {}: {:?}", from, message);
            }
            _ => {}
        }
    }
});
```

## 🔧 Configuration

Synapse supports extensive configuration:

```rust
use synapse::{SynapseNodeBuilder, GossipConfig, MeshConfig, DiscoveryConfig};
use std::time::Duration;

let gossip_config = GossipConfig {
    fanout: 5,
    gossip_interval: Duration::from_secs(15),
    max_message_age: Duration::from_secs(1800),
    ..Default::default()
};

let mesh_config = MeshConfig {
    max_connections: 20,
    route_timeout: Duration::from_secs(10),
    ..Default::default()
};

let (node, _, _) = SynapseNodeBuilder::new()
    .with_node_id("configured-node".to_string())
    .with_gossip_config(gossip_config)
    .with_mesh_config(mesh_config)
    .with_auto_discovery(true)
    .build();
```

## 📊 Monitoring

Get real-time network statistics:

```rust
let stats = node.get_stats().await;
println!("Node: {}", stats.node_id);
println!("Uptime: {}s", stats.uptime);
println!("Messages processed: {}", stats.messages_processed);
println!("Healthy peers: {}", stats.peer_stats.healthy_peers);
println!("Routes cached: {}", stats.mesh_stats.routes_cached);
```

## 🧪 Testing

Run the test suite:

```bash
cargo test
```

Run with coverage:

```bash
cargo test --all-features
```

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

This project is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🔬 Research & Inspiration

Synapse draws inspiration from:

- Neural network architectures and synaptic communication
- Gossip protocols and epidemic algorithms  
- Self-organizing systems and emergent behavior
- Distributed hash tables and peer-to-peer networks

## 🛣️ Roadmap

- [ ] WebRTC transport support
- [ ] Byzantine fault tolerance
- [ ] Advanced routing algorithms (DHT, Kademlia)
- [ ] Network simulation and testing tools
- [ ] Performance benchmarking suite
- [ ] Integration with existing P2P frameworks

---

Made with ❤️ by [Dendritic Labs](https://github.com/dendritic-labs)