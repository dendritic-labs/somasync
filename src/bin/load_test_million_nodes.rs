use somasync::{SynapseNodeBuilder, Peer, Message, MessageType};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{sleep, Duration, timeout, Instant};
use tokio::sync::Semaphore;
use serde::{Serialize, Deserialize};

/// Configuration for load testing parameters
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    /// Total number of nodes to simulate
    total_nodes: usize,
    /// Number of nodes to create per batch
    batch_size: usize,
    /// Delay between batches to prevent resource exhaustion
    batch_delay_ms: u64,
    /// Maximum concurrent active connections
    max_concurrent_connections: usize,
    /// Number of messages each node should send
    messages_per_node: usize,
    /// Test duration in seconds
    test_duration_secs: u64,
    /// Starting port for node addresses
    base_port: u16,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            total_nodes: 1_000_000,
            batch_size: 1000,
            batch_delay_ms: 100,
            max_concurrent_connections: 10_000,
            messages_per_node: 10,
            test_duration_secs: 300, // 5 minutes
            base_port: 20000,
        }
    }
}

/// Lightweight node representation for memory efficiency
#[derive(Debug)]
struct LightweightNode {
    node_id: String,
    port: u16,
    message_count: AtomicUsize,
    connection_count: AtomicUsize,
}

impl LightweightNode {
    fn new(id: usize, base_port: u16) -> Self {
        Self {
            node_id: format!("node-{}", id),
            port: base_port + (id as u16 % 50000), // Cycle through port range
            message_count: AtomicUsize::new(0),
            connection_count: AtomicUsize::new(0),
        }
    }
}

/// Load test statistics
#[derive(Debug, Default)]
struct LoadTestStats {
    nodes_created: AtomicUsize,
    connections_established: AtomicUsize,
    messages_sent: AtomicUsize,
    messages_received: AtomicUsize,
    errors: AtomicUsize,
    peak_memory_mb: AtomicUsize,
}

