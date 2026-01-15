use renoir::prelude::*;
// use renoir_sql::sql;

fn main() {
    println!("SQL Macro Example - Demonstrating Code Generation");
    println!("===============================================\n");
    
    let config = RuntimeConfig::local(4).unwrap();
    let ctx = StreamContext::new(config);
    
    // sql! { ctx,
    //     ""
    // }
}
