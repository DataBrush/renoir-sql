use renoir::StreamContext;
use renoir_sql::sql;

/// Example 7: Long session filtering demonstration
/// Demonstrates the SQL syntax for streaming event filtering
/// 
/// NOTE: Currently uses CSV for demonstration. Kafka connector implementation pending.
/// The SQL shows how streaming filtering would work once Kafka connectors are ready.
/// 
/// Prerequisites:
/// 1. Start Kafka: ./kafka-scripts/setup-kafka.sh
/// 2. Publish events: ./kafka-scripts/publish-events.sh
/// 3. Run this example: cargo run --example example7_kafka_aggregation
fn main() {
    let ctx = StreamContext::new_local();

    // TODO: Future version with GROUP BY aggregation and Kafka connector
    // Requires:
    // 1. GROUP BY support with aggregate functions (COUNT, AVG, SUM)
    // 2. Aggregate result sink implementation
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
            topic = 'user-sessions',
            bootstrap_servers = 'localhost:9092'
        );

        CREATE SINK event_summary (
            user_id i64,
            session_count i64,
            avg_duration f64,
            total_duration i64
        )
        WITH (
            connector = 'kafka',
            topic = 'session-statistics',
            bootstrap_servers = 'localhost:9092'
        );
        
        INSERT INTO event_summary
        SELECT user_id, COUNT(*) as session_count, AVG(session_duration) as avg_duration, SUM(session_duration) as total_duration
        FROM user_events
        GROUP BY user_id;
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

        CREATE SINK event_summary (
            event_id i64,
            event_type String,
            user_id i64,
            session_duration i64
        )
        WITH (
            connector = 'csv',
            path = 'test-data/output-long-sessions.csv',
            has_headers = 'false'
        );
        
        INSERT INTO event_summary
        SELECT event_id, event_type, user_id, session_duration
        FROM user_events
        WHERE session_duration > 300;
        "
    }

    ctx.execute_blocking();
}
