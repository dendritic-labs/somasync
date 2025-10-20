//! Message types and serialization for distributed communication
//!
//! This module defines the core message types used for communication
//! across the neural mesh     pub fn add_header(mut self, key: String, value: String) -> Self {
//! Message types and serialization for distributed communication
//!
//! This module defines the core message types used for communication
//! across the neural mesh network.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Types of messages that can be sent across the network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    /// Raw data payload
    Data(String),
    /// Structured key-value data
    Structured(HashMap<String, String>),
    /// Binary data payload
    Binary(Vec<u8>),
    /// Command or control message
    Command {
        action: String,
        params: HashMap<String, String>,
    },
    /// Status or heartbeat message
    Status { component: String, state: String },
    /// Error or alert message
    Alert {
        level: String,
        message: String,
        details: Option<HashMap<String, String>>,
    },
}

/// Digital signature for message integrity and authenticity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSignature {
    /// The actual signature bytes
    pub signature: Vec<u8>,
    /// Public key of the signer for verification
    pub public_key: Vec<u8>,
    /// Signature algorithm used
    pub algorithm: String,
    /// Timestamp when signature was created
    pub signed_at: u64,
}

impl MessageSignature {
    /// Create a new Ed25519 signature
    pub fn new_ed25519(signature: Signature, public_key: VerifyingKey) -> Self {
        let signed_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            signature: signature.to_bytes().to_vec(),
            public_key: public_key.to_bytes().to_vec(),
            algorithm: "Ed25519".to_string(),
            signed_at,
        }
    }

    /// Verify this signature against message content
    pub fn verify(&self, message_content: &[u8]) -> Result<(), crate::error::SynapseError> {
        match self.algorithm.as_str() {
            "Ed25519" => self.verify_ed25519(message_content),
            _ => Err(crate::error::SynapseError::Security(format!(
                "Unsupported signature algorithm: {}",
                self.algorithm
            ))),
        }
    }

    /// Verify Ed25519 signature
    fn verify_ed25519(&self, message_content: &[u8]) -> Result<(), crate::error::SynapseError> {
        // Reconstruct public key
        let public_key_bytes: [u8; 32] = self.public_key.as_slice().try_into().map_err(|_| {
            crate::error::SynapseError::Security("Invalid public key length".to_string())
        })?;
        let public_key = VerifyingKey::from_bytes(&public_key_bytes).map_err(|e| {
            crate::error::SynapseError::Security(format!("Invalid public key: {}", e))
        })?;

        // Reconstruct signature
        let signature_bytes: [u8; 64] = self.signature.as_slice().try_into().map_err(|_| {
            crate::error::SynapseError::Security("Invalid signature length".to_string())
        })?;
        let signature = Signature::try_from(signature_bytes.as_slice()).map_err(|e| {
            crate::error::SynapseError::Security(format!("Invalid signature: {}", e))
        })?;

        // Verify signature
        public_key
            .verify(message_content, &signature)
            .map_err(|e| {
                crate::error::SynapseError::Security(format!(
                    "Signature verification failed: {}",
                    e
                ))
            })?;

        Ok(())
    }
}

/// Core message structure for network communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message identifier
    pub id: u64,
    /// Message type and payload
    pub message_type: MessageType,
    /// Source node identifier
    pub source: String,
    /// Message timestamp (Unix epoch)
    pub timestamp: u64,
    /// Time-to-live in seconds
    pub ttl: u64,
    /// Message priority (0-255, higher = more priority)
    pub priority: u8,
    /// Optional correlation ID for request/response patterns
    pub correlation_id: Option<String>,
    /// Custom headers/metadata
    pub headers: HashMap<String, String>,
    /// Digital signature for message integrity (optional)
    pub signature: Option<MessageSignature>,
}

