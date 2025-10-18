//! Peer management and discovery for distributed mesh networks
//!
//! This module provides functionality for discovering, connecting to, and
//! managing peers in the neural mesh network.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::SynapseError;

/// Peer connection state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PeerState {
    /// Peer is disconnected
    Disconnected,
    /// Attempting to connect to peer
    Connecting,
    /// Peer is connected and healthy
    Connected,
    /// Peer is connected but unhealthy
    Unhealthy,
    /// Peer connection failed
    Failed,
    /// Peer is being validated
    Validating,
}

/// Information about a network peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    /// Unique peer identifier
    pub id: String,
    /// Network address
    pub address: SocketAddr,
    /// Current connection state
    pub state: PeerState,
    /// Last successful communication timestamp
    pub last_seen: u64,
    /// Number of consecutive failures
    pub failure_count: u32,
    /// Peer capabilities and metadata
    pub metadata: HashMap<String, String>,
    /// Trust score (0-100)
    pub trust_score: u8,
    /// Connection latency in milliseconds
    pub latency_ms: Option<u64>,
    /// Bandwidth estimate in bytes/sec
    pub bandwidth_estimate: Option<u64>,
    /// Version information
    pub version: Option<String>,
    /// Supported protocols
    pub protocols: HashSet<String>,
}

impl Peer {
    /// Create a new peer
    pub fn new(id: String, address: SocketAddr) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            id,
            address,
            state: PeerState::Disconnected,
            last_seen: now,
            failure_count: 0,
            metadata: HashMap::new(),
            trust_score: 50, // Neutral trust
            latency_ms: None,
            bandwidth_estimate: None,
            version: None,
            protocols: HashSet::new(),
        }
    }
    
    /// Create peer with metadata
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
    
    /// Create peer with version
    pub fn with_version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }
    
    /// Add supported protocol
    pub fn add_protocol(mut self, protocol: String) -> Self {
        self.protocols.insert(protocol);
        self
    }
    
    /// Check if peer is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.state, PeerState::Connected) && self.failure_count < 3
    }
    
    /// Check if peer is stale (hasn't been seen recently)
    pub fn is_stale(&self, threshold: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now - self.last_seen > threshold.as_secs()
    }
    
    /// Update last seen timestamp
    pub fn update_last_seen(&mut self, timestamp: u64) {
        self.last_seen = timestamp;
        self.failure_count = 0; // Reset failure count on successful contact
    }
    
    /// Mark peer as healthy
    pub fn mark_healthy(&mut self) {
        self.state = PeerState::Connected;
        self.failure_count = 0;
        if self.trust_score < 100 {
            self.trust_score = (self.trust_score + 1).min(100);
        }
    }
    
    /// Mark peer as unhealthy
    pub fn mark_unhealthy(&mut self) {
        self.state = PeerState::Unhealthy;
        self.failure_count += 1;
        if self.trust_score > 0 {
            self.trust_score = self.trust_score.saturating_sub(5);
        }
    }
    
    /// Mark peer as failed
    pub fn mark_failed(&mut self) {
        self.state = PeerState::Failed;
        self.failure_count += 1;
        self.trust_score = self.trust_score.saturating_sub(10);
    }
    
    /// Update connection metrics
    pub fn update_metrics(&mut self, latency_ms: u64, bandwidth_bps: u64) {
        self.latency_ms = Some(latency_ms);
        self.bandwidth_estimate = Some(bandwidth_bps);
    }
    
    /// Get peer quality score (0-100)
    pub fn quality_score(&self) -> u8 {
        let mut score = self.trust_score;
        
        // Adjust for latency
        if let Some(latency) = self.latency_ms {
            if latency < 50 {
                score = score.saturating_add(10);
            } else if latency > 500 {
                score = score.saturating_sub(20);
            }
        }
        
        // Adjust for failure count
        score = score.saturating_sub(self.failure_count as u8 * 5);
        
        score.min(100)
    }
    
    /// Check if peer supports a protocol
    pub fn supports_protocol(&self, protocol: &str) -> bool {
        self.protocols.contains(protocol)
    }
    
    /// Get peer uptime estimate
    pub fn uptime_estimate(&self) -> Duration {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Duration::from_secs(now - self.last_seen)
    }
}

