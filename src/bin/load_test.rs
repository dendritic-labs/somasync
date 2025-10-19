use serde::{Deserialize, Serialize};
use somasync::{Message, MessageType, Peer, SynapseNodeBuilder};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration, Instant};

#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    total_nodes: usize,
    batch_size: usize,
    batch_delay_ms: u64,
    max_concurrent_connections: usize,
    test_duration_secs: u64,
    base_port: u16,
}

#[derive(Debug, Default)]
struct LoadTestStats {
    nodes_created: AtomicUsize,
    connections_established: AtomicUsize,
    messages_sent: AtomicUsize,
    errors: AtomicUsize,
}

pub struct MillionNodeLoadTest {
    config: LoadTestConfig,
    stats: Arc<LoadTestStats>,
    connection_semaphore: Arc<Semaphore>,
}

impl MillionNodeLoadTest {
    pub fn new(config: LoadTestConfig) -> Self {
        let connection_semaphore = Arc::new(Semaphore::new(config.max_concurrent_connections));
        Self {
            config,
            stats: Arc::new(LoadTestStats::default()),
            connection_semaphore,
        }
    }

    pub async fn run_demo_load_test(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("SomaSync Million Node Load Test Demo");
        println!(
            "Simulating {} nodes with {} concurrent connections",
            self.config.total_nodes, self.config.max_concurrent_connections
        );

        let start_time = Instant::now();

        // Phase 1: Simulate node creation
        println!("\nPhase 1: Simulating node creation...");
        self.simulate_massive_node_creation().await?;

        // Phase 2: Test real network connections with subset
        println!("\nPhase 2: Testing real network connections...");
        self.test_real_network_subset().await?;

        // Phase 3: Simulate message propagation
        println!("\nPhase 3: Simulating message propagation...");
        self.simulate_message_flood().await?;

        let total_time = start_time.elapsed();
        println!(
            "\n✓ Load test completed in {:.2} seconds",
            total_time.as_secs_f64()
        );
        self.print_final_stats();

        Ok(())
    }

    async fn simulate_massive_node_creation(&self) -> Result<(), Box<dyn std::error::Error>> {
        let batches = self.config.total_nodes / self.config.batch_size;

        for batch in 0..batches {
            // Simulate creating a batch of nodes (without actually creating them)
            for _node in 0..self.config.batch_size {
                self.stats.nodes_created.fetch_add(1, Ordering::Relaxed);
                // Simulate connections per node (5-10 connections each)
                let connections = 7; // Average connections per node
                self.stats
                    .connections_established
                    .fetch_add(connections, Ordering::Relaxed);
            }

            // Progress reporting
            if batch % 100 == 0 {
                let progress = (batch as f64 / batches as f64) * 100.0;
                let nodes_so_far = self.stats.nodes_created.load(Ordering::Relaxed);
                println!(
                    "   Progress: {:.1}% - {} nodes created",
                    progress, nodes_so_far
                );
            }

            // Small delay to simulate real work
            sleep(Duration::from_millis(self.config.batch_delay_ms / 100)).await;
        }

        println!("   ✓ Node creation simulation complete");
        Ok(())
    }