impl Message {
    pub fn new(message_type: MessageType, source: String) -> Self {
        let id = Self::generate_id(&message_type, &source);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id,
            message_type,
            source,
            timestamp,
            ttl: 3600,     // 1 hour default TTL
            priority: 128, // Medium priority
            correlation_id: None,
            headers: HashMap::new(),
            signature: None,
        }
    }

    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl = ttl_seconds;
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Add a header to the message
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Sign the message with Ed25519 for integrity verification
    pub fn sign_with_ed25519(mut self, signing_key: &SigningKey) -> Self {
        let message_content = self.get_signable_content();
        let signature = signing_key.sign(&message_content);
        let public_key = signing_key.verifying_key();

        self.signature = Some(MessageSignature::new_ed25519(signature, public_key));
        self
    }

    /// Verify the message signature
    pub fn verify_signature(&self) -> Result<bool, crate::error::SynapseError> {
        if let Some(signature) = &self.signature {
            let message_content = self.get_signable_content();
            signature.verify(&message_content)?;
            Ok(true)
        } else {
            Ok(false) // No signature present
        }
    }

    /// Get the content that should be signed (everything except signature)
    fn get_signable_content(&self) -> Vec<u8> {
        let mut signable = self.clone();
        signable.signature = None; // Remove signature for signing/verification

        // Serialize to bytes for signing
        serde_json::to_vec(&signable).unwrap_or_default()
    }

    /// Check if message is signed
    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    /// Get the public key of the signer (if message is signed)
    pub fn signer_public_key(&self) -> Option<&[u8]> {
        self.signature.as_ref().map(|s| s.public_key.as_slice())
    }

    /// Check if message has expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now > self.timestamp + self.ttl
    }

    /// Returns the age of this message in seconds since creation
    pub fn age(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now.saturating_sub(self.timestamp)
    }

    /// Returns how many seconds remain before this message expires
    pub fn remaining_ttl(&self) -> u64 {
        if self.is_expired() {
            0
        } else {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            (self.timestamp + self.ttl).saturating_sub(now)
        }
    }

    /// Generate a unique message ID based on content and source
    fn generate_id(message_type: &MessageType, source: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        // Hash the message content
        match message_type {
            MessageType::Data(data) => data.hash(&mut hasher),
            MessageType::Structured(map) => {
                for (k, v) in map {
                    k.hash(&mut hasher);
                    v.hash(&mut hasher);
                }
            }
            MessageType::Binary(data) => data.hash(&mut hasher),
            MessageType::Command { action, params } => {
                action.hash(&mut hasher);
                for (k, v) in params {
                    k.hash(&mut hasher);
                    v.hash(&mut hasher);
                }
            }
            MessageType::Status { component, state } => {
                component.hash(&mut hasher);
                state.hash(&mut hasher);
            }
            MessageType::Alert {
                level,
                message,
                details,
            } => {
                level.hash(&mut hasher);
                message.hash(&mut hasher);
                if let Some(details) = details {
                    for (k, v) in details {
                        k.hash(&mut hasher);
                        v.hash(&mut hasher);
                    }
                }
            }
        }

        // Hash the source and timestamp
        source.hash(&mut hasher);
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .hash(&mut hasher);

        // Add some randomness
        Uuid::new_v4().to_string().hash(&mut hasher);

        hasher.finish()
    }

    /// Create a response message correlated to this message
    pub fn create_response(&self, response_type: MessageType, source: String) -> Message {
        let correlation_id = self
            .correlation_id
            .clone()
            .unwrap_or_else(|| self.id.to_string());

        Message::new(response_type, source)
            .with_correlation_id(correlation_id)
            .with_priority(self.priority)
    }

    /// Get message size estimate in bytes
    pub fn estimated_size(&self) -> usize {
        let base_size = std::mem::size_of::<Message>();
        let type_size = match &self.message_type {
            MessageType::Data(data) => data.len(),
            MessageType::Structured(map) => map.iter().map(|(k, v)| k.len() + v.len()).sum(),
            MessageType::Binary(data) => data.len(),
            MessageType::Command { action, params } => {
                action.len() + params.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>()
            }
            MessageType::Status { component, state } => component.len() + state.len(),
            MessageType::Alert {
                level,
                message,
                details,
            } => {
                level.len()
                    + message.len()
                    + details
                        .as_ref()
                        .map(|d| d.iter().map(|(k, v)| k.len() + v.len()).sum())
                        .unwrap_or(0)
            }
        };

        let headers_size: usize = self.headers.iter().map(|(k, v)| k.len() + v.len()).sum();
        let correlation_size = self.correlation_id.as_ref().map(|s| s.len()).unwrap_or(0);
        let source_size = self.source.len();

        base_size + type_size + headers_size + correlation_size + source_size
    }
}

/// Message envelope for network transport with routing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// The actual message payload
    pub message: crate::gossip::GossipMessage,
    /// Unique envelope identifier
    pub message_id: u64,
    /// Source node that created this envelope
    pub source_node: String,
    /// Timestamp when envelope was created
    pub timestamp: u64,
    /// Number of hops this message has taken
    pub hop_count: u8,
    /// Maximum hops allowed
    pub max_hops: u8,
    /// Time-to-live for the envelope
    pub envelope_ttl: Duration,
    /// Routing hints for mesh traversal
    pub routing_hints: Vec<String>,
}

