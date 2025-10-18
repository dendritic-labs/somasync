//! Gossip Protocol for Distributed Message Propagation
//!
//! Neural-inspired gossip protocol that efficiently propagates messages across
//! the mesh network with minimal overhead and anti-entropy guarantees.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::error::SynapseError;
use crate::message::{Message, MessageEnvelope};
use crate::peer::Peer;

/// Configuration for the gossip protocol
#[derive(Debug, Clone)]
pub struct GossipConfig {
    /// Maximum number of peers to gossip with per round (fanout)
    pub fanout: usize,
    /// Interval between gossip rounds
    pub gossip_interval: Duration,
    /// Maximum age of gossip messages
    pub max_message_age: Duration,
    /// Maximum number of messages to store
    pub max_stored_messages: usize,
    /// Network timeout for peer communication
    pub network_timeout: Duration,
    /// Maximum number of hops for message propagation
    pub max_hops: u8,
    /// Anti-entropy sync interval
    pub sync_interval: Duration,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            fanout: 3,
            gossip_interval: Duration::from_secs(30),
            max_message_age: Duration::from_secs(3600),
            max_stored_messages: 10000,
            network_timeout: Duration::from_secs(5),
            max_hops: 5,
            sync_interval: Duration::from_secs(300),
        }
    }
}

/// Types of gossip messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GossipMessage {
    /// User data message
    Data(Message),
    /// Request for specific message by hash
    MessageRequest(u64),
    /// Heartbeat to maintain peer connections
    Heartbeat { node_id: String, timestamp: u64 },
    /// Anti-entropy synchronization request
    SyncRequest { known_hashes: Vec<u64> },
    /// Anti-entropy synchronization response
    SyncResponse { missing_messages: Vec<Message> },
    /// Acknowledgment of received message
    Ack { message_id: u64 },
}

/// Message cache for deduplication and anti-entropy
#[derive(Debug)]
struct MessageCache {
    /// Known message IDs with timestamps
    known_messages: HashMap<u64, u64>,
    /// Message cache by hash
    message_cache: HashMap<u64, Message>,
    /// Message queue for pending delivery
    #[allow(dead_code)]
    pending_queue: VecDeque<MessageEnvelope>,
    /// Maximum cache size
    max_size: usize,
}

impl MessageCache {
    /// Create a new message cache
    fn new(max_size: usize) -> Self {
        Self {
            known_messages: HashMap::new(),
            message_cache: HashMap::new(),
            pending_queue: VecDeque::new(),
            max_size,
        }
    }

    /// Check if we've seen this message before
    fn has_seen(&self, message_id: u64) -> bool {
        self.known_messages.contains_key(&message_id)
    }

    /// Record that we've seen this message
    fn mark_seen(&mut self, message_id: u64, timestamp: u64) {
        if self.known_messages.len() >= self.max_size {
            self.evict_old_messages();
        }
        self.known_messages.insert(message_id, timestamp);
    }

    /// Add message to cache
    fn cache_message(&mut self, message: Message) {
        let hash = message.id;
        self.message_cache.insert(hash, message);

        if self.message_cache.len() > self.max_size {
            self.evict_old_messages_from_cache();
        }
    }

    /// Get message from cache
    fn get_message(&self, hash: u64) -> Option<&Message> {
        self.message_cache.get(&hash)
    }

    /// Get all known message hashes
    fn known_message_hashes(&self) -> Vec<u64> {
        self.message_cache.keys().copied().collect()
    }

    /// Evict old messages to maintain cache size
    fn evict_old_messages(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Remove messages older than 1 hour
        self.known_messages
            .retain(|_, timestamp| now - *timestamp < 3600);

        // If still over limit, remove oldest 25%
        if self.known_messages.len() >= self.max_size {
            let entries_to_remove: Vec<u64> = {
                let mut entries: Vec<_> = self.known_messages.iter().collect();
                entries.sort_by_key(|(_, timestamp)| *timestamp);

                let remove_count = self.max_size / 4;
                entries
                    .iter()
                    .take(remove_count)
                    .map(|(message_id, _)| **message_id)
                    .collect()
            };

            for message_id in entries_to_remove {
                self.known_messages.remove(&message_id);
            }
        }
    }

    /// Evict old messages from cache
    fn evict_old_messages_from_cache(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Remove expired messages
        self.message_cache.retain(|_, message| {
            now - message.timestamp < 3600 // 1 hour
        });

        // If still over limit, remove oldest 25%
        if self.message_cache.len() > self.max_size {
            let hashes_to_remove: Vec<u64> = {
                let mut entries: Vec<_> = self.message_cache.iter().collect();
                entries.sort_by_key(|(_, message)| message.timestamp);

                let remove_count = self.max_size / 4;
                entries
                    .iter()
                    .take(remove_count)
                    .map(|(hash, _)| **hash)
                    .collect()
            };

            for hash in hashes_to_remove {
                self.message_cache.remove(&hash);
            }
        }
    }
}