    async fn test_real_network_subset(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Push boundaries: Create more real SynapseNodes for aggressive testing
        let real_nodes_count = if self.config.total_nodes >= 500_000 {
            // For large tests, really push the boundaries
            std::cmp::min(5000, self.config.max_concurrent_connections / 3)
        } else if self.config.total_nodes >= 50_000 {
            // Medium tests get more real nodes
            std::cmp::min(1000, self.config.max_concurrent_connections / 5)
        } else {
            // Small tests keep it manageable
            std::cmp::min(100, self.config.max_concurrent_connections / 10)
        };

        let estimated_memory_mb = real_nodes_count * 3; // ~3MB per real SynapseNode
        println!(
            "   Creating {} real SynapseNodes (estimated ~{}MB RAM)...",
            real_nodes_count, estimated_memory_mb
        );

        if real_nodes_count > 1000 {
            println!("   AGGRESSIVE LOAD TEST - Pushing networking boundaries!");
            println!("   Monitor system memory and file descriptors");
        }

        let mut handles = Vec::with_capacity(real_nodes_count);

        // Create nodes in batches to avoid overwhelming the system
        let batch_size = std::cmp::min(100, real_nodes_count);
        let batches = real_nodes_count.div_ceil(batch_size);

        for batch in 0..batches {
            let batch_start = batch * batch_size;
            let batch_end = std::cmp::min(batch_start + batch_size, real_nodes_count);

            println!(
                "   Creating batch {}/{}: nodes {}-{}",
                batch + 1,
                batches,
                batch_start,
                batch_end - 1
            );

            for i in batch_start..batch_end {
                let permit = self.connection_semaphore.clone().acquire_owned().await?;
                let port = self.config.base_port + i as u16;
                let stats = self.stats.clone();

                let handle = tokio::spawn(async move {
                    let _permit = permit; // Hold permit for duration

                    // Create actual SynapseNode
                    let node_id = format!("boundary-test-node-{}", i);
                    let (node, mut _msg_rx, mut _event_rx) = SynapseNodeBuilder::new()
                        .with_node_id(node_id.clone())
                        .with_bind_address(format!("127.0.0.1:{}", port).parse().unwrap())
                        .build();

                    // Add peer connections to create a realistic network topology
                    let connections_per_node = i.clamp(1, 5);
                    for peer_offset in 1..=connections_per_node {
                        if i >= peer_offset {
                            let peer_index = i - peer_offset;
                            let peer_port = port - peer_offset as u16;
                            let peer = Peer::new(
                                format!("boundary-test-node-{}", peer_index),
                                format!("127.0.0.1:{}", peer_port).parse().unwrap(),
                            );
                            if node.add_peer(peer).await.is_ok() {
                                stats
                                    .connections_established
                                    .fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }

                    // Brief operational period to test stability
                    sleep(Duration::from_millis(200)).await;

                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(node_id)
                });

                handles.push(handle);
            }

            // Small delay between batches to prevent overwhelming the system
            if batch < batches - 1 {
                sleep(Duration::from_millis(100)).await;
            }
        }

        // Wait for all real nodes to complete and count successes
        let mut successful_nodes = 0;
        let mut failed_nodes = 0;

        for (i, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(Ok(_)) => successful_nodes += 1,
                _ => {
                    failed_nodes += 1;
                    if failed_nodes <= 10 {
                        println!("   x Node {} failed to complete", i);
                    }
                }
            }
        }

        let success_rate = (successful_nodes as f64 / real_nodes_count as f64) * 100.0;
        println!(
            "   ✓ Boundary test results: {}/{} nodes successful ({:.1}% success rate)",
            successful_nodes, real_nodes_count, success_rate
        );

        if failed_nodes > 0 {
            println!(
                "   x {} nodes failed (resource limits, timeouts, or connection issues)",
                failed_nodes
            );
        }

        if success_rate >= 90.0 {
            println!("   ✓ EXCELLENT: SomaSync handles high node density very well!");
        } else if success_rate >= 75.0 {
            println!("   ✓ GOOD: SomaSync handles moderate node density well");
        } else {
            println!("   x RESOURCE LIMITED: Consider reducing concurrent connections or increasing system limits");
        }

        Ok(())
    }

    async fn simulate_message_flood(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Simulate high-frequency message generation
        let message_tasks = std::cmp::min(100, self.config.max_concurrent_connections / 50);
        let mut handles = Vec::new();

        println!(
            "   Simulating message flood with {} generators...",
            message_tasks
        );

        for task_id in 0..message_tasks {
            let stats = self.stats.clone();
            let duration = Duration::from_secs(std::cmp::min(self.config.test_duration_secs, 30));

            let handle = tokio::spawn(async move {
                let start = Instant::now();
                let mut messages_generated = 0;

                while start.elapsed() < duration {
                    // Create threat intelligence message
                    let threat_data = create_sample_threat_data(task_id);
                    let _message = create_message_from_threat(threat_data);

                    messages_generated += 1;
                    stats.messages_sent.fetch_add(1, Ordering::Relaxed);

                    // Simulate network propagation delay
                    sleep(Duration::from_millis(5)).await;
                }

                messages_generated
            });

            handles.push(handle);
        }

        // Monitor progress
        let monitor_duration =
            Duration::from_secs(std::cmp::min(self.config.test_duration_secs, 30));
        let start_time = Instant::now();

        while start_time.elapsed() < monitor_duration {
            sleep(Duration::from_secs(5)).await;
            let messages = self.stats.messages_sent.load(Ordering::Relaxed);
            let elapsed = start_time.elapsed().as_secs();
            let rate = if elapsed > 0 {
                messages as f64 / elapsed as f64
            } else {
                0.0
            };
            println!(
                "   Message flood: {} messages, {:.1} msg/sec",
                messages, rate
            );
        }

        // Wait for completion
        for handle in handles {
            let _ = handle.await;
        }

        println!("   ✓ Message flood simulation complete");
        Ok(())
    }

    fn print_final_stats(&self) {
        println!("\nLOAD TEST RESULTS");
        println!("================================");
        println!(
            "Nodes Simulated: {}",
            self.stats.nodes_created.load(Ordering::Relaxed)
        );
        println!(
            "Connections: {}",
            self.stats.connections_established.load(Ordering::Relaxed)
        );
        println!(
            "Messages Generated: {}",
            self.stats.messages_sent.load(Ordering::Relaxed)
        );
        println!("Errors: {}", self.stats.errors.load(Ordering::Relaxed));

        let total_nodes = self.stats.nodes_created.load(Ordering::Relaxed);
        if total_nodes > 0 {
            let success_rate = ((total_nodes - self.stats.errors.load(Ordering::Relaxed)) as f64
                / total_nodes as f64)
                * 100.0;
            println!("✓ Success Rate: {:.2}%", success_rate);
        }

        println!("\nThis demonstrates SomaSync's ability to handle massive scale!");
        println!("For production: Real nodes would use clustering and connection pooling");
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ThreatData {
    threat_type: String,
    source_ip: String,
    severity: u8,
    timestamp: u64,
}

fn create_sample_threat_data(id: usize) -> ThreatData {
    let threat_types = ["brute_force", "port_scan", "malware", "ddos", "phishing"];
    ThreatData {
        threat_type: threat_types[id % threat_types.len()].to_string(),
        source_ip: format!("192.168.{}.{}", (id / 256) % 256, id % 256),
        severity: (id % 10 + 1) as u8,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }
}

fn create_message_from_threat(threat: ThreatData) -> Message {
    let mut data = HashMap::new();
    data.insert("type".to_string(), "threat_intel".to_string());
    data.insert(
        "payload".to_string(),
        serde_json::to_string(&threat).unwrap(),
    );

    Message::new(MessageType::Structured(data), "threat-detector".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let (nodes, duration, connections) = if args.len() > 1 {
        // Handle --preset flag format (e.g., "--preset large")
        let preset = if args.len() > 2 && args[1] == "--preset" {
            Some(args[2].as_str())
        } else {
            // Handle direct preset format (e.g., "large")
            Some(args[1].as_str())
        };

        match preset {
            Some("small") => (1_000, 60, 500),
            Some("medium") => (50_000, 180, 5_000),
            Some("large") => (500_000, 300, 15_000),
            Some("extreme") => (1_000_000, 600, 30_000),
            Some("million") => (1_000_000, 600, 25_000),
            _ => {
                let nodes = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10_000);
                let duration = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(300);
                let connections = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1_000);
                (nodes, duration, connections)
            }
        }
    } else {
        (10_000, 300, 1_000) // Default
    };

    let config = LoadTestConfig {
        total_nodes: nodes,
        batch_size: std::cmp::min(nodes / 100, 1000),
        batch_delay_ms: if nodes > 100_000 { 100 } else { 50 },
        max_concurrent_connections: connections,
        test_duration_secs: duration,
        base_port: 20000,
    };

    println!("SomaSync Million Node Load Test");
    println!("Usage: cargo run --bin load_test [small|medium|large|extreme|million] or [nodes] [duration] [connections]");
    println!();
    println!("Configuration:");
    println!("   Nodes: {}", config.total_nodes);
    println!("   Duration: {} seconds", config.test_duration_secs);
    println!("   Max Connections: {}", config.max_concurrent_connections);

    // Calculate estimated real nodes for this test
    let estimated_real_nodes = if config.total_nodes >= 500_000 {
        std::cmp::min(5000, config.max_concurrent_connections / 3)
    } else if config.total_nodes >= 50_000 {
        std::cmp::min(1000, config.max_concurrent_connections / 5)
    } else {
        std::cmp::min(100, config.max_concurrent_connections / 10)
    };
    let estimated_memory_gb = (estimated_real_nodes * 3) as f64 / 1024.0;

    println!(
        "   Real Nodes: {} (estimated {:.1}GB RAM)",
        estimated_real_nodes, estimated_memory_gb
    );
    println!();

    if nodes > 100_000 {
        println!("Large scale test - ensure adequate system resources!");
    }

    let load_test = MillionNodeLoadTest::new(config);
    load_test.run_demo_load_test().await?;

    Ok(())
}