impl MessageEnvelope {
    /// Create a new message envelope
    pub fn new(source_node: String, message: crate::gossip::GossipMessage) -> Self {
        let message_id = Self::generate_envelope_id(&source_node, &message);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            message,
            message_id,
            source_node,
            timestamp,
            hop_count: 0,
            max_hops: 10,                            // Default max hops
            envelope_ttl: Duration::from_secs(3600), // 1 hour
            routing_hints: Vec::new(),
        }
    }

    /// Create envelope with custom max hops
    pub fn with_max_hops(mut self, max_hops: u8) -> Self {
        self.max_hops = max_hops;
        self
    }

    /// Create envelope with custom TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.envelope_ttl = ttl;
        self
    }

    /// Add routing hint
    pub fn with_routing_hint(mut self, hint: String) -> Self {
        self.routing_hints.push(hint);
        self
    }

    /// Increment hop count
    pub fn increment_hop(&mut self) {
        self.hop_count += 1;
    }

    /// Check if envelope has expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let created_secs = self.timestamp;

        now.saturating_sub(created_secs) > self.envelope_ttl.as_secs()
    }

    /// Check if hop count has been exceeded
    pub fn is_hop_exceeded(&self, max_hops: u8) -> bool {
        self.hop_count >= max_hops.min(self.max_hops)
    }

    /// Get envelope age
    pub fn age(&self) -> Duration {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let created_secs = self.timestamp;

        Duration::from_secs(now.saturating_sub(created_secs))
    }

    /// Generate unique envelope ID
    fn generate_envelope_id(source_node: &str, _message: &crate::gossip::GossipMessage) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        source_node.hash(&mut hasher);
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .hash(&mut hasher);
        Uuid::new_v4().to_string().hash(&mut hasher);

        hasher.finish()
    }

    /// Check if envelope should be routed through specific nodes
    pub fn should_route_through(&self, node_id: &str) -> bool {
        if self.routing_hints.is_empty() {
            true // No hints means route through anyone
        } else {
            self.routing_hints.contains(&node_id.to_string())
        }
    }

    /// Get estimated envelope size
    pub fn estimated_size(&self) -> usize {
        let base_size = std::mem::size_of::<MessageEnvelope>();
        let source_size = self.source_node.len();
        let hints_size: usize = self.routing_hints.iter().map(|h| h.len()).sum();

        // Note: This doesn't include the actual message size as it would be recursive
        // The message size should be calculated separately
        base_size + source_size + hints_size
    }
}

/// Batch of messages for efficient transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBatch {
    /// Batch identifier
    pub batch_id: String,
    /// Messages in this batch
    pub messages: Vec<MessageEnvelope>,
    /// Batch creation timestamp
    pub created_at: u64,
    /// Compression algorithm used (if any)
    pub compression: Option<String>,
    /// Batch checksum for integrity
    pub checksum: Option<u64>,
}

