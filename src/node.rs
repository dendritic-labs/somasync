//! Main Synapse node implementation
//!
//! This module provides the SynapseNode which coordinates all the distributed
//! networking components and provides a high-level API for mesh communication.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::error::SynapseError;
use crate::gossip::{GossipConfig, GossipProtocol, GossipStats};
use crate::mesh::{MeshConfig, MeshNetwork, MeshStats};
use crate::message::{Message, MessageType};
use crate::peer::{DiscoveryConfig, Peer, PeerManager, PeerStats};

/// Configuration for a Synapse node
#[derive(Debug, Clone)]
pub struct SynapseConfig {
    /// Node identifier (if None, a UUID will be generated)
    pub node_id: Option<String>,
    /// Local bind address for the node
    pub bind_address: SocketAddr,
    /// Gossip protocol configuration
    pub gossip: GossipConfig,
    /// Mesh network configuration
    pub mesh: MeshConfig,
    /// Peer discovery configuration
    pub discovery: DiscoveryConfig,
    /// Message processing buffer size
    pub message_buffer_size: usize,
    /// Enable automatic peer discovery
    pub auto_discovery: bool,
    /// Network interface name (optional)
    pub interface: Option<String>,
}

impl Default for SynapseConfig {
    fn default() -> Self {
        Self {
            node_id: None,
            bind_address: "127.0.0.1:0".parse().unwrap_or_else(|_| ([127, 0, 0, 1], 0).into()),
            gossip: GossipConfig::default(),
            mesh: MeshConfig::default(),
            discovery: DiscoveryConfig::default(),
            message_buffer_size: 1000,
            auto_discovery: true,
            interface: None,
        }
    }
}

/// Statistics for the entire Synapse node
#[derive(Debug, Clone)]
pub struct SynapseStats {
    /// Node identifier
    pub node_id: String,
    /// Node uptime in seconds
    pub uptime: u64,
    /// Total messages processed
    pub messages_processed: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Peer statistics
    pub peer_stats: PeerStats,
    /// Gossip statistics
    pub gossip_stats: GossipStats,
    /// Mesh statistics
    pub mesh_stats: MeshStats,
}

/// Event types that can be emitted by the Synapse node
#[derive(Debug, Clone)]
pub enum SynapseEvent {
    /// Node started successfully
    NodeStarted {
        node_id: String,
        address: SocketAddr,
    },
    /// Node is shutting down
    NodeStopping { node_id: String },
    /// New peer connected
    PeerConnected {
        peer_id: String,
        address: SocketAddr,
    },
    /// Peer disconnected
    PeerDisconnected { peer_id: String, reason: String },
    /// Message received from network
    MessageReceived { from: String, message: Box<Message> },
    /// Message sent to network
    MessageSent { to: String, message_id: u64 },
    /// Network partition detected
    PartitionDetected { affected_peers: Vec<String> },
    /// Route discovered
    RouteDiscovered { destination: String, hops: u8 },
    /// Error occurred
    Error { error: String },
}

/// Main Synapse node that coordinates all networking components
pub struct SynapseNode {
    /// Node configuration
    config: SynapseConfig,
    /// Unique node identifier
    node_id: String,
    /// Peer manager
    peer_manager: Arc<PeerManager>,
    /// Gossip protocol
    gossip: Arc<RwLock<Option<Arc<GossipProtocol>>>>,
    /// Mesh network
    mesh: Arc<RwLock<Option<Arc<MeshNetwork>>>>,
    /// Message processing channel
    message_tx: mpsc::UnboundedSender<Message>,
    /// Event notification channel
    event_tx: mpsc::UnboundedSender<SynapseEvent>,
    /// Node statistics
    stats: Arc<RwLock<SynapseStats>>,
    /// Shutdown signal
    shutdown_tx: Option<mpsc::UnboundedSender<()>>,
}