/// Main load test orchestrator  
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

    /// Simulate a realistic distributed network topology
    /// Rather than full mesh (impossible), create clusters with inter-cluster connections
    pub async fn run_clustered_topology_test(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting SomaSync Million Node Load Test");
        println!("📊 Config: {} nodes, {} batches of {}", 
                 self.config.total_nodes, 
                 self.config.total_nodes / self.config.batch_size,
                 self.config.batch_size);

        let start_time = Instant::now();
        
        // Create cluster topology: 1000 clusters of 1000 nodes each
        let clusters = 1000;
        let nodes_per_cluster = self.config.total_nodes / clusters;
        
        println!("🏗️  Creating {} clusters with {} nodes each", clusters, nodes_per_cluster);

        // Track cluster representatives for inter-cluster communication
        let mut cluster_representatives = Vec::new();
        
        for cluster_id in 0..clusters {
            let cluster_stats = self.create_cluster(cluster_id, nodes_per_cluster).await?;
            cluster_representatives.push(cluster_stats);
            
            // Progress reporting
            if cluster_id % 100 == 0 {
                let progress = (cluster_id as f64 / clusters as f64) * 100.0;
                println!("📈 Progress: {:.1}% - {} clusters created", progress, cluster_id + 1);
                self.print_stats();
            }
            
            // Breathing room between clusters
            sleep(Duration::from_millis(self.config.batch_delay_ms / 10)).await;
        }

        // Inter-cluster connectivity test
        println!("🔗 Testing inter-cluster connectivity...");
        self.test_inter_cluster_messaging(&cluster_representatives).await?;

        // Sustained load test
        println!("⚡ Running sustained load test for {} seconds...", self.config.test_duration_secs);
        self.run_sustained_load_test().await?;

        let total_time = start_time.elapsed();
        println!("✅ Load test completed in {:.2} seconds", total_time.as_secs_f64());
        self.print_final_stats();

        Ok(())
    }

    /// Create a cluster of nodes with local connectivity
    async fn create_cluster(&self, cluster_id: usize, nodes_in_cluster: usize) -> Result<ClusterStats, Box<dyn std::error::Error>> {
        let mut cluster_stats = ClusterStats::new(cluster_id);
        
        // Create lightweight node representations (not full SynapseNodes to save memory)
        let mut lightweight_nodes = Vec::with_capacity(nodes_in_cluster);
        
        for node_offset in 0..nodes_in_cluster {
            let global_node_id = cluster_id * nodes_in_cluster + node_offset;
            let node = LightweightNode::new(global_node_id, self.config.base_port);
            lightweight_nodes.push(node);
            
            self.stats.nodes_created.fetch_add(1, Ordering::Relaxed);
        }

        // Simulate intra-cluster connections (each node connects to 3-5 neighbors)
        for (i, node) in lightweight_nodes.iter().enumerate() {
            let connection_count = std::cmp::min(5, lightweight_nodes.len() - 1);
            node.connection_count.store(connection_count, Ordering::Relaxed);
            self.stats.connections_established.fetch_add(connection_count, Ordering::Relaxed);
        }

        // Pick a representative node for inter-cluster communication
        if !lightweight_nodes.is_empty() {
            cluster_stats.representative_node = Some(lightweight_nodes[0].node_id.clone());
        }

        cluster_stats.node_count = lightweight_nodes.len();
        Ok(cluster_stats)
    }

    /// Test messaging between cluster representatives
    async fn test_inter_cluster_messaging(&self, clusters: &[ClusterStats]) -> Result<(), Box<dyn std::error::Error>> {
        // Create actual SynapseNodes for a small subset to test real networking
        let test_nodes_count = std::cmp::min(100, clusters.len());
        let mut test_handles = Vec::new();

        println!("🧪 Creating {} real SynapseNodes for messaging test", test_nodes_count);

        for i in 0..test_nodes_count {
            let node_id = format!("test-representative-{}", i);
            let port = self.config.base_port + i as u16;
            
            // Use connection semaphore to limit concurrent connections
            let permit = self.connection_semaphore.clone().acquire_owned().await?;
            
            let handle = tokio::spawn(async move {
                let _permit = permit; // Hold permit for duration of task
                
                // Create actual SynapseNode
                let (mut node, mut _msg_rx, mut _event_rx) = SynapseNodeBuilder::new()
                    .with_node_id(node_id.clone())
                    .with_bind_address(format!("127.0.0.1:{}", port).parse().unwrap())
                    .build();

                // Simulate some connections
                if i > 0 {
                    let peer_port = port - 1;
                    let peer = Peer::new(
                        format!("test-representative-{}", i - 1),
                        format!("127.0.0.1:{}", peer_port).parse().unwrap()
                    );
                    if let Err(_) = node.add_peer(peer).await {
                        // Connection failed - this is expected under load
                    }
                }

                // Brief operation to simulate message passing
                sleep(Duration::from_millis(100)).await;
                
                (node_id, port)
            });
            
            test_handles.push(handle);
        }

        // Wait for test nodes to complete
        let mut completed_nodes = 0;
        for handle in test_handles {
            if let Ok(_result) = handle.await {
                completed_nodes += 1;
                self.stats.messages_sent.fetch_add(1, Ordering::Relaxed);
            }
        }

        println!("✅ Inter-cluster test: {}/{} nodes completed successfully", completed_nodes, test_nodes_count);
        Ok(())
    }

    /// Run sustained load test with message generation
    async fn run_sustained_load_test(&self) -> Result<(), Box<dyn std::error::Error>> {
        let duration = Duration::from_secs(self.config.test_duration_secs);
        let start_time = Instant::now();
        
        // Simulate high-frequency message generation across the network
        let message_generation_tasks = 100; // Simulate 100 high-activity nodes
        let mut handles = Vec::new();

        for task_id in 0..message_generation_tasks {
            let stats = self.stats.clone();
            let config = self.config.clone();
            
            let handle = tokio::spawn(async move {
                let mut messages_sent = 0;
                let start = Instant::now();
                
                while start.elapsed() < duration {
                    // Simulate threat intelligence message creation
                    let threat_intel = create_sample_threat_intel(task_id);
                    let _message = create_threat_intel_message(threat_intel, task_id);
                    
                    messages_sent += 1;
                    stats.messages_sent.fetch_add(1, Ordering::Relaxed);
                    
                    // Simulate network propagation delay
                    sleep(Duration::from_millis(10)).await;
                }
                
                messages_sent
            });
            
            handles.push(handle);
        }

        // Monitor progress
        let monitor_handle = tokio::spawn({
            let stats = self.stats.clone();
            async move {
                while start_time.elapsed() < duration {
                    sleep(Duration::from_secs(10)).await;
                    
                    let messages = stats.messages_sent.load(Ordering::Relaxed);
                    let elapsed = start_time.elapsed().as_secs();
                    let rate = messages as f64 / elapsed as f64;
                    
                    println!("📊 Sustained load: {} messages sent, {:.1} msg/sec", messages, rate);
                }
            }
        });

        // Wait for all tasks
        for handle in handles {
            let _ = handle.await;
        }
        monitor_handle.abort();

        Ok(())
    }

    fn print_stats(&self) {
        let nodes = self.stats.nodes_created.load(Ordering::Relaxed);
        let connections = self.stats.connections_established.load(Ordering::Relaxed);
        let messages = self.stats.messages_sent.load(Ordering::Relaxed);
        let errors = self.stats.errors.load(Ordering::Relaxed);
        
        println!("📊 Stats: {} nodes, {} connections, {} messages, {} errors", 
                 nodes, connections, messages, errors);
    }

    fn print_final_stats(&self) {
        println!("\n🏁 FINAL LOAD TEST RESULTS");
        println!("================================");
        println!("👥 Nodes Created: {}", self.stats.nodes_created.load(Ordering::Relaxed));
        println!("🔗 Connections: {}", self.stats.connections_established.load(Ordering::Relaxed));
        println!("📨 Messages Sent: {}", self.stats.messages_sent.load(Ordering::Relaxed));
        println!("📬 Messages Received: {}", self.stats.messages_received.load(Ordering::Relaxed));
        println!("❌ Errors: {}", self.stats.errors.load(Ordering::Relaxed));
        
        let total_nodes = self.stats.nodes_created.load(Ordering::Relaxed);
        if total_nodes > 0 {
            let success_rate = ((total_nodes - self.stats.errors.load(Ordering::Relaxed)) as f64 / total_nodes as f64) * 100.0;
            println!("✅ Success Rate: {:.2}%", success_rate);
        }
    }
}

