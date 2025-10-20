use somasync::{development_config, init_logging, CorrelationId, SynapseNodeBuilder};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with development configuration
    let log_config = development_config();
    init_logging(&log_config)?;

    info!("Starting SomaSync logging demonstration");

    // Create correlation ID for this session
    let correlation_id = CorrelationId::new();

    info!(
        correlation_id = %correlation_id.short(),
        "Session started with correlation ID"
    );

    // Create and start first node
    let (mut node1, _message_rx1, _event_rx1) = SynapseNodeBuilder::new()
        .with_node_id("demo-node-1".to_string())
        .build()?;

    // Create and start second node
    let (mut node2, _message_rx2, _event_rx2) = SynapseNodeBuilder::new()
        .with_node_id("demo-node-2".to_string())
        .build()?;

    info!(
        correlation_id = %correlation_id.short(),
        "Created two demo nodes for logging demonstration"
    );

    // Start nodes (this will generate structured logs)
    tokio::spawn(async move {
        if let Err(e) = node1.start().await {
            tracing::error!("Node 1 failed: {}", e);
        }
    });

    tokio::spawn(async move {
        if let Err(e) = node2.start().await {
            tracing::error!("Node 2 failed: {}", e);
        }
    });

    // Wait a bit for nodes to initialize
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!(
        correlation_id = %correlation_id.short(),
        "Logging demonstration completed - check logs for structured output"
    );

    Ok(())
}
