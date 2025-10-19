//! # Enterprise Threat Intelligence Sharing Example
//!
//! This example demonstrates how to use SomaSync's enterprise-scale gossip optimizations
//! for distributed threat intelligence sharing across security operations.
//!
//! ## Features Demonstrated
//! - Message signing for threat intel authenticity
//! - Enterprise gossip configuration with adaptive scaling
//! - Priority routing for critical threats
//! - Smart routing based on peer reputation
//! - Bandwidth throttling for enterprise networks
//! - Network statistics monitoring

use ed25519_dalek::Keypair;
use rand::rngs::OsRng as RandOsRng;
use somasync::{EnterpriseGossipConfig, GossipProtocol, Message, MessageType, Peer};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

/// A security operations center node for threat sharing
struct SecurityNode {
    name: String,
    gossip_protocol: GossipProtocol,
    signing_key: Keypair,
    threat_message_rx: mpsc::UnboundedReceiver<Message>,
    _outbound_rx: mpsc::UnboundedReceiver<(std::net::SocketAddr, somasync::MessageEnvelope)>,
}

impl SecurityNode {
    /// Create a new security node optimized for threat intel sharing
    async fn new_enterprise_node(name: String, node_id: String, _bind_port: u16) -> Self {
        let (threat_tx, threat_rx) = mpsc::unbounded_channel();

        // Generate signing key for threat intel authenticity
        let signing_key = Keypair::generate(&mut RandOsRng);

        // Configure enterprise gossip for threat intelligence
        let enterprise_config = EnterpriseGossipConfig::for_threat_intel();

        println!("🔒 Creating enterprise security node '{}' with:", name);
        println!(
            "   • Adaptive fanout: {} (max: {})",
            enterprise_config.base.fanout, enterprise_config.max_fanout
        );
        println!(
            "   • Priority routing: {}",
            enterprise_config.priority_routing
        );
        println!("   • Smart routing: {}", enterprise_config.smart_routing);
        println!(
            "   • Bandwidth limit: {} MB/s",
            enterprise_config.max_bandwidth_per_sec.unwrap_or(0) / 1_000_000
        );
        println!(
            "   • Target delivery: {:.1}%",
            enterprise_config.target_delivery_probability * 100.0
        );

        // Create enterprise gossip protocol
        let (gossip_protocol, outbound_rx) =
            GossipProtocol::new_enterprise(node_id, enterprise_config, threat_tx);

        Self {
            name,
            gossip_protocol,
            signing_key,
            threat_message_rx: threat_rx,
            _outbound_rx: outbound_rx,
        }
    }

    /// Create a basic security node (for comparison)
    async fn new_basic_node(name: String, node_id: String, _bind_port: u16) -> Self {
        let (threat_tx, threat_rx) = mpsc::unbounded_channel();

        let signing_key = Keypair::generate(&mut RandOsRng);

        // Use basic gossip configuration
        let basic_config = somasync::GossipConfig::default();

        println!(
            "📡 Creating basic security node '{}' with standard gossip",
            name
        );

        let (gossip_protocol, outbound_rx) = GossipProtocol::new(node_id, basic_config, threat_tx);

        Self {
            name,
            gossip_protocol,
            signing_key,
            threat_message_rx: threat_rx,
            _outbound_rx: outbound_rx,
        }
    }

    /// Share a signed threat intelligence alert
    async fn share_threat_intel(&self, threat_type: &str, details: HashMap<String, String>) {
        // Create threat intel message
        let threat_message = Message::new(
            MessageType::Alert {
                level: "CRITICAL".to_string(),
                message: format!("Threat detected: {}", threat_type),
                details: Some(details),
            },
            self.gossip_protocol.node_id().to_string(),
        )
        .with_priority(255) // Critical priority
        .with_header("threat_type".to_string(), threat_type.to_string())
        .with_header("source_soc".to_string(), self.name.clone())
        .sign_with_ed25519(&self.signing_key); // Sign for authenticity

        println!(
            "🚨 {} sharing signed threat intel: {}",
            self.name, threat_type
        );

        if let Err(e) = self.gossip_protocol.gossip_message(threat_message).await {
            eprintln!("❌ Failed to share threat intel: {}", e);
        }
    }

    /// Connect to another security node
    async fn connect_to_peer(&self, peer_name: &str, peer_id: &str, peer_addr: &str) {
        let peer = Peer::new(peer_id.to_string(), peer_addr.parse().unwrap());

        if let Err(e) = self.gossip_protocol.add_peer(peer).await {
            eprintln!("❌ Failed to connect to {}: {}", peer_name, e);
        } else {
            println!("🔗 {} connected to {}", self.name, peer_name);
        }
    }

    /// Process incoming threat intelligence
    async fn process_incoming_threats(&mut self) {
        while let Some(threat_message) = self.threat_message_rx.recv().await {
            // Verify message signature for authenticity
            match threat_message.verify_signature() {
                Ok(true) => {
                    if let Some(soc_name) = threat_message.headers.get("source_soc") {
                        if let Some(threat_type) = threat_message.headers.get("threat_type") {
                            println!(
                                "✅ {} received verified threat intel from {}: {}",
                                self.name, soc_name, threat_type
                            );
                        }
                    }
                }
                Ok(false) => {
                    println!(
                        "⚠️  {} received unsigned threat intel (skipping verification)",
                        self.name
                    );
                }
                Err(e) => {
                    println!(
                        "❌ {} received threat intel with invalid signature: {}",
                        self.name, e
                    );
                }
            }
        }
    }