/// Gossip protocol implementation
pub struct GossipProtocol {
    /// Our node identifier
    node_id: String,
    /// Protocol configuration
    config: GossipConfig,
    /// Known peers
    peers: Arc<RwLock<HashMap<String, Peer>>>,
    /// Message cache
    cache: Arc<RwLock<MessageCache>>,
    /// Channel for outgoing messages
    outbound_tx: mpsc::UnboundedSender<(SocketAddr, MessageEnvelope)>,
    /// Channel for incoming messages
    inbound_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<MessageEnvelope>>>>,
    /// Channel for processed messages
    message_tx: mpsc::UnboundedSender<Message>,
}

impl GossipProtocol {
    /// Create a new gossip protocol instance
    pub fn new(
        node_id: String,
        config: GossipConfig,
        message_tx: mpsc::UnboundedSender<Message>,
    ) -> (Self, mpsc::UnboundedReceiver<(SocketAddr, MessageEnvelope)>) {
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        let (_inbound_tx, inbound_rx) = mpsc::unbounded_channel();

        let protocol = Self {
            node_id,
            config: config.clone(),
            peers: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(MessageCache::new(config.max_stored_messages))),
            outbound_tx,
            inbound_rx: Arc::new(RwLock::new(Some(inbound_rx))),
            message_tx,
        };

