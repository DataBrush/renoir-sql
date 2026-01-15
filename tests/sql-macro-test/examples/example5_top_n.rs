use renoir::StreamContext;
use renoir_sql::sql;

/// Example 5: Management team filtering
/// Demonstrates filtering by department and complex conditions
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

        CREATE SINK managers (id i64, name String, salary i64, city String)
        WITH (
            connector = 'csv', 
            path = 'test-data/output-managers.csv',
            has_headers = 'false'
        );
        
        INSERT INTO managers
        SELECT id, name, salary, city
        FROM users
        WHERE department = 'Management';
        "
    }

    ctx.execute_blocking();
}
