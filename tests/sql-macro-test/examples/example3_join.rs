use renoir::StreamContext;
use renoir_sql::sql;

/// Example 3: Department filtering - Engineering team
/// Demonstrates WHERE clause with string equality
/// 
/// NOTE: JOIN support requires additional connector integration work.
/// This example shows string-based filtering.
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

        CREATE SINK engineers (id i64, name String, salary i64, city String)
        WITH (
            connector = 'csv', 
            path = 'test-data/output-engineers.csv',
            has_headers = 'false'
        );
        
        INSERT INTO engineers
        SELECT id, name, salary, city
        FROM users
        WHERE department = 'Engineering';
        "
    }

    ctx.execute_blocking();
}