        (protocol, outbound_rx)
    }

    /// Start the gossip protocol
    pub async fn start(&self) -> Result<(), SynapseError> {
        info!("Starting gossip protocol for node {}", self.node_id);

        // Start gossip timer
        let gossip_task = async { self.start_gossip_timer().await };

        // Start sync timer
        let sync_task = async { self.start_sync_timer().await };

        // Start message processor
        let process_task = async { self.start_message_processor().await };

        // Start peer health checker
        let health_task = async { self.start_health_checker().await };

        // Join all tasks
        tokio::try_join!(gossip_task, sync_task, process_task, health_task)?;

        Ok(())
    }

    /// Add a peer to the gossip network
    pub async fn add_peer(&self, peer: Peer) -> Result<(), SynapseError> {
        let mut peers = self.peers.write().await;
        let node_id = peer.id.clone();
        peers.insert(node_id.clone(), peer);

        info!("Added peer {} to gossip network", node_id);
        Ok(())
    }

    /// Remove a peer from the network
    pub async fn remove_peer(&self, node_id: &str) -> Result<(), SynapseError> {
        let mut peers = self.peers.write().await;
        if peers.remove(node_id).is_some() {
            info!("Removed peer {} from gossip network", node_id);
        }
        Ok(())
    }

    /// Gossip a message to the network
    pub async fn gossip_message(&self, message: Message) -> Result<(), SynapseError> {
        let envelope = MessageEnvelope::new(self.node_id.clone(), GossipMessage::Data(message));
        self.broadcast_message(envelope).await
    }

    /// Process incoming gossip message
    pub async fn handle_message(&self, envelope: MessageEnvelope) -> Result<(), SynapseError> {
        // Check if message is expired or has too many hops
        if envelope.is_expired() {
            debug!("Dropping expired message {}", envelope.message_id);
            return Ok(());
        }

        if envelope.is_hop_exceeded(self.config.max_hops) {
            debug!("Dropping message {} - too many hops", envelope.message_id);
            return Ok(());
        }

        // Check for duplicate
        let mut cache = self.cache.write().await;
        if cache.has_seen(envelope.message_id) {
            debug!("Ignoring duplicate message {}", envelope.message_id);
            return Ok(());
        }

        // Mark as seen
        cache.mark_seen(envelope.message_id, envelope.timestamp);

        // Process message based on type
        match &envelope.message {
            GossipMessage::Data(message) => {
                debug!("Received data message from {}", envelope.source_node);
                cache.cache_message(message.clone());

                // Forward to local message processor
                if let Err(e) = self.message_tx.send(message.clone()) {
                    warn!("Failed to forward message: {}", e);
                }

                // Forward to other peers (with incremented hop count)
                let mut forward_envelope = envelope.clone();
                forward_envelope.increment_hop();
                self.forward_message(forward_envelope).await?;
            }

            GossipMessage::MessageRequest(hash) => {
                debug!("Received message request for hash {}", hash);
                if let Some(message) = cache.get_message(*hash) {
                    let _response = MessageEnvelope::new(
                        self.node_id.clone(),
                        GossipMessage::Data(message.clone()),
                    );
                    // Send directly back to requester
                    // This would be implemented with direct peer communication
                }
            }

            GossipMessage::Heartbeat { node_id, timestamp } => {
                debug!("Received heartbeat from {} at {}", node_id, timestamp);
                self.update_peer_heartbeat(node_id, *timestamp).await;
            }

            GossipMessage::SyncRequest { known_hashes } => {
                debug!("Received sync request from {}", envelope.source_node);
                self.handle_sync_request(&envelope.source_node, known_hashes)
                    .await?;
            }

            GossipMessage::SyncResponse { missing_messages } => {
                debug!(
                    "Received sync response with {} messages",
                    missing_messages.len()
                );
                for message in missing_messages {
                    cache.cache_message(message.clone());
                    if let Err(e) = self.message_tx.send(message.clone()) {
                        warn!("Failed to forward synced message: {}", e);
                    }
                }
            }

            GossipMessage::Ack { message_id } => {
                debug!("Received ack for message {}", message_id);
                // Handle acknowledgment if needed
            }
        }

        Ok(())
    }

    /// Broadcast message to random subset of peers
    async fn broadcast_message(&self, envelope: MessageEnvelope) -> Result<(), SynapseError> {
        let peers = self.peers.read().await;
        let healthy_peers: Vec<_> = peers
            .values()
            .filter(|p| p.is_healthy() && p.id != self.node_id)
            .collect();

        if healthy_peers.is_empty() {
            warn!("No healthy peers available for gossip");
            return Ok(());
        }

        // Select random subset of peers (fanout)
        let fanout = self.config.fanout.min(healthy_peers.len());
        let mut selected_peers = Vec::new();

        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        let mut peer_indices: Vec<usize> = (0..healthy_peers.len()).collect();
        peer_indices.shuffle(&mut rng);

        for i in 0..fanout {
            selected_peers.push(&healthy_peers[peer_indices[i]]);
        }

        // Send to selected peers
        for peer in selected_peers {
            if let Err(e) = self.outbound_tx.send((peer.address, envelope.clone())) {
                error!("Failed to queue message for peer {}: {}", peer.id, e);
            }
        }

        Ok(())
    }

    /// Forward message to peers (excluding source)
    async fn forward_message(&self, envelope: MessageEnvelope) -> Result<(), SynapseError> {
        let peers = self.peers.read().await;
        let forward_peers: Vec<_> = peers
            .values()
            .filter(|p| p.is_healthy() && p.id != envelope.source_node && p.id != self.node_id)
            .collect();

        let fanout = (self.config.fanout / 2).max(1).min(forward_peers.len());

        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        let selected_peers: Vec<_> = forward_peers.choose_multiple(&mut rng, fanout).collect();

        for peer in selected_peers {
            if let Err(e) = self.outbound_tx.send((peer.address, envelope.clone())) {
                error!("Failed to forward message to peer {}: {}", peer.id, e);
            }
        }

        Ok(())
    }

    /// Start gossip timer
    async fn start_gossip_timer(&self) -> Result<(), SynapseError> {
        let mut interval = interval(self.config.gossip_interval);
        let node_id = self.node_id.clone();
        let peers = Arc::clone(&self.peers);
        let outbound_tx = self.outbound_tx.clone();

        loop {
            interval.tick().await;

            // Send heartbeat
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let heartbeat = MessageEnvelope::new(
                node_id.clone(),
                GossipMessage::Heartbeat {
                    node_id: node_id.clone(),
                    timestamp: now,
                },
            );

            let peers_guard = peers.read().await;
            for peer in peers_guard.values().filter(|p| p.is_healthy()) {
                if let Err(e) = outbound_tx.send((peer.address, heartbeat.clone())) {
                    error!("Failed to send heartbeat to {}: {}", peer.id, e);
                }
            }
        }
    }

    /// Start anti-entropy synchronization timer
    async fn start_sync_timer(&self) -> Result<(), SynapseError> {
        let mut interval = interval(self.config.sync_interval);
        let node_id = self.node_id.clone();
        let peers = Arc::clone(&self.peers);
        let cache = Arc::clone(&self.cache);
        let outbound_tx = self.outbound_tx.clone();

        loop {
            interval.tick().await;

            let known_hashes = {
                let cache_guard = cache.read().await;
                cache_guard.known_message_hashes()
            };

            let sync_request =
                MessageEnvelope::new(node_id.clone(), GossipMessage::SyncRequest { known_hashes });

            // Send sync request to one random peer
            let peers_guard = peers.read().await;
            let healthy_peers: Vec<_> = peers_guard.values().filter(|p| p.is_healthy()).collect();

            if !healthy_peers.is_empty() {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                if let Some(peer) = healthy_peers.choose(&mut rng) {
                    if let Err(e) = outbound_tx.send((peer.address, sync_request)) {
                        error!("Failed to send sync request to {}: {}", peer.id, e);
                    }
                }
            }
        }
    }

    /// Start message processor
    async fn start_message_processor(&self) -> Result<(), SynapseError> {
        let mut inbound_rx = {
            let mut rx_guard = self.inbound_rx.write().await;
            rx_guard
                .take()
                .ok_or_else(|| SynapseError::config("Message processor already started"))?
        };

        loop {
            if let Some(envelope) = inbound_rx.recv().await {
                if let Err(e) = self.handle_message(envelope).await {
                    error!("Failed to handle gossip message: {}", e);
                }
            }
        }
    }

    /// Start peer health checker
    async fn start_health_checker(&self) -> Result<(), SynapseError> {
        let mut interval = interval(Duration::from_secs(60)); // Check every minute
        let peers = Arc::clone(&self.peers);
        let stale_threshold = Duration::from_secs(300); // 5 minutes

        loop {
            interval.tick().await;

            let mut peers_guard = peers.write().await;
            let mut stale_peers = Vec::new();

            for (node_id, peer) in peers_guard.iter_mut() {
                if peer.is_stale(stale_threshold) {
                    peer.mark_unhealthy();
                    stale_peers.push(node_id.clone());
                }
            }

            for node_id in stale_peers {
                warn!("Peer {} marked as unhealthy due to staleness", node_id);
            }
        }
    }

    /// Update peer heartbeat
    async fn update_peer_heartbeat(&self, node_id: &str, timestamp: u64) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(node_id) {
            peer.update_last_seen(timestamp);
            peer.mark_healthy();
        }
    }

    /// Handle synchronization request
    async fn handle_sync_request(
        &self,
        requester: &str,
        known_hashes: &[u64],
    ) -> Result<(), SynapseError> {
        let cache = self.cache.read().await;
        let our_hashes: HashSet<u64> = cache.known_message_hashes().into_iter().collect();
        let their_hashes: HashSet<u64> = known_hashes.iter().copied().collect();

        // Find messages they don't have
        let missing_hashes: Vec<u64> = our_hashes.difference(&their_hashes).copied().collect();
        let missing_messages: Vec<Message> = missing_hashes
            .iter()
            .filter_map(|hash| cache.get_message(*hash).cloned())
            .collect();

        if !missing_messages.is_empty() {
            let sync_response = MessageEnvelope::new(
                self.node_id.clone(),
                GossipMessage::SyncResponse { missing_messages },
            );

            // Send response to requester
            let peers = self.peers.read().await;
            if let Some(peer) = peers.values().find(|p| p.id == requester) {
                if let Err(e) = self.outbound_tx.send((peer.address, sync_response)) {
                    error!("Failed to send sync response to {}: {}", requester, e);
                }
            }
        }

        Ok(())
    }

    /// Get protocol statistics
    pub async fn get_stats(&self) -> GossipStats {
        let peers = self.peers.read().await;
        let cache = self.cache.read().await;

        let healthy_peers = peers.values().filter(|p| p.is_healthy()).count();
        let total_peers = peers.len();
        let cached_messages = cache.message_cache.len();
        let known_messages = cache.known_messages.len();

        GossipStats {
            healthy_peers,
            total_peers,
            cached_messages,
            known_messages,
        }
    }
}