impl MessageBatch {
    /// Create a new message batch
    pub fn new(messages: Vec<MessageEnvelope>) -> Self {
        let batch_id = Uuid::new_v4().to_string();
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            batch_id,
            messages,
            created_at,
            compression: None,
            checksum: None,
        }
    }

    /// Create batch with compression
    pub fn with_compression(mut self, compression: String) -> Self {
        self.compression = Some(compression);
        self
    }

    /// Add checksum to batch
    pub fn with_checksum(mut self) -> Self {
        self.checksum = Some(self.calculate_checksum());
        self
    }

    /// Calculate batch checksum
    fn calculate_checksum(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        self.batch_id.hash(&mut hasher);
        self.created_at.hash(&mut hasher);

        for msg in &self.messages {
            msg.message_id.hash(&mut hasher);
            msg.source_node.hash(&mut hasher);
            msg.timestamp.hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Verify batch integrity
    pub fn verify_checksum(&self) -> bool {
        if let Some(expected) = self.checksum {
            expected == self.calculate_checksum()
        } else {
            true // No checksum to verify
        }
    }

    /// Get batch size
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get total estimated size of batch
    pub fn estimated_size(&self) -> usize {
        let base_size = std::mem::size_of::<MessageBatch>();
        let batch_id_size = self.batch_id.len();
        let messages_size: usize = self.messages.iter().map(|m| m.estimated_size()).sum();
        let compression_size = self.compression.as_ref().map(|c| c.len()).unwrap_or(0);

        base_size + batch_id_size + messages_size + compression_size
    }

    /// Split batch into smaller batches
    pub fn split(self, max_size: usize) -> Vec<MessageBatch> {
        if self.messages.len() <= max_size {
            return vec![self];
        }

        let mut batches = Vec::new();
        let chunks: Vec<_> = self.messages.chunks(max_size).collect();

        for chunk in chunks {
            let batch = MessageBatch::new(chunk.to_vec());
            batches.push(batch);
        }

        batches
    }
}

/// Message priority levels for easier use
pub mod priority {
    pub const CRITICAL: u8 = 255;
    pub const HIGH: u8 = 192;
    pub const NORMAL: u8 = 128;
    pub const LOW: u8 = 64;
    pub const BACKGROUND: u8 = 0;
}

/// Common TTL values for convenience
pub mod ttl {
    pub const IMMEDIATE: u64 = 60; // 1 minute
    pub const SHORT: u64 = 300; // 5 minutes
    pub const MEDIUM: u64 = 3600; // 1 hour
    pub const LONG: u64 = 86400; // 24 hours
    pub const PERSISTENT: u64 = 604800; // 1 week
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::new(
            MessageType::Data("test data".to_string()),
            "node1".to_string(),
        );

        assert_eq!(msg.source, "node1");
        assert!(!msg.is_expired());
        assert_eq!(msg.priority, 128);
    }

    #[test]
    fn test_message_with_ttl() {
        let msg =
            Message::new(MessageType::Data("test".to_string()), "node1".to_string()).with_ttl(60);

        assert_eq!(msg.ttl, 60);
        assert!(msg.remaining_ttl() <= 60);
    }

    #[test]
    fn test_message_envelope() {
        use crate::gossip::GossipMessage;

        let msg = Message::new(MessageType::Data("test".to_string()), "node1".to_string());

        let envelope =
            MessageEnvelope::new("node1".to_string(), GossipMessage::Data(Box::new(msg)));

        assert_eq!(envelope.source_node, "node1");
        assert_eq!(envelope.hop_count, 0);
        assert!(!envelope.is_expired());
    }

    #[test]
    fn test_message_batch() {
        use crate::gossip::GossipMessage;

        let msg1 = Message::new(MessageType::Data("test1".to_string()), "node1".to_string());

        let msg2 = Message::new(MessageType::Data("test2".to_string()), "node1".to_string());

        let env1 = MessageEnvelope::new("node1".to_string(), GossipMessage::Data(Box::new(msg1)));
        let env2 = MessageEnvelope::new("node1".to_string(), GossipMessage::Data(Box::new(msg2)));

        let batch = MessageBatch::new(vec![env1, env2]).with_checksum();

        assert_eq!(batch.len(), 2);
        assert!(batch.verify_checksum());
    }

    #[test]
    fn test_message_response() {
        let original = Message::new(
            MessageType::Command {
                action: "ping".to_string(),
                params: HashMap::new(),
            },
            "node1".to_string(),
        )
        .with_correlation_id("test-correlation".to_string());

        let response = original.create_response(
            MessageType::Status {
                component: "pong".to_string(),
                state: "ok".to_string(),
            },
            "node2".to_string(),
        );

        assert_eq!(
            response.correlation_id,
            Some("test-correlation".to_string())
        );
        assert_eq!(response.priority, original.priority);
    }

    #[test]
    fn test_priority_constants() {
        assert_eq!(priority::CRITICAL, 255);
        assert_eq!(priority::NORMAL, 128);
        assert_eq!(priority::BACKGROUND, 0);
    }

    #[test]
    fn test_ttl_constants() {
        assert_eq!(ttl::IMMEDIATE, 60);
        assert_eq!(ttl::MEDIUM, 3600);
        assert_eq!(ttl::PERSISTENT, 604800);
    }

    #[test]
    fn test_message_signing() {
        use rand::RngCore;

        // Generate a signing key
        let mut csprng = rand::rngs::OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);

        // Create and sign a message
        let msg = Message::new(
            MessageType::Data("sensitive threat intel".to_string()),
            "threat-detector-1".to_string(),
        )
        .sign_with_ed25519(&signing_key);

        // Verify the message is signed
        assert!(msg.is_signed());
        assert!(msg.signer_public_key().is_some());

        // Verify the signature
        assert!(msg.verify_signature().unwrap());
    }

    #[test]
    fn test_message_signature_verification_fails_on_tampered_content() {
        use rand::RngCore;

        // Generate a signing key
        let mut csprng = rand::rngs::OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);

        // Create and sign a message
        let mut msg = Message::new(
            MessageType::Data("original content".to_string()),
            "node1".to_string(),
        )
        .sign_with_ed25519(&signing_key);

        // Tamper with the message content after signing
        msg.message_type = MessageType::Data("tampered content".to_string());

        // Verification should fail
        assert!(msg.verify_signature().is_err());
    }

    #[test]
    fn test_unsigned_message_verification() {
        let msg = Message::new(
            MessageType::Data("unsigned message".to_string()),
            "node1".to_string(),
        );

        // Unsigned message should return false (not error)
        assert!(!msg.verify_signature().unwrap());
        assert!(!msg.is_signed());
        assert!(msg.signer_public_key().is_none());
    }
}
