//! Error types for the Synapse mesh networking library

use thiserror::Error;

/// Main error type for Synapse operations
#[derive(Error, Debug)]
pub enum SynapseError {
    /// Network-related errors
    #[error("Network error: {message} (endpoint: {endpoint})")]
    Network { message: String, endpoint: String },

    /// Peer discovery and connection errors
    #[error("Peer error: {message} (peer: {peer_id})")]
    Peer { message: String, peer_id: String },

    /// Message serialization/deserialization errors
    #[error("Message error: {message}")]
    Message { message: String },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Gossip protocol errors
    #[error("Gossip protocol error: {message}")]
    Gossip { message: String },

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Join handle errors
    #[error("Task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}

impl SynapseError {
    /// Create a network error
    pub fn network(message: impl Into<String>, endpoint: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
            endpoint: endpoint.into(),
        }
    }

    /// Create a peer error
    pub fn peer(message: impl Into<String>, peer_id: impl Into<String>) -> Self {
        Self::Peer {
            message: message.into(),
            peer_id: peer_id.into(),
        }
    }

    /// Create a message error
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message {
            message: message.into(),
        }
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a gossip protocol error
    pub fn gossip(message: impl Into<String>) -> Self {
        Self::Gossip {
            message: message.into(),
        }
    }
}
