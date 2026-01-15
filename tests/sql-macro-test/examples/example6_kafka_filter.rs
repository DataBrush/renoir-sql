use renoir::StreamContext;
use renoir_sql::sql;

/// Example 6: Event filtering demonstration
/// Demonstrates the SQL syntax for Kafka-style event filtering
/// 
/// NOTE: Currently uses CSV for demonstration. Kafka connector implementation pending.
/// The SQL shows how Kafka streaming would be configured once connectors are ready.
/// 
/// Prerequisites:
/// 1. Start Kafka: ./kafka-scripts/setup-kafka.sh
/// 2. Publish events: ./kafka-scripts/publish-events.sh
/// 3. Run this example: cargo run --example example6_kafka_filter
fn main() {
    let ctx = StreamContext::new_local();

    // TODO: Future version with IN operator and Kafka connector
    // Requires:
    // 1. Parser support for IN operator
    // 2. Kafka source/sink connector implementation
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

        CREATE SINK important_events (
            event_id i64,
            event_type String,
            user_id i64,
            timestamp String
        )
        WITH (
            connector = 'kafka',
            topic = 'filtered-events',
            bootstrap_servers = 'localhost:9092'
        );
        
        INSERT INTO important_events
        SELECT event_id, event_type, user_id, timestamp
        FROM user_events
        WHERE event_type IN ('login', 'purchase', 'logout');
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

        CREATE SINK important_events (
            event_id i64,
            event_type String,
            user_id i64,
            timestamp String
        )
        WITH (
            connector = 'csv',
            path = 'test-data/output-purchase-events.csv',
            has_headers = 'false'
        );
        
        INSERT INTO important_events
        SELECT event_id, event_type, user_id, timestamp
        FROM user_events
        WHERE event_type = 'purchase';
        "
    }
    
    ctx.execute_blocking();
}
