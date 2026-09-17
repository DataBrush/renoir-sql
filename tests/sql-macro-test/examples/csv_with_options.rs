use renoir::prelude::*;
use renoir_sql::sql;

fn main() {
    env_logger::init();
    let ctx = StreamContext::new_local();

    // sql! {ctx,
    //     "
    //     CREATE SOURCE users (id i64, name String, age i32, city String, salary i64, department String)
    //     WITH (
    //         connector = 'csv', 
    //         path = 'shared/users.csv',
    //         has_headers = 'true'
    //     );

    //     CREATE SINK adults (id i64, name String, city String)
    //     WITH (
    //         connector = 'csv', 
    //         path = 'shared/output-adults.csv',
    //         has_headers = 'false'
    //     );

    //     INSERT INTO adults
    //     SELECT id, name, city FROM users WHERE department = 'Engineering';
    //     "
    // }

    sql! {ctx,
        "
        CREATE SOURCE users (id i64, name String, age i32, city String, salary i64, department String)
        WITH (
            connector = 'kafka',
            brokers = '127.0.0.1:9092',
            topic = 'users'
        );

        CREATE SINK adults (id i64, name String, city String)
        WITH (
            connector = 'kafka',
            brokers = '127.0.0.1:9092',
            topic = 'adults'
        );

        INSERT INTO adults
        SELECT id, name, city FROM users WHERE department = 'Engineering';
        "
    }

    ctx.execute_blocking();
}