impl SynapseNode {
    /// Create a new Synapse node
    pub fn new(
        config: SynapseConfig,
    ) -> (
        Self,
        mpsc::UnboundedReceiver<Message>,
        mpsc::UnboundedReceiver<SynapseEvent>,
    ) {
        let node_id = config
            .node_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let peer_manager = Arc::new(PeerManager::new(node_id.clone(), config.discovery.clone()));

        let stats = SynapseStats {
            node_id: node_id.clone(),
            uptime: 0,
            messages_processed: 0,
            messages_sent: 0,
            peer_stats: PeerStats {
                total_peers: 0,
                healthy_peers: 0,
                connecting_peers: 0,
                failed_peers: 0,
                avg_trust_score: 0,
                avg_latency_ms: None,
            },
            gossip_stats: GossipStats {
                healthy_peers: 0,
                total_peers: 0,
                cached_messages: 0,
                known_messages: 0,
            },
            mesh_stats: MeshStats {
                routes_cached: 0,
                healthy_connections: 0,
                topology_version: 0,
                known_nodes: 0,
            },
        };

        let node = Self {
            config,
            node_id,
            peer_manager,
            gossip: Arc::new(RwLock::new(None)),
            mesh: Arc::new(RwLock::new(None)),
            message_tx,
            event_tx,
            stats: Arc::new(RwLock::new(stats)),
            shutdown_tx: None,
        };

        (node, message_rx, event_rx)
    }

