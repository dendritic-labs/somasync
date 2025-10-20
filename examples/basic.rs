//! Basic example showing how to use the Synapse library

use somasync::{Peer, SynapseNodeBuilder};
use tokio::time::{sleep, Duration};
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting Synapse example");

    // Create first node
    let (mut node1, mut msg_rx1, mut event_rx1) = SynapseNodeBuilder::new()
        .with_node_id("node-1".to_string())
        .with_bind_address("127.0.0.1:8080".parse()?)
        .build()?;

    // Create second node
    let (mut node2, mut msg_rx2, mut event_rx2) = SynapseNodeBuilder::new()
        .with_node_id("node-2".to_string())
        .with_bind_address("127.0.0.1:8081".parse()?)
        .add_bootstrap_peer("127.0.0.1:8080".parse()?)
        .build()?;

    info!("Created nodes: {} and {}", node1.node_id(), node2.node_id());

    // Add node2 as a peer to node1
    let peer2 = Peer::new("node-2".to_string(), "127.0.0.1:8081".parse()?);
    node1.add_peer(peer2).await?;

    // Add node1 as a peer to node2
    let peer1 = Peer::new("node-1".to_string(), "127.0.0.1:8080".parse()?);
    node2.add_peer(peer1).await?;

    // Spawn tasks to handle messages and events
    tokio::spawn(async move {
        while let Some(message) = msg_rx1.recv().await {
            info!("Node 1 received message: {:?}", message);
        }
    });

    tokio::spawn(async move {
        while let Some(event) = event_rx1.recv().await {
            info!("Node 1 event: {:?}", event);
        }
    });

    tokio::spawn(async move {
        while let Some(message) = msg_rx2.recv().await {
            info!("Node 2 received message: {:?}", message);
        }
    });

    tokio::spawn(async move {
        while let Some(event) = event_rx2.recv().await {
            info!("Node 2 event: {:?}", event);
        }
    });

    // Start both nodes in background
    let node1_handle = tokio::spawn(async move {
        if let Err(e) = node1.start().await {
            eprintln!("Node 1 failed: {}", e);
        }
    });

    let node2_handle = tokio::spawn(async move {
        if let Err(e) = node2.start().await {
            eprintln!("Node 2 failed: {}", e);
        }
    });

    // Wait a bit for nodes to start
    sleep(Duration::from_secs(2)).await;

    info!("Nodes started, example completed");

    // Let it run for a few seconds
    sleep(Duration::from_secs(5)).await;

    // Clean shutdown
    node1_handle.abort();
    node2_handle.abort();

    Ok(())
}