    /// Get network performance statistics (enterprise feature)
    async fn show_network_stats(&self) {
        if let Some(stats) = self.gossip_protocol.get_network_stats().await {
            println!("📊 {} Network Statistics:", self.name);
            println!("   • Messages sent: {}", stats.messages_sent);
            println!("   • Messages received: {}", stats.messages_received);
            println!("   • Bytes sent: {} KB", stats.bytes_sent / 1024);
            println!("   • Active peers: {}", stats.active_peers);
            println!("   • Avg latency: {} ms", stats.avg_latency_ms);
            println!("   • Delivery rate: {:.1}%", stats.delivery_rate * 100.0);
        } else {
            println!(
                "📊 {}: Network statistics not available (basic node)",
                self.name
            );
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🛡️  SomaSync Enterprise Threat Intelligence Sharing Demo");
    println!("=====================================================\n");

    // Create security operations center nodes
    println!("Setting up security operations centers...\n");

    let enterprise_soc = SecurityNode::new_enterprise_node(
        "Enterprise-SOC-Alpha".to_string(),
        "ent-soc-001".to_string(),
        8080,
    )
    .await;

    let enterprise_soc_beta = SecurityNode::new_enterprise_node(
        "Enterprise-SOC-Beta".to_string(),
        "ent-soc-002".to_string(),
        8081,
    )
    .await;

    let basic_soc = SecurityNode::new_basic_node(
        "Basic-SOC-Gamma".to_string(),
        "basic-gamma".to_string(),
        8003,
    )
    .await;

    println!();

    // Connect nodes to form threat sharing network
    println!("Establishing threat sharing network...\n");

    enterprise_soc
        .connect_to_peer("Enterprise-SOC-Beta", "enterprise-beta", "127.0.0.1:8002")
        .await;
    enterprise_soc
        .connect_to_peer("Basic-SOC-Gamma", "basic-gamma", "127.0.0.1:8003")
        .await;
    enterprise_soc_beta
        .connect_to_peer("Basic-SOC-Gamma", "basic-gamma", "127.0.0.1:8003")
        .await;

    println!();

    // Note: In a real application, you would spawn background tasks for processing
    // For this demo, we'll process threats synchronously

    // Simulate threat intelligence sharing
    sleep(Duration::from_millis(100)).await;

    println!("Simulating threat intelligence sharing...\n");

    // Enterprise SOC Alpha detects APT campaign
    let mut apt_details = HashMap::new();
    apt_details.insert("campaign".to_string(), "APT-X-2025".to_string());
    apt_details.insert(
        "tactics".to_string(),
        "spear_phishing,lateral_movement".to_string(),
    );
    apt_details.insert(
        "iocs".to_string(),
        "malicious-domain.evil,192.168.1.100".to_string(),
    );
    apt_details.insert("confidence".to_string(), "HIGH".to_string());

    enterprise_soc
        .share_threat_intel("Advanced Persistent Threat", apt_details)
        .await;

    sleep(Duration::from_millis(200)).await;

    // Enterprise SOC Beta detects ransomware
    let mut ransomware_details = HashMap::new();
    ransomware_details.insert("family".to_string(), "CryptoLocker-V5".to_string());
    ransomware_details.insert("encryption".to_string(), "AES-256".to_string());
    ransomware_details.insert(
        "ransom_note".to_string(),
        "YOUR_FILES_ENCRYPTED.txt".to_string(),
    );
    ransomware_details.insert(
        "payment_address".to_string(),
        "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string(),
    );

    enterprise_soc_beta
        .share_threat_intel("Ransomware Attack", ransomware_details)
        .await;

    sleep(Duration::from_millis(200)).await;

    // Basic SOC detects DDoS
    let mut ddos_details = HashMap::new();
    ddos_details.insert("type".to_string(), "HTTP_FLOOD".to_string());
    ddos_details.insert("volume".to_string(), "50Gbps".to_string());
    ddos_details.insert("source_countries".to_string(), "multiple".to_string());

    basic_soc
        .share_threat_intel("DDoS Attack", ddos_details)
        .await;

    // Allow time for threat propagation
    sleep(Duration::from_millis(500)).await;

    println!("\nNetwork Performance Statistics:");
    println!("==============================\n");

    // Show network statistics (enterprise feature)
    enterprise_soc.show_network_stats().await;
    println!();
    enterprise_soc_beta.show_network_stats().await;
    println!();
    basic_soc.show_network_stats().await;

    println!("\n🎯 Demo completed! Enterprise nodes provide:");
    println!("   • Message signing for threat authenticity");
    println!("   • Adaptive fanout for large-scale networks");
    println!("   • Priority routing for critical threats");
    println!("   • Smart routing based on peer reputation");
    println!("   • Bandwidth throttling for enterprise networks");
    println!("   • Comprehensive network monitoring");

    Ok(())
}