    /// Start the Synapse node
    pub async fn start(&mut self) -> Result<(), SynapseError> {
        info!("Starting Synapse node {}", self.node_id);

        // Emit startup event
        self.emit_event(SynapseEvent::NodeStarted {
            node_id: self.node_id.clone(),
            address: self.config.bind_address,
        })
        .await;

        // Initialize gossip protocol
        let (gossip, _gossip_outbound_rx) = GossipProtocol::new(
            self.node_id.clone(),
            self.config.gossip.clone(),
            self.message_tx.clone(),
        );

        *self.gossip.write().await = Some(Arc::new(gossip));

        // Initialize mesh network
        let (mesh, _mesh_outbound_rx) = MeshNetwork::new(
            self.node_id.clone(),
            self.config.mesh.clone(),
            Arc::clone(&self.peer_manager),
        );

        *self.mesh.write().await = Some(Arc::new(mesh));

        // Set up shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();
        self.shutdown_tx = Some(shutdown_tx);

        // Start peer discovery if enabled
        if self.config.auto_discovery {
            let peer_manager = Arc::clone(&self.peer_manager);
            tokio::spawn(async move {
                if let Err(e) = peer_manager.start_discovery().await {
                    error!("Peer discovery failed: {}", e);
                }
            });
        }

        // Start gossip protocol
        {
            let gossip_guard = self.gossip.read().await;
            if let Some(gossip) = gossip_guard.as_ref() {
                let gossip_clone = Arc::clone(gossip);
                drop(gossip_guard);
                tokio::spawn(async move {
                    if let Err(e) = gossip_clone.start().await {
                        error!("Gossip protocol failed: {}", e);
                    }
                });
            }
        }

        // Start mesh network
        {
            let mesh_guard = self.mesh.read().await;
            if let Some(mesh) = mesh_guard.as_ref() {
                let mesh_clone = Arc::clone(mesh);
                drop(mesh_guard);
                tokio::spawn(async move {
                    if let Err(e) = mesh_clone.start().await {
                        error!("Mesh network failed: {}", e);
                    }
                });
            }
        }

        // Start statistics update task
        Self::start_stats_updater(Arc::clone(&self.stats));

        // Wait for shutdown signal
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Received shutdown signal for node {}", self.node_id);
            }
        }

        self.emit_event(SynapseEvent::NodeStopping {
            node_id: self.node_id.clone(),
        })
        .await;

        Ok(())
    }

    /// Stop the Synapse node
    pub async fn stop(&self) -> Result<(), SynapseError> {
        if let Some(shutdown_tx) = &self.shutdown_tx {
            let _ = shutdown_tx.send(());
        }
        Ok(())
    }

    /// Add a peer to the network
    pub async fn add_peer(&self, peer: Peer) -> Result<(), SynapseError> {
        let peer_id = peer.id.clone();
        let address = peer.address;

        self.peer_manager.add_peer(peer).await?;

        self.emit_event(SynapseEvent::PeerConnected { peer_id, address })
            .await;

        Ok(())
    }

    /// Remove a peer from the network
    pub async fn remove_peer(&self, peer_id: &str) -> Result<(), SynapseError> {
        self.peer_manager.remove_peer(peer_id).await?;

        self.emit_event(SynapseEvent::PeerDisconnected {
            peer_id: peer_id.to_string(),
            reason: "Manual removal".to_string(),
        })
        .await;

        Ok(())
    }

    /// Send a message to a specific peer
    pub async fn send_message(
        &self,
        target: &str,
        message_type: MessageType,
    ) -> Result<(), SynapseError> {
        let message = Message::new(message_type, self.node_id.clone());
        let message_id = message.id;

        // Try direct gossip first
        if let Some(gossip) = self.gossip.read().await.as_ref() {
            gossip.gossip_message(message.clone()).await?;
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.messages_sent += 1;
        }

        self.emit_event(SynapseEvent::MessageSent {
            to: target.to_string(),
            message_id,
        })
        .await;

        Ok(())
    }

    /// Broadcast a message to all peers
    pub async fn broadcast_message(&self, message_type: MessageType) -> Result<(), SynapseError> {
        let message = Message::new(message_type, self.node_id.clone());

        if let Some(gossip) = self.gossip.read().await.as_ref() {
            gossip.gossip_message(message).await?;
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.messages_sent += 1;
        }

        Ok(())
    }

    /// Route a message through the mesh network
    pub async fn route_message(
        &self,
        target: &str,
        message_type: MessageType,
    ) -> Result<(), SynapseError> {
        let message = Message::new(message_type, self.node_id.clone());
        let message_id = message.id;

        if let Some(mesh) = self.mesh.read().await.as_ref() {
            mesh.route_message(target, message).await?;
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.messages_sent += 1;
        }

        self.emit_event(SynapseEvent::MessageSent {
            to: target.to_string(),
            message_id,
        })
        .await;

        Ok(())
    }

    /// Get node statistics
    pub async fn get_stats(&self) -> SynapseStats {
        let mut stats = self.stats.write().await;

        // Update with current component stats
        stats.peer_stats = self.peer_manager.get_stats().await;

        if let Some(gossip) = self.gossip.read().await.as_ref() {
            stats.gossip_stats = gossip.get_stats().await;
        }

        if let Some(mesh) = self.mesh.read().await.as_ref() {
            stats.mesh_stats = mesh.get_stats().await;
        }

        stats.clone()
    }

    /// Get node ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Get node configuration
    pub fn config(&self) -> &SynapseConfig {
        &self.config
    }

    /// Get healthy peers
    pub async fn get_healthy_peers(&self) -> Vec<Peer> {
        self.peer_manager.get_healthy_peers().await
    }

    /// Get best peers for communication
    pub async fn get_best_peers(&self, count: usize) -> Vec<Peer> {
        self.peer_manager.get_best_peers(count).await
    }

    /// Handle incoming message from the network
    pub async fn handle_incoming_message(
        &self,
        from: &str,
        message: Message,
    ) -> Result<(), SynapseError> {
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.messages_processed += 1;
        }

        // Emit event
        self.emit_event(SynapseEvent::MessageReceived {
            from: from.to_string(),
            message: Box::new(message.clone()),
        })
        .await;

        // Forward to local message processor
        if let Err(e) = self.message_tx.send(message) {
            warn!("Failed to forward incoming message: {}", e);
        }

        Ok(())
    }

    /// Emit an event
    async fn emit_event(&self, event: SynapseEvent) {
        if let Err(e) = self.event_tx.send(event) {
            warn!("Failed to emit event: {}", e);
        }
    }

    /// Start statistics updater task
    fn start_stats_updater(stats: Arc<RwLock<SynapseStats>>) {
        let start_time = std::time::Instant::now();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                let mut stats_guard = stats.write().await;
                stats_guard.uptime = start_time.elapsed().as_secs();
            }
        });
    }
}

