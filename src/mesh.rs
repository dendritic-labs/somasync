//! Mesh network topology and routing for distributed systems
//!
//! This module implements a self-organizing mesh network that automatically
//! discovers optimal routes and maintains network connectivity.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::error::SynapseError;
use crate::message::Message;
use crate::peer::{PeerManager, PeerState};

/// Mesh network configuration
#[derive(Debug, Clone)]
pub struct MeshConfig {
    /// Maximum number of direct connections per node
    pub max_connections: usize,
    /// Minimum number of connections to maintain
    pub min_connections: usize,
    /// Route discovery timeout
    pub route_timeout: Duration,
    /// Route cache expiration
    pub route_cache_ttl: Duration,
    /// Maximum route hops
    pub max_route_hops: u8,
    /// Topology update interval
    pub topology_update_interval: Duration,
    /// Connection health check interval
    pub health_check_interval: Duration,
    /// Network partition detection threshold
    pub partition_threshold: Duration,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 3,
            route_timeout: Duration::from_secs(30),
            route_cache_ttl: Duration::from_secs(300),
            max_route_hops: 8,
            topology_update_interval: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(30),
            partition_threshold: Duration::from_secs(180),
        }
    }
}

/// Network route information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// Destination node ID
    pub destination: String,
    /// Next hop in the route
    pub next_hop: String,
    /// Total number of hops to destination
    pub hop_count: u8,
    /// Route cost/weight
    pub cost: u32,
    /// Route creation timestamp
    pub created_at: u64,
    /// Route last used timestamp
    pub last_used: u64,
    /// Route reliability score (0-100)
    pub reliability: u8,
    /// Alternative routes for redundancy
    pub alternatives: Vec<String>,
}

impl Route {
    /// Create a new route
    pub fn new(destination: String, next_hop: String, hop_count: u8, cost: u32) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            destination,
            next_hop,
            hop_count,
            cost,
            created_at: now,
            last_used: now,
            reliability: 100,
            alternatives: Vec::new(),
        }
    }

    /// Check if route is expired
    pub fn is_expired(&self, ttl: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now - self.created_at > ttl.as_secs()
    }

    /// Update route usage
    pub fn mark_used(&mut self) {
        self.last_used = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Decrease reliability on failure
    pub fn mark_failed(&mut self) {
        self.reliability = self.reliability.saturating_sub(10);
    }

    /// Increase reliability on success
    pub fn mark_successful(&mut self) {
        self.reliability = (self.reliability + 2).min(100);
    }
}

/// Network topology information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTopology {
    /// Node connections map
    pub connections: HashMap<String, HashSet<String>>,
    /// Node metadata
    pub node_info: HashMap<String, NodeInfo>,
    /// Topology version for consistency
    pub version: u64,
    /// Topology update timestamp
    pub updated_at: u64,
}

/// Information about a network node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node identifier
    pub id: String,
    /// Node network address
    pub address: SocketAddr,
    /// Node capabilities
    pub capabilities: HashSet<String>,
    /// Node load factor (0-100)
    pub load: u8,
    /// Node uptime
    pub uptime: u64,
    /// Node version
    pub version: Option<String>,
}

/// Mesh network protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeshMessage {
    /// Route discovery request
    RouteDiscovery {
        target: String,
        origin: String,
        trace: Vec<String>,
        timestamp: u64,
    },
    /// Route discovery response
    RouteResponse {
        target: String,
        origin: String,
        route: Vec<String>,
        cost: u32,
        timestamp: u64,
    },
    /// Topology update announcement
    TopologyUpdate {
        node_id: String,
        connections: HashSet<String>,
        node_info: NodeInfo,
        version: u64,
    },
    /// Network partition detection
    PartitionDetected {
        partition_id: String,
        affected_nodes: HashSet<String>,
        timestamp: u64,
    },
    /// Health check ping
    HealthPing {
        node_id: String,
        timestamp: u64,
        sequence: u64,
    },
    /// Health check pong
    HealthPong {
        node_id: String,
        timestamp: u64,
        sequence: u64,
        original_timestamp: u64,
    },
}

/// Mesh network implementation
pub struct MeshNetwork {
    /// Our node identifier
    node_id: String,
    /// Mesh configuration
    config: MeshConfig,
    /// Peer manager for connection handling
    peer_manager: Arc<PeerManager>,
    /// Routing table
    routing_table: Arc<RwLock<HashMap<String, Route>>>,
    /// Network topology
    topology: Arc<RwLock<NetworkTopology>>,
    /// Pending route discoveries
    pending_discoveries: Arc<RwLock<HashMap<String, (String, u64)>>>,
    /// Message sender for outbound mesh messages
    outbound_tx: mpsc::UnboundedSender<(String, MeshMessage)>,
    /// Health check sequence counter
    health_sequence: Arc<RwLock<u64>>,
}

