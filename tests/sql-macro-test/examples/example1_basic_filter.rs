use renoir::StreamContext;
use renoir_sql::sql;

/// Example 1: Simple filtering - Find adults eligible to vote
/// Demonstrates basic WHERE clause with age filtering
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

        CREATE SINK adults (id i64, name String, city String)
        WITH (
            connector = 'csv', 
            path = 'test-data/output-adults.csv',
            has_headers = 'false'
        );
        
        INSERT INTO adults
        SELECT id, name, city FROM users WHERE age >= 18;
        "
    }

    ctx.execute_blocking();
}