/// Builder for creating and configuring Synapse nodes
pub struct SynapseNodeBuilder {
    config: SynapseConfig,
}

impl SynapseNodeBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: SynapseConfig::default(),
        }
    }

    /// Set node ID
    pub fn with_node_id(mut self, node_id: String) -> Self {
        self.config.node_id = Some(node_id);
        self
    }

    /// Set bind address
    pub fn with_bind_address(mut self, address: SocketAddr) -> Self {
        self.config.bind_address = address;
        self
    }

    /// Set gossip configuration
    pub fn with_gossip_config(mut self, gossip: GossipConfig) -> Self {
        self.config.gossip = gossip;
        self
    }

    /// Set mesh configuration
    pub fn with_mesh_config(mut self, mesh: MeshConfig) -> Self {
        self.config.mesh = mesh;
        self
    }

    /// Set discovery configuration
    pub fn with_discovery_config(mut self, discovery: DiscoveryConfig) -> Self {
        self.config.discovery = discovery;
        self
    }

    /// Add bootstrap peer
    pub fn add_bootstrap_peer(mut self, address: SocketAddr) -> Self {
        self.config.discovery.bootstrap_peers.push(address);
        self
    }

    /// Enable/disable auto discovery
    pub fn with_auto_discovery(mut self, enabled: bool) -> Self {
        self.config.auto_discovery = enabled;
        self
    }

    /// Set message buffer size
    pub fn with_message_buffer_size(mut self, size: usize) -> Self {
        self.config.message_buffer_size = size;
        self
    }

    /// Build the Synapse node
    pub fn build(
        self,
    ) -> (
        SynapseNode,
        mpsc::UnboundedReceiver<Message>,
        mpsc::UnboundedReceiver<SynapseEvent>,
    ) {
        SynapseNode::new(self.config)
    }
}

impl Default for SynapseNodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_node_creation() {
        let (node, _msg_rx, _event_rx) = SynapseNodeBuilder::new()
            .with_node_id("test-node".to_string())
            .build();

        assert_eq!(node.node_id(), "test-node");
        assert_eq!(node.config().bind_address, "127.0.0.1:0".parse().unwrap());
    }

    #[tokio::test]
    async fn test_node_builder() {
        let addr = SocketAddr::from_str("192.168.1.100:8080").unwrap();
        let bootstrap_addr = SocketAddr::from_str("192.168.1.1:8080").unwrap();

        let (node, _msg_rx, _event_rx) = SynapseNodeBuilder::new()
            .with_node_id("builder-test".to_string())
            .with_bind_address(addr)
            .add_bootstrap_peer(bootstrap_addr)
            .with_auto_discovery(false)
            .build();

        assert_eq!(node.node_id(), "builder-test");
        assert_eq!(node.config().bind_address, addr);
        assert!(!node.config().auto_discovery);
        assert!(node
            .config()
            .discovery
            .bootstrap_peers
            .contains(&bootstrap_addr));
    }

    #[tokio::test]
    async fn test_node_stats() {
        let (node, _msg_rx, _event_rx) = SynapseNodeBuilder::new()
            .with_node_id("stats-test".to_string())
            .build();

        let stats = node.get_stats().await;
        assert_eq!(stats.node_id, "stats-test");
        assert_eq!(stats.messages_processed, 0);
        assert_eq!(stats.messages_sent, 0);
    }

    #[tokio::test]
    async fn test_add_peer() {
        let (node, _msg_rx, _event_rx) = SynapseNodeBuilder::new()
            .with_node_id("peer-test".to_string())
            .build();

        let addr = SocketAddr::from_str("127.0.0.1:9090").unwrap();
        let mut peer = Peer::new("test-peer".to_string(), addr);
        peer.mark_healthy(); // Mark peer as healthy for test

        assert!(node.add_peer(peer).await.is_ok());

        let peers = node.get_healthy_peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].id, "test-peer");
    }
}
