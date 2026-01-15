use renoir::StreamContext;
use renoir_sql::sql;

fn main() {
    let ctx = StreamContext::new_local();

    sql! {ctx,
        "
        CREATE SOURCE data (id i32, name String, score f64)
        WITH (
            connector = 'csv', 
            path = 'data.csv',
            has_headers = 'true'
        );

        CREATE SINK output (name String, score f64)
        WITH (connector = 'csv', path = 'output.csv');
        
        INSERT INTO output
        SELECT name, score FROM data WHERE score > 75.0;
        "
    }

    ctx.execute_blocking();
}
