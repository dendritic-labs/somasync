//! Comprehensive logging infrastructure for SomaSync
//!
//! This module provides structured logging capabilities including:
//! - Correlation ID tracking for request/session tracing
//! - Security event logging with metadata
//! - Performance metrics and timing instrumentation
//! - Configurable log levels and formatting
//! - Production-ready logging configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use tracing_subscriber::{
    filter::EnvFilter,
    fmt::{format::FmtSpan, time::UtcTime},
    layer::SubscriberExt,
    Layer, Registry,
};
use uuid::Uuid;

/// Correlation ID for tracking related operations across the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(Uuid);

impl CorrelationId {
    /// Generate a new random correlation ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a correlation ID from a UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the UUID value
    pub fn uuid(&self) -> Uuid {
        self.0
    }

    /// Get a short string representation for logging
    pub fn short(&self) -> String {
        self.0.to_string()[..8].to_string()
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Security event types for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEvent {
    /// Authentication attempt
    AuthAttempt {
        node_id: String,
        success: bool,
        method: String,
    },
    /// Message signature verification
    SignatureVerification {
        message_id: String,
        node_id: String,
        success: bool,
        algorithm: String,
    },
    /// Peer connection security validation
    PeerValidation {
        peer_id: String,
        success: bool,
        reason: Option<String>,
    },
    /// Rate limiting triggered
    RateLimitTriggered {
        peer_id: String,
        operation: String,
        limit: u32,
    },
    /// Suspicious activity detected
    SuspiciousActivity {
        peer_id: String,
        activity_type: String,
        details: HashMap<String, String>,
    },
    /// Configuration change
    ConfigChange {
        component: String,
        old_value: Option<String>,
        new_value: String,
        changed_by: String,
    },
}

/// Performance metrics for instrumentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Operation name
    pub operation: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Additional contextual data
    pub metadata: HashMap<String, String>,
}

/// Log configuration for different environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log level filter
    pub level: String,
    /// Enable structured JSON output
    pub json: bool,
    /// Enable ANSI colors in output
    pub colors: bool,
    /// Include file and line numbers
    pub include_location: bool,
    /// Include thread names
    pub include_thread: bool,
    /// Include span information
    pub include_spans: bool,
    /// Custom log targets and their levels
    pub targets: HashMap<String, String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json: false,
            colors: true,
            include_location: true,
            include_thread: true,
            include_spans: true,
            targets: HashMap::new(),
        }
    }
}

