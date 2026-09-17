use renoir::prelude::*;
use renoir_sql::sql;

fn main() {
    let ctx = StreamContext::new_local();

    sql! {ctx,
        "
        CREATE SOURCE users (id i64, name String, age i32, city String)
        WITH (connector = 'csv', path = 'users.csv');

        CREATE SINK adults (id i64, name String, city String)
        WITH (connector = 'csv', path = 'adults.csv');
        
        INSERT INTO adults
        SELECT * FROM users WHERE age >= 18 LIMIT 100;
        "
    }

    // #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
    // struct User {
    //     id: i64,
    //     name: String,
    //     age: i32,
    //     city: String,
    // }

    // let source: CsvSource<User> = CsvSource::new("users.csv")
    //     .has_headers(true);

    // ctx.stream(source)
    //     .filter(|user| user.age >= 18)
    //     .map(|user| (user.id, user.name, user.city))
    //     .write_csv_seq("adults.csv".into(), false);

    ctx.execute_blocking();
}
