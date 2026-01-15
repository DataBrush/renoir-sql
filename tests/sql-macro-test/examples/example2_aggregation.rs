use renoir::StreamContext;
use renoir_sql::sql;

/// Example 2: Simple aggregation - Count users by department
/// Demonstrates WHERE clause with complex filtering
///
/// NOTE: Full GROUP BY with aggregation sinks requires additional integration work.
/// This example demonstrates the SQL parsing and code generation capabilities.
fn main() {
    let ctx = StreamContext::new_local();

    sql! {ctx,
        "
        CREATE SOURCE users (id i64, name String, age i32, city String, salary i64, department String)
        WITH (
            connector = 'csv', 
            path = 'test-data/users.csv',
            has_headers = 'true'
        );

        CREATE SINK high_earners (id i64, name String, salary i64, department String)
        WITH (
            connector = 'csv', 
            path = 'test-data/output-high-earners.csv',
            has_headers = 'false'
        );
        
        INSERT INTO high_earners
        SELECT id, name, salary, department
        FROM users
        WHERE salary > 70000;
        "
    }

    ctx.execute_blocking();
}