impl MeshNetwork {
    /// Create a new mesh network
    pub fn new(
        node_id: String,
        config: MeshConfig,
        peer_manager: Arc<PeerManager>,
    ) -> (Self, mpsc::UnboundedReceiver<(String, MeshMessage)>) {
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();

        let topology = NetworkTopology {
            connections: HashMap::new(),
            node_info: HashMap::new(),
            version: 0,
            updated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let mesh = Self {
            node_id,
            config,
            peer_manager,
            routing_table: Arc::new(RwLock::new(HashMap::new())),
            topology: Arc::new(RwLock::new(topology)),
            pending_discoveries: Arc::new(RwLock::new(HashMap::new())),
            outbound_tx,
            health_sequence: Arc::new(RwLock::new(0)),
        };

        (mesh, outbound_rx)
    }

    /// Start the mesh network
    pub async fn start(&self) -> Result<(), SynapseError> {
        info!("Starting mesh network for node {}", self.node_id);

        // Start various mesh tasks
        let topology_task = async { self.start_topology_updates().await };
        let health_task = async { self.start_health_checks().await };
        let route_cleanup_task = async { self.start_route_cleanup().await };
        let partition_detection_task = async { self.start_partition_detection().await };

        tokio::try_join!(
            topology_task,
            health_task,
            route_cleanup_task,
            partition_detection_task
        )?;

        Ok(())
    }

    /// Route a message to a destination
    pub async fn route_message(&self, target: &str, message: Message) -> Result<(), SynapseError> {
        // Check if we have a direct connection
        let peers = self.peer_manager.get_healthy_peers().await;
        if peers.iter().any(|p| p.id == target) {
            // Direct connection available
            return self.send_direct(target, message).await;
        }

        // Look up route in routing table
        let route = {
            let routing_table = self.routing_table.read().await;
            routing_table.get(target).cloned()
        };

        match route {
            Some(mut route) if !route.is_expired(self.config.route_cache_ttl) => {
                // Use cached route
                route.mark_used();
                let next_hop = route.next_hop.clone();

                // Update route in table
                {
                    let mut routing_table = self.routing_table.write().await;
                    routing_table.insert(target.to_string(), route);
                }

                self.send_via_route(&next_hop, target, message).await
            }
            _ => {
                // Need to discover route
                self.discover_route(target).await?;

                // For now, return an error - in practice, we might queue the message
                Err(SynapseError::network("Route discovery in progress", target))
            }
        }
    }

    /// Handle incoming mesh message
    pub async fn handle_mesh_message(
        &self,
        _from: &str,
        message: MeshMessage,
    ) -> Result<(), SynapseError> {
        match message {
            MeshMessage::RouteDiscovery {
                target,
                origin,
                mut trace,
                timestamp,
            } => {
                debug!("Received route discovery for {} from {}", target, origin);

                // Check for loops
                if trace.contains(&self.node_id) {
                    return Ok(());
                }

                trace.push(self.node_id.clone());

                if target == self.node_id {
                    // We are the target, send response
                    let response = MeshMessage::RouteResponse {
                        target: target.clone(),
                        origin: origin.clone(),
                        route: trace.clone(),
                        cost: trace.len() as u32,
                        timestamp,
                    };

                    self.send_mesh_message(&origin, response).await?;
                } else {
                    // Forward the discovery
                    let discovery = MeshMessage::RouteDiscovery {
                        target,
                        origin,
                        trace,
                        timestamp,
                    };

                    self.broadcast_to_neighbors(discovery).await?;
                }
            }

            MeshMessage::RouteResponse {
                target,
                origin,
                route,
                cost,
                timestamp: _,
            } => {
                debug!("Received route response for {} from {}", target, origin);

                if origin == self.node_id && !route.is_empty() {
                    // This response is for us
                    if let Some(next_hop) = route.get(1) {
                        let new_route = Route::new(
                            target.clone(),
                            next_hop.clone(),
                            route.len() as u8 - 1,
                            cost,
                        );

                        let mut routing_table = self.routing_table.write().await;
                        routing_table.insert(target.clone(), new_route);

                        // Remove from pending discoveries
                        let mut pending = self.pending_discoveries.write().await;
                        pending.remove(&target);
                    }
                }
            }

            MeshMessage::TopologyUpdate {
                node_id,
                connections,
                node_info,
                version,
            } => {
                debug!("Received topology update from {}", node_id);
                self.update_topology(node_id, connections, node_info, version)
                    .await?;
            }

            MeshMessage::PartitionDetected {
                partition_id,
                affected_nodes,
                timestamp: _,
            } => {
                warn!(
                    "Network partition detected: {} affecting {} nodes",
                    partition_id,
                    affected_nodes.len()
                );
                self.handle_partition(affected_nodes).await?;
            }

            MeshMessage::HealthPing {
                node_id,
                timestamp,
                sequence,
            } => {
                debug!("Received health ping from {} (seq: {})", node_id, sequence);

                let pong = MeshMessage::HealthPong {
                    node_id: self.node_id.clone(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    sequence,
                    original_timestamp: timestamp,
                };

                self.send_mesh_message(&node_id, pong).await?;
            }

            MeshMessage::HealthPong {
                node_id,
                timestamp: _,
                sequence,
                original_timestamp,
            } => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let latency = now - original_timestamp;

                debug!(
                    "Received health pong from {} (seq: {}, latency: {}ms)",
                    node_id,
                    sequence,
                    latency * 1000
                );

                // Update peer metrics
                if let Some(mut peer) = self.peer_manager.get_peer(&node_id).await {
                    peer.update_metrics(latency * 1000, 0); // Convert to ms
                    self.peer_manager.add_peer(peer).await?;
                }
            }
        }

        Ok(())
    }

    /// Discover route to target
    async fn discover_route(&self, target: &str) -> Result<(), SynapseError> {
        // Check if discovery is already in progress
        {
            let pending = self.pending_discoveries.read().await;
            if pending.contains_key(target) {
                return Ok(()); // Discovery already in progress
            }
        }

        // Mark discovery as pending
        {
            let mut pending = self.pending_discoveries.write().await;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            pending.insert(target.to_string(), (self.node_id.clone(), now));
        }

        // Send route discovery
        let discovery = MeshMessage::RouteDiscovery {
            target: target.to_string(),
            origin: self.node_id.clone(),
            trace: vec![self.node_id.clone()],
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        self.broadcast_to_neighbors(discovery).await?;

        info!("Started route discovery for {}", target);
        Ok(())
    }

    /// Send message directly to a peer
    async fn send_direct(&self, target: &str, _message: Message) -> Result<(), SynapseError> {
        debug!("Sending message directly to {}", target);

        // Mark peer as seen
        self.peer_manager.mark_peer_seen(target).await?;

        // In a real implementation, this would send the message
        // through the appropriate transport mechanism

        Ok(())
    }

    /// Send message via discovered route
    async fn send_via_route(
        &self,
        next_hop: &str,
        _target: &str,
        _message: Message,
    ) -> Result<(), SynapseError> {
        debug!("Routing message to {} via {}", _target, next_hop);

        // In a real implementation, this would forward the message
        // to the next hop with appropriate routing headers

        Ok(())
    }

    /// Send mesh protocol message
    async fn send_mesh_message(
        &self,
        target: &str,
        message: MeshMessage,
    ) -> Result<(), SynapseError> {
        if let Err(e) = self.outbound_tx.send((target.to_string(), message)) {
            error!("Failed to send mesh message to {}: {}", target, e);
            return Err(SynapseError::network("Failed to send mesh message", target));
        }
        Ok(())
    }

    /// Broadcast message to all neighbors
    async fn broadcast_to_neighbors(&self, message: MeshMessage) -> Result<(), SynapseError> {
        let peers = self.peer_manager.get_healthy_peers().await;

        for peer in peers {
            if let Err(e) = self.send_mesh_message(&peer.id, message.clone()).await {
                warn!("Failed to broadcast to {}: {}", peer.id, e);
            }
        }

        Ok(())
    }

    /// Update network topology
    async fn update_topology(
        &self,
        node_id: String,
        connections: HashSet<String>,
        node_info: NodeInfo,
        version: u64,
    ) -> Result<(), SynapseError> {
        let mut topology = self.topology.write().await;

        // Only update if version is newer
        if version > topology.version {
            topology.connections.insert(node_id.clone(), connections);
            topology.node_info.insert(node_id, node_info);
            topology.version = version;
            topology.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            debug!("Updated topology to version {}", version);
        }

        Ok(())
    }

    /// Handle network partition
    async fn handle_partition(&self, affected_nodes: HashSet<String>) -> Result<(), SynapseError> {
        // Remove affected nodes from routing table
        {
            let mut routing_table = self.routing_table.write().await;
            for node in &affected_nodes {
                routing_table.remove(node);
            }
        }

        // Mark affected peers as disconnected
        for node in affected_nodes {
            if let Err(e) = self
                .peer_manager
                .update_peer_state(&node, PeerState::Disconnected)
                .await
            {
                warn!("Failed to update peer state for {}: {}", node, e);
            }
        }

        Ok(())
    }

    /// Start topology update task
    async fn start_topology_updates(&self) -> Result<(), SynapseError> {
        let mut interval = interval(self.config.topology_update_interval);

        loop {
            interval.tick().await;

            let peers = self.peer_manager.get_healthy_peers().await;
            let connections: HashSet<String> = peers.iter().map(|p| p.id.clone()).collect();

            let node_info = NodeInfo {
                id: self.node_id.clone(),
                address: "127.0.0.1:0".parse().unwrap_or_else(|_| ([127, 0, 0, 1], 0).into()), // This should be our actual address
                capabilities: HashSet::new(),
                load: 50,  // This should be calculated based on actual load
                uptime: 0, // This should be actual uptime
                version: Some("1.0.0".to_string()),
            };

            let topology_version = {
                let topology = self.topology.read().await;
                topology.version + 1
            };

            let update = MeshMessage::TopologyUpdate {
                node_id: self.node_id.clone(),
                connections,
                node_info,
                version: topology_version,
            };

            if let Err(e) = self.broadcast_to_neighbors(update).await {
                warn!("Failed to broadcast topology update: {}", e);
            }
        }
    }

    /// Start health check task
    async fn start_health_checks(&self) -> Result<(), SynapseError> {
        let mut interval = interval(self.config.health_check_interval);

        loop {
            interval.tick().await;

            let sequence = {
                let mut seq = self.health_sequence.write().await;
                *seq += 1;
                *seq
            };

            let ping = MeshMessage::HealthPing {
                node_id: self.node_id.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                sequence,
            };

            if let Err(e) = self.broadcast_to_neighbors(ping).await {
                warn!("Failed to broadcast health ping: {}", e);
            }
        }
    }

    /// Start route cleanup task
    async fn start_route_cleanup(&self) -> Result<(), SynapseError> {
        let mut interval = interval(Duration::from_secs(300)); // 5 minutes

        loop {
            interval.tick().await;

            let mut routing_table = self.routing_table.write().await;
            let expired_routes: Vec<String> = routing_table
                .iter()
                .filter(|(_, route)| route.is_expired(self.config.route_cache_ttl))
                .map(|(dest, _)| dest.clone())
                .collect();

            for dest in expired_routes {
                routing_table.remove(&dest);
                debug!("Removed expired route to {}", dest);
            }
        }
    }

    /// Start partition detection task
    async fn start_partition_detection(&self) -> Result<(), SynapseError> {
        let mut interval = interval(self.config.partition_threshold);

        loop {
            interval.tick().await;

            // Simple partition detection based on peer connectivity
            let healthy_peers = self.peer_manager.get_healthy_peers().await;

            if healthy_peers.len() < self.config.min_connections {
                warn!(
                    "Potential network partition detected: only {} healthy peers",
                    healthy_peers.len()
                );

                // In a more sophisticated implementation, this would
                // analyze the network topology to detect actual partitions
            }
        }
    }

    /// Get mesh network statistics
    pub async fn get_stats(&self) -> MeshStats {
        let routing_table = self.routing_table.read().await;
        let topology = self.topology.read().await;
        let peers = self.peer_manager.get_healthy_peers().await;

        MeshStats {
            routes_cached: routing_table.len(),
            healthy_connections: peers.len(),
            topology_version: topology.version,
            known_nodes: topology.node_info.len(),
        }
    }
}

/// Mesh network statistics
#[derive(Debug, Clone)]
pub struct MeshStats {
    pub routes_cached: usize,
    pub healthy_connections: usize,
    pub topology_version: u64,
    pub known_nodes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::{DiscoveryConfig, PeerManager};

    #[test]
    fn test_route_creation() {
        let route = Route::new("dest".to_string(), "next".to_string(), 3, 100);

        assert_eq!(route.destination, "dest");
        assert_eq!(route.next_hop, "next");
        assert_eq!(route.hop_count, 3);
        assert_eq!(route.cost, 100);
        assert_eq!(route.reliability, 100);
    }

    #[test]
    fn test_route_reliability() {
        let mut route = Route::new("dest".to_string(), "next".to_string(), 1, 10);

        let initial_reliability = route.reliability;

        route.mark_failed();
        let failed_reliability = route.reliability;
        assert!(failed_reliability < initial_reliability);

        route.mark_successful();
        let success_reliability = route.reliability;
        assert!(success_reliability > failed_reliability);
    }

    #[tokio::test]
    async fn test_mesh_creation() {
        let config = MeshConfig::default();
        let peer_config = DiscoveryConfig::default();
        let peer_manager = Arc::new(PeerManager::new("test".to_string(), peer_config));

        let (mesh, _rx) = MeshNetwork::new("test-node".to_string(), config, peer_manager);

        let stats = mesh.get_stats().await;
        assert_eq!(stats.routes_cached, 0);
        assert_eq!(stats.healthy_connections, 0);
    }

    #[test]
    fn test_mesh_config() {
        let config = MeshConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 3);
        assert_eq!(config.max_route_hops, 8);
    }
}