/// Configuration for peer discovery
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Discovery methods to use
    pub methods: HashSet<DiscoveryMethod>,
    /// Bootstrap peers for initial connection
    pub bootstrap_peers: Vec<SocketAddr>,
    /// Discovery interval
    pub discovery_interval: Duration,
    /// Maximum number of peers to maintain
    pub max_peers: usize,
    /// Minimum number of peers to maintain
    pub min_peers: usize,
    /// Peer validation timeout
    pub validation_timeout: Duration,
    /// Local discovery port ranges
    pub local_port_range: Option<(u16, u16)>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            methods: vec![
                DiscoveryMethod::Bootstrap,
                DiscoveryMethod::Gossip,
            ].into_iter().collect(),
            bootstrap_peers: Vec::new(),
            discovery_interval: Duration::from_secs(60),
            max_peers: 50,
            min_peers: 3,
            validation_timeout: Duration::from_secs(10),
            local_port_range: None,
        }
    }
}

/// Peer discovery methods
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum DiscoveryMethod {
    /// Bootstrap from known peer list
    Bootstrap,
    /// Discover through gossip protocol
    Gossip,
    /// Local network scanning
    LocalScan,
    /// mDNS/Bonjour discovery
    Mdns,
    /// DHT-based discovery
    Dht,
    /// Static configuration
    Static,
}

/// Peer discovery and management
pub struct PeerManager {
    /// Our node identifier
    node_id: String,
    /// Known peers
    peers: Arc<RwLock<HashMap<String, Peer>>>,
    /// Discovery configuration
    config: DiscoveryConfig,
    /// Peer quality cache
    quality_cache: Arc<RwLock<HashMap<String, (u8, u64)>>>, // (score, timestamp)
}

impl PeerManager {
    /// Create a new peer manager
    pub fn new(node_id: String, config: DiscoveryConfig) -> Self {
        Self {
            node_id,
            peers: Arc::new(RwLock::new(HashMap::new())),
            config,
            quality_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add a peer to the manager
    pub async fn add_peer(&self, peer: Peer) -> Result<(), SynapseError> {
        let mut peers = self.peers.write().await;
        
        // Don't add ourselves
        if peer.id == self.node_id {
            return Ok(());
        }
        
        // Check if we're at capacity
        if peers.len() >= self.config.max_peers {
            // Remove lowest quality peer if needed
            self.evict_worst_peer(&mut peers).await;
        }
        
        let peer_id = peer.id.clone();
        peers.insert(peer_id.clone(), peer);
        
        info!("Added peer {} to peer manager", peer_id);
        Ok(())
    }
    
    /// Remove a peer
    pub async fn remove_peer(&self, peer_id: &str) -> Result<(), SynapseError> {
        let mut peers = self.peers.write().await;
        if peers.remove(peer_id).is_some() {
            info!("Removed peer {} from peer manager", peer_id);
        }
        Ok(())
    }
    
    /// Get a peer by ID
    pub async fn get_peer(&self, peer_id: &str) -> Option<Peer> {
        let peers = self.peers.read().await;
        peers.get(peer_id).cloned()
    }
    
    /// Get all healthy peers
    pub async fn get_healthy_peers(&self) -> Vec<Peer> {
        let peers = self.peers.read().await;
        peers.values()
            .filter(|p| p.is_healthy())
            .cloned()
            .collect()
    }
    
    /// Get best peers for communication (by quality score)
    pub async fn get_best_peers(&self, count: usize) -> Vec<Peer> {
        let peers = self.peers.read().await;
        let mut peer_list: Vec<_> = peers.values()
            .filter(|p| p.is_healthy())
            .cloned()
            .collect();
        
        // Sort by quality score descending
        peer_list.sort_by(|a, b| b.quality_score().cmp(&a.quality_score()));
        
        peer_list.into_iter().take(count).collect()
    }
    
    /// Get random healthy peers
    pub async fn get_random_peers(&self, count: usize) -> Vec<Peer> {
        use rand::seq::SliceRandom;
        
        let peers = self.peers.read().await;
        let healthy_peers: Vec<_> = peers.values()
            .filter(|p| p.is_healthy())
            .cloned()
            .collect();
        
        let mut rng = rand::thread_rng();
        healthy_peers.choose_multiple(&mut rng, count).cloned().collect()
    }
    
    /// Update peer state
    pub async fn update_peer_state(&self, peer_id: &str, state: PeerState) -> Result<(), SynapseError> {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.state = state;
            debug!("Updated peer {} state to {:?}", peer_id, peer.state);
        }
        Ok(())
    }
    
    /// Mark peer as seen
    pub async fn mark_peer_seen(&self, peer_id: &str) -> Result<(), SynapseError> {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            peer.update_last_seen(now);
            peer.mark_healthy();
        }
        Ok(())
    }
    