/// Initialize logging with the given configuration
pub fn init_logging(config: &LogConfig) -> Result<(), Box<dyn std::error::Error>> {
    let mut env_filter = EnvFilter::new(&config.level);

    // Add custom target filters
    for (target, level) in &config.targets {
        env_filter = env_filter.add_directive(format!("{}={}", target, level).parse()?);
    }

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_timer(UtcTime::rfc_3339())
        .with_target(true)
        .with_level(true)
        .with_thread_names(config.include_thread)
        .with_file(config.include_location)
        .with_line_number(config.include_location)
        .with_span_events(if config.include_spans {
            FmtSpan::NEW | FmtSpan::CLOSE
        } else {
            FmtSpan::NONE
        });

    let subscriber = Registry::default().with(env_filter).with(if config.json {
        fmt_layer.json().boxed()
    } else if config.colors {
        fmt_layer.boxed()
    } else {
        fmt_layer.with_ansi(false).boxed()
    });

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

/// Production logging configuration
pub fn production_config() -> LogConfig {
    LogConfig {
        level: "info".to_string(),
        json: true,
        colors: false,
        include_location: false,
        include_thread: true,
        include_spans: false,
        targets: [
            ("somasync".to_string(), "info".to_string()),
            ("somasync::security".to_string(), "warn".to_string()),
            ("somasync::performance".to_string(), "info".to_string()),
        ]
        .into_iter()
        .collect(),
    }
}

/// Development logging configuration
pub fn development_config() -> LogConfig {
    LogConfig {
        level: "debug".to_string(),
        json: false,
        colors: true,
        include_location: true,
        include_thread: true,
        include_spans: true,
        targets: [
            ("somasync".to_string(), "debug".to_string()),
            ("tokio".to_string(), "info".to_string()),
            ("hyper".to_string(), "info".to_string()),
        ]
        .into_iter()
        .collect(),
    }
}

/// Macro for creating instrumented spans with correlation ID
#[macro_export]
macro_rules! instrumented_span {
    ($level:expr, $name:expr, $correlation_id:expr) => {
        tracing::span!(
            $level,
            $name,
            correlation_id = %$correlation_id.short(),
            node_id = tracing::field::Empty,
            peer_id = tracing::field::Empty,
            operation = tracing::field::Empty
        )
    };
    ($level:expr, $name:expr, $correlation_id:expr, $($fields:tt)*) => {
        tracing::span!(
            $level,
            $name,
            correlation_id = %$correlation_id.short(),
            $($fields)*
        )
    };
}

/// Macro for logging security events
#[macro_export]
macro_rules! security_event {
    ($event:expr, $correlation_id:expr) => {
        tracing::warn!(
            target: "somasync::security",
            correlation_id = %$correlation_id.short(),
            event = ?$event,
            "Security event recorded"
        );
    };
    ($event:expr, $correlation_id:expr, $($fields:tt)*) => {
        tracing::warn!(
            target: "somasync::security",
            correlation_id = %$correlation_id.short(),
            event = ?$event,
            $($fields)*,
            "Security event recorded"
        );
    };
}

/// Macro for logging performance metrics
#[macro_export]
macro_rules! performance_metric {
    ($metrics:expr, $correlation_id:expr) => {
        tracing::info!(
            target: "somasync::performance",
            correlation_id = %$correlation_id.short(),
            operation = %$metrics.operation,
            duration_ms = $metrics.duration_ms,
            metadata = ?$metrics.metadata,
            "Performance metric recorded"
        );
    };
    ($metrics:expr, $correlation_id:expr, $($fields:tt)*) => {
        tracing::info!(
            target: "somasync::performance",
            correlation_id = %$correlation_id.short(),
            operation = %$metrics.operation,
            duration_ms = $metrics.duration_ms,
            metadata = ?$metrics.metadata,
            $($fields)*,
            "Performance metric recorded"
        );
    };
}

/// Helper struct for measuring operation performance
pub struct PerformanceTimer {
    operation: String,
    start: std::time::Instant,
    metadata: HashMap<String, String>,
}

impl PerformanceTimer {
    /// Start timing an operation
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            start: std::time::Instant::now(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the performance measurement
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Finish timing and return performance metrics
    pub fn finish(self) -> PerformanceMetrics {
        let duration = self.start.elapsed();
        PerformanceMetrics {
            operation: self.operation,
            duration_ms: duration.as_millis() as u64,
            metadata: self.metadata,
        }
    }
}

/// Trait extension for adding correlation context to spans
pub trait SpanExt {
    /// Record node ID in the current span
    fn record_node_id(&self, node_id: &str);
    /// Record peer ID in the current span
    fn record_peer_id(&self, peer_id: &str);
    /// Record operation name in the current span
    fn record_operation(&self, operation: &str);
}

impl SpanExt for tracing::Span {
    fn record_node_id(&self, node_id: &str) {
        self.record("node_id", node_id);
    }

    fn record_peer_id(&self, peer_id: &str) {
        self.record("peer_id", peer_id);
    }

    fn record_operation(&self, operation: &str) {
        self.record("operation", operation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correlation_id_generation() {
        let id1 = CorrelationId::new();
        let id2 = CorrelationId::new();
        assert_ne!(id1, id2);
        assert_eq!(id1.short().len(), 8);
    }

    #[test]
    fn test_performance_timer() {
        let timer = PerformanceTimer::new("test_operation")
            .with_metadata("peer_count", "5")
            .with_metadata("message_size", "1024");

        std::thread::sleep(std::time::Duration::from_millis(10));
        let metrics = timer.finish();

        assert_eq!(metrics.operation, "test_operation");
        assert!(metrics.duration_ms >= 10);
        assert_eq!(metrics.metadata.get("peer_count"), Some(&"5".to_string()));
        assert_eq!(
            metrics.metadata.get("message_size"),
            Some(&"1024".to_string())
        );
    }

    #[test]
    fn test_security_event_serialization() {
        let event = SecurityEvent::AuthAttempt {
            node_id: "node123".to_string(),
            success: true,
            method: "ed25519".to_string(),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: SecurityEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            SecurityEvent::AuthAttempt {
                node_id,
                success,
                method,
            } => {
                assert_eq!(node_id, "node123");
                assert!(success);
                assert_eq!(method, "ed25519");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_log_config_defaults() {
        let config = LogConfig::default();
        assert_eq!(config.level, "info");
        assert!(!config.json);
        assert!(config.colors);
        assert!(config.include_location);
    }
}
