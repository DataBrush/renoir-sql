use renoir::StreamContext;
use renoir_sql::sql;

/// Example 8: Purchase event extraction demonstration
/// Demonstrates the SQL syntax for streaming event filtering
/// 
/// NOTE: Currently uses CSV for demonstration. Kafka connector implementation pending.
/// The SQL shows how stream processing would work once Kafka connectors are ready.
/// 
/// Prerequisites:
/// 1. Start Kafka: ./kafka-scripts/setup-kafka.sh
/// 2. Publish events: ./kafka-scripts/publish-events.sh
/// 3. Run this example: cargo run --example example8_kafka_join
fn main() {
    let ctx = StreamContext::new_local();

    // TODO: Future version with JOIN and Kafka connector
    // Requires:
    // 1. JOIN support with multi-source coordination
    // 2. Stream-to-stream join implementation
    // 3. Kafka source/sink connector implementation
    /*
    sql! {ctx,
        "
        CREATE SOURCE user_events (
            event_id i64, 
            event_type String, 
            user_id i64, 
            timestamp String,
            session_duration i64
        )
        WITH (
            connector = 'kafka',
            topic = 'user-events',
            bootstrap_servers = 'localhost:9092'
        );

        CREATE SOURCE users (
            id i64,
            name String,
            age i64,
            city String,
            salary f64,
            department String
        )
        WITH (
            connector = 'kafka',
            topic = 'users',
            bootstrap_servers = 'localhost:9092'
        );

        CREATE SINK enriched_purchases (
            event_id i64,
            user_id i64,
            user_name String,
            timestamp String
        )
        WITH (
            connector = 'kafka',
            topic = 'enriched-purchases',
            bootstrap_servers = 'localhost:9092'
        );
        
        INSERT INTO enriched_purchases
        SELECT e.event_id, e.user_id, u.name as user_name, e.timestamp
        FROM user_events e
        JOIN users u ON e.user_id = u.id
        WHERE e.event_type = 'purchase';
        "
    }
    */

    sql! {ctx,
        "
        CREATE SOURCE user_events (
            event_id i64, 
            event_type String, 
            user_id i64, 
            timestamp String,
            session_duration i64
        )
        WITH (
            connector = 'csv',
            path = 'test-data/events-stream.csv',
            has_headers = 'false'
        );

        CREATE SINK enriched_purchases (
            event_id i64,
            user_id i64,
            event_type String,
            timestamp String
        )
        WITH (
            connector = 'csv',
            path = 'test-data/output-purchases.csv',
            has_headers = 'false'
        );
        
        INSERT INTO enriched_purchases
        SELECT event_id, user_id, event_type, timestamp
        FROM user_events
        WHERE event_type = 'purchase';
        "
    }
    
    ctx.execute_blocking();
}