    /// Mark peer as failed
    pub async fn mark_peer_failed(&self, peer_id: &str) -> Result<(), SynapseError> {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.mark_failed();
            warn!("Marked peer {} as failed (failure count: {})", peer_id, peer.failure_count);
        }
        Ok(())
    }
    
    /// Start peer discovery
    pub async fn start_discovery(&self) -> Result<(), SynapseError> {
        info!("Starting peer discovery for node {}", self.node_id);
        
        // Bootstrap from known peers
        if self.config.methods.contains(&DiscoveryMethod::Bootstrap) {
            self.bootstrap_discovery().await?;
        }
        
        // Start periodic discovery
        let discovery_task = async { self.start_discovery_timer().await };
        let cleanup_task = async { self.start_cleanup_timer().await };
        
        tokio::try_join!(discovery_task, cleanup_task)?;
        
        Ok(())
    }
    
    /// Bootstrap discovery from known peers
    async fn bootstrap_discovery(&self) -> Result<(), SynapseError> {
        for address in &self.config.bootstrap_peers {
            let peer_id = format!("bootstrap-{}", address);
            let peer = Peer::new(peer_id, *address)
                .add_protocol("bootstrap".to_string());
            
            if let Err(e) = self.add_peer(peer).await {
                warn!("Failed to add bootstrap peer {}: {}", address, e);
            }
        }
        
        info!("Bootstrap discovery completed with {} peers", self.config.bootstrap_peers.len());
        Ok(())
    }
    
    /// Start discovery timer
    async fn start_discovery_timer(&self) -> Result<(), SynapseError> {
        let mut interval = tokio::time::interval(self.config.discovery_interval);
        
        loop {
            interval.tick().await;
            
            let peer_count = {
                let peers = self.peers.read().await;
                peers.len()
            };
            
            if peer_count < self.config.min_peers {
                info!("Peer count ({}) below minimum ({}), starting discovery", 
                     peer_count, self.config.min_peers);
                
                if let Err(e) = self.discover_new_peers().await {
                    warn!("Peer discovery failed: {}", e);
                }
            }
        }
    }
    
    /// Start cleanup timer
    async fn start_cleanup_timer(&self) -> Result<(), SynapseError> {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
        
        loop {
            interval.tick().await;
            self.cleanup_stale_peers().await;
        }
    }
    
    /// Discover new peers
    async fn discover_new_peers(&self) -> Result<(), SynapseError> {
        // This would implement various discovery mechanisms
        // For now, we'll just log that discovery is happening
        debug!("Discovering new peers using methods: {:?}", self.config.methods);
        
        // Implementation would depend on specific discovery methods
        // e.g., DHT lookup, mDNS scan, gossip requests, etc.
        
        Ok(())
    }
    
    /// Clean up stale and failed peers
    async fn cleanup_stale_peers(&self) {
        let mut peers = self.peers.write().await;
        let stale_threshold = Duration::from_secs(3600); // 1 hour
        let mut to_remove = Vec::new();
        
        for (peer_id, peer) in peers.iter() {
            if peer.is_stale(stale_threshold) || 
               (peer.state == PeerState::Failed && peer.failure_count > 5) {
                to_remove.push(peer_id.clone());
            }
        }
        
        for peer_id in to_remove {
            peers.remove(&peer_id);
            info!("Removed stale/failed peer: {}", peer_id);
        }
    }
    
    /// Evict worst peer to make room
    async fn evict_worst_peer(&self, peers: &mut HashMap<String, Peer>) {
        let worst_peer = peers.iter()
            .min_by_key(|(_, peer)| peer.quality_score())
            .map(|(id, _)| id.clone());
        
        if let Some(peer_id) = worst_peer {
            peers.remove(&peer_id);
            info!("Evicted worst peer: {}", peer_id);
        }
    }
    
    /// Get peer statistics
    pub async fn get_stats(&self) -> PeerStats {
        let peers = self.peers.read().await;
        
        let total_peers = peers.len();
        let healthy_peers = peers.values().filter(|p| p.is_healthy()).count();
        let connecting_peers = peers.values().filter(|p| p.state == PeerState::Connecting).count();
        let failed_peers = peers.values().filter(|p| p.state == PeerState::Failed).count();
        
        let avg_trust_score = if total_peers > 0 {
            peers.values().map(|p| p.trust_score as u32).sum::<u32>() / total_peers as u32
        } else {
            0
        };
        
        let avg_latency = {
            let latencies: Vec<u64> = peers.values()
                .filter_map(|p| p.latency_ms)
                .collect();
            
            if !latencies.is_empty() {
                Some(latencies.iter().sum::<u64>() / latencies.len() as u64)
            } else {
                None
            }
        };
        
        PeerStats {
            total_peers,
            healthy_peers,
            connecting_peers,
            failed_peers,
            avg_trust_score: avg_trust_score as u8,
            avg_latency_ms: avg_latency,
        }
    }
}