/// Gossip protocol statistics
#[derive(Debug, Clone)]
pub struct GossipStats {
    pub healthy_peers: usize,
    pub total_peers: usize,
    pub cached_messages: usize,
    pub known_messages: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageType;
    use tokio::sync::mpsc;

    #[test]
    fn test_gossip_config_defaults() {
        let config = GossipConfig::default();
        assert_eq!(config.fanout, 3);
        assert_eq!(config.max_hops, 5);
    }

    #[tokio::test]
    async fn test_gossip_protocol_creation() {
        let config = GossipConfig::default();
        let (message_tx, _message_rx) = mpsc::unbounded_channel();
        let (protocol, _outbound_rx) =
            GossipProtocol::new("test-node".to_string(), config, message_tx);

        let stats = protocol.get_stats().await;
        assert_eq!(stats.total_peers, 0);
    }

    #[tokio::test]
    async fn test_message_cache() {
        let mut cache = MessageCache::new(100);

        // Test duplicate detection
        assert!(!cache.has_seen(12345));
        cache.mark_seen(12345, 1234567890);
        assert!(cache.has_seen(12345));

        // Test message caching
        let message = Message::new(
            MessageType::Data("test".to_string()),
            "test-node".to_string(),
        );

        let hash = message.id;
        cache.cache_message(message);

        assert!(cache.get_message(hash).is_some());
        let hashes = cache.known_message_hashes();
        assert!(hashes.contains(&hash));
    }
}
