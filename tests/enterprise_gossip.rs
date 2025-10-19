use somasync::{EnterpriseGossipConfig, GossipConfig, GossipProtocol};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_basic_gossip_protocol() {
    let (message_tx, _message_rx) = mpsc::unbounded_channel();

    // Create basic gossip protocol
    let config = GossipConfig::default();
    let (protocol, _outbound_rx) =
        GossipProtocol::new("basic-node".to_string(), config, message_tx);

    // Should not have enterprise features
    assert!(!protocol.has_enterprise_features());
    assert!(protocol.get_network_stats().await.is_none());
}

#[tokio::test]
async fn test_enterprise_gossip_protocol() {
    let (message_tx, _message_rx) = mpsc::unbounded_channel();

    // Create enterprise gossip protocol with threat intel optimizations
    let enterprise_config = EnterpriseGossipConfig::for_threat_intel();
    let (protocol, _outbound_rx) = GossipProtocol::new_enterprise(
        "enterprise-node".to_string(),
        enterprise_config,
        message_tx,
    );

    // Should have enterprise features
    assert!(protocol.has_enterprise_features());
    assert!(protocol.get_network_stats().await.is_some());
}

#[tokio::test]
async fn test_builder_pattern_customization() {
    let (message_tx, _message_rx) = mpsc::unbounded_channel();

    // Build custom enterprise configuration
    let custom_config = EnterpriseGossipConfig::builder()
        .with_adaptive_fanout(20)
        .with_priority_routing()
        .with_bandwidth_limit(10_000_000) // 10MB/s
        .with_smart_routing()
        .with_epidemic_tuning(0.995);

    let (protocol, _outbound_rx) = GossipProtocol::new_enterprise(
        "custom-enterprise-node".to_string(),
        custom_config,
        message_tx,
    );

    assert!(protocol.has_enterprise_features());
    let stats = protocol.get_network_stats().await;
    assert!(stats.is_some());
}

#[tokio::test]
async fn test_adaptive_fanout_calculation() {
    // Test small network
    let config = EnterpriseGossipConfig::builder().with_adaptive_fanout(15);
    assert_eq!(config.calculate_adaptive_fanout(10), 3); // Uses base fanout for small networks

    // Test medium network
    assert_eq!(config.calculate_adaptive_fanout(100), 5); // log(100) ≈ 4.6, rounded up to 5

    // Test large network
    assert_eq!(config.calculate_adaptive_fanout(10000), 10); // log(10000) ≈ 9.2, rounded up to 10

    // Test very large network (should cap at max_fanout)
    let very_large_fanout = config.calculate_adaptive_fanout(1_000_000);
    assert!(very_large_fanout <= 15); // Should be capped at max_fanout
    assert!(very_large_fanout >= 10); // Should be at least log-based calculation
}

#[tokio::test]
async fn test_threat_intel_preset() {
    let threat_config = EnterpriseGossipConfig::for_threat_intel();

    // Verify threat intel optimizations
    assert!(threat_config.adaptive_fanout);
    assert!(threat_config.priority_routing);
    assert!(threat_config.smart_routing);
    assert!(threat_config.epidemic_tuning);
    assert_eq!(threat_config.max_fanout, 15);
    assert_eq!(threat_config.batch_size, 25);
    assert_eq!(threat_config.target_delivery_probability, 0.999);
    assert_eq!(threat_config.max_bandwidth_per_sec, Some(5_000_000));
}

#[tokio::test]
async fn test_enterprise_config_with_custom_base() {
    // Create custom base configuration
    let base_config = GossipConfig {
        fanout: 7,
        max_stored_messages: 100_000,
        ..Default::default()
    };

    // Build enterprise config with custom base
    let enterprise_config = EnterpriseGossipConfig::with_base(base_config)
        .with_priority_routing()
        .with_smart_routing();

    assert_eq!(enterprise_config.base.fanout, 7);
    assert_eq!(enterprise_config.base.max_stored_messages, 100_000);
    assert!(enterprise_config.priority_routing);
    assert!(enterprise_config.smart_routing);
    assert!(!enterprise_config.adaptive_fanout); // Should be false by default
}

#[tokio::test]
async fn test_adaptive_interval_calculation() {
    let config = EnterpriseGossipConfig::builder().with_epidemic_tuning(0.99);

    // Test low latency, small network
    let interval = config.calculate_adaptive_interval(10, 100);
    assert!(interval.as_millis() >= 30_000); // Should be >= base interval

    // Test high latency, large network
    let interval = config.calculate_adaptive_interval(500, 10000);
    assert!(interval.as_millis() >= 30_000); // Should be at least base interval

    // Test without epidemic tuning
    let basic_config = EnterpriseGossipConfig::builder(); // epidemic_tuning = false
    let interval = basic_config.calculate_adaptive_interval(500, 10000);
    assert_eq!(interval.as_millis(), 30_000); // Should return base interval
}