/// Peer management statistics
#[derive(Debug, Clone)]
pub struct PeerStats {
    pub total_peers: usize,
    pub healthy_peers: usize,
    pub connecting_peers: usize,
    pub failed_peers: usize,
    pub avg_trust_score: u8,
    pub avg_latency_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    
    #[test]
    fn test_peer_creation() {
        let addr = SocketAddr::from_str("127.0.0.1:8080").unwrap();
        let peer = Peer::new("test-peer".to_string(), addr);
        
        assert_eq!(peer.id, "test-peer");
        assert_eq!(peer.address, addr);
        assert_eq!(peer.state, PeerState::Disconnected);
        assert_eq!(peer.trust_score, 50);
    }
    
    #[test]
    fn test_peer_health() {
        let addr = SocketAddr::from_str("127.0.0.1:8080").unwrap();
        let mut peer = Peer::new("test-peer".to_string(), addr);
        
        assert!(!peer.is_healthy());
        
        peer.mark_healthy();
        assert!(peer.is_healthy());
        assert_eq!(peer.state, PeerState::Connected);
        
        peer.mark_unhealthy();
        assert!(!peer.is_healthy());
    }
    
    #[test]
    fn test_peer_quality_score() {
        let addr = SocketAddr::from_str("127.0.0.1:8080").unwrap();
        let mut peer = Peer::new("test-peer".to_string(), addr);
        
        let initial_score = peer.quality_score();
        
        peer.update_metrics(20, 1000000); // Low latency, good bandwidth
        let good_score = peer.quality_score();
        assert!(good_score >= initial_score);
        
        peer.mark_failed();
        let bad_score = peer.quality_score();
        assert!(bad_score < good_score);
    }
    
    #[tokio::test]
    async fn test_peer_manager() {
        let config = DiscoveryConfig::default();
        let manager = PeerManager::new("test-node".to_string(), config);
        
        let addr = SocketAddr::from_str("127.0.0.1:8080").unwrap();
        let peer = Peer::new("test-peer".to_string(), addr);
        
        assert!(manager.add_peer(peer).await.is_ok());
        
        let retrieved = manager.get_peer("test-peer").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test-peer");
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_peers, 1);
    }
    
    #[test]
    fn test_discovery_config() {
        let config = DiscoveryConfig::default();
        assert!(config.methods.contains(&DiscoveryMethod::Bootstrap));
        assert!(config.methods.contains(&DiscoveryMethod::Gossip));
        assert_eq!(config.max_peers, 50);
        assert_eq!(config.min_peers, 3);
    }
}