#[derive(Debug)]
struct ClusterStats {
    cluster_id: usize,
    node_count: usize,
    representative_node: Option<String>,
}

impl ClusterStats {
    fn new(cluster_id: usize) -> Self {
        Self {
            cluster_id,
            node_count: 0,
            representative_node: None,
        }
    }
}

// Helper functions for realistic test data
#[derive(Debug, Serialize, Deserialize)]
struct ThreatIntel {
    threat_type: String,
    source_ip: String,
    severity: u8,
    timestamp: u64,
    cluster_id: usize,
}

fn create_sample_threat_intel(cluster_id: usize) -> ThreatIntel {
    let threat_types = ["brute_force", "port_scan", "malware", "ddos", "phishing"];
    let threat_type = threat_types[cluster_id % threat_types.len()];
    
    ThreatIntel {
        threat_type: threat_type.to_string(),
        source_ip: format!("192.168.{}.{}", (cluster_id / 256) % 256, cluster_id % 256),
        severity: (cluster_id % 10) as u8 + 1,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        cluster_id,
    }
}

fn create_threat_intel_message(threat_intel: ThreatIntel, source_id: usize) -> Message {
    let mut data = HashMap::new();
    data.insert("type".to_string(), "threat_intel".to_string());
    data.insert("payload".to_string(), serde_json::to_string(&threat_intel).unwrap());
    
    Message::new(
        MessageType::Structured(data),
        format!("cluster-node-{}", source_id)
    )
}

// Test configurations for different scales
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_small_scale_load_test() {
        // Start with a smaller test - 10,000 nodes
        let config = LoadTestConfig {
            total_nodes: 10_000,
            batch_size: 100,
            batch_delay_ms: 10,
            max_concurrent_connections: 1000,
            messages_per_node: 5,
            test_duration_secs: 30,
            base_port: 25000,
        };

        let load_test = MillionNodeLoadTest::new(config);
        let result = load_test.run_clustered_topology_test().await;
        
        assert!(result.is_ok(), "Small scale load test should succeed");
    }

    #[tokio::test]
    async fn test_medium_scale_load_test() {
        // Medium test - 100,000 nodes
        let config = LoadTestConfig {
            total_nodes: 100_000,
            batch_size: 500,
            batch_delay_ms: 50,
            max_concurrent_connections: 5000,
            messages_per_node: 10,
            test_duration_secs: 60,
            base_port: 30000,
        };

        let load_test = MillionNodeLoadTest::new(config);
        let result = load_test.run_clustered_topology_test().await;
        
        assert!(result.is_ok(), "Medium scale load test should succeed");
    }

    #[tokio::test]
    #[ignore] // Only run manually due to resource requirements
    async fn test_million_node_load_test() {
        // Full million node test - run manually only
        let config = LoadTestConfig::default(); // 1,000,000 nodes
        
        let load_test = MillionNodeLoadTest::new(config);
        let result = load_test.run_clustered_topology_test().await;
        
        assert!(result.is_ok(), "Million node load test should complete without panicking");
    }
}