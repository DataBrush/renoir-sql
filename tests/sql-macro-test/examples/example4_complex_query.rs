use renoir::StreamContext;
use renoir_sql::sql;

/// Example 4: Product filtering by category
/// Demonstrates category-based filtering
fn main() {
    let ctx = StreamContext::new_local();

    sql! {ctx,
        "
        CREATE SOURCE products (product_id i64, product_name String, category String, price f64, stock i32)
        WITH (
            connector = 'csv', 
            path = 'test-data/products.csv',
            has_headers = 'true'
        );

        CREATE SINK premium_products (product_id i64, product_name String, category String, price f64)
        WITH (
            connector = 'csv', 
            path = 'test-data/output-premium-products.csv',
            has_headers = 'false'
        );
        
        INSERT INTO premium_products
        SELECT product_id, product_name, category, price
        FROM products
        WHERE category = 'Electronics';
        "
    }

    ctx.execute_blocking();
}
