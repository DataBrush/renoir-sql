// Example: Building a streaming program using the IR builder API

use renoir_ir::*;
use renoir_ir::builder::*;
use std::sync::Arc;

fn main() {
    // Example 1: Simple filtering pipeline
    let program = ProgramBuilder::new()
        // Define a Kafka source
        .add_source(
            SourceDefBuilder::new("users")
                .add_field("id", "i64")
                .add_field("name", "String")
                .add_field("age", "i32")
                .with_connector("kafka")
                .with_option("topic", "users-topic")
                .with_option("bootstrap.servers", "localhost:9092")
                .build()
        )
        // Define a CSV sink
        .add_sink(
            SinkDefBuilder::new("output")
                .add_field("id", "i64")
                .add_field("name", "String")
                .with_connector("csv")
                .with_option("path", "/tmp/output.csv")
                .build()
        )
        // Define a streaming pipeline: users -> filter -> map -> output
        .add_pipeline(
            PipelineBuilder::new("output")
                .add_column("id")
                .add_column("name")
                .with_plan(
                    // Build the streaming plan
                    IrPlanBuilder::source("users")
                        .filter(FilterBuilder::binary_op(
                            BinaryOp::GreaterThan,
                            ComplexField::Column(ColumnRefBuilder::simple("age")),
                            ComplexField::Literal(Literal::Integer(18))
                        ))
                        .map(vec![
                            ProjectionBuilder::column("id"),
                            ProjectionBuilder::column("name"),
                        ])
                        .build()
                )
                .build()
        )
        .build();

    // The resulting program can be passed to code generation
    println!("Program with {} sources, {} sinks, {} pipelines", 
        program.sources.len(), 
        program.sinks.len(), 
        program.pipelines.len()
    );

    // Example 2: Aggregation pipeline
    let aggregation_program = ProgramBuilder::new()
        .add_source(
            SourceDefBuilder::new("orders")
                .add_field("order_id", "i64")
                .add_field("user_id", "i64")
                .add_field("amount", "f64")
                .with_connector("kafka")
                .with_option("topic", "orders")
                .build()
        )
        .add_sink(
            SinkDefBuilder::new("user_totals")
                .add_field("user_id", "i64")
                .add_field("total", "f64")
                .with_connector("postgres")
                .with_option("table", "user_totals")
                .build()
        )
        .add_pipeline(
            PipelineBuilder::new("user_totals")
                .add_column("user_id")
                .add_column("total")
                .with_plan(
                    IrPlanBuilder::source("orders")
                        .group_by(
                            vec![ColumnRefBuilder::simple("user_id")],  // GROUP BY user_id
                            vec![
                                // SELECT user_id, SUM(amount) as total
                                ProjectionBuilder::column("user_id"),
                                ProjectionBuilder::aggregate(
                                    AggregateFunctionBuilder::sum(
                                        ComplexField::Column(ColumnRefBuilder::simple("amount"))
                                    )
                                ).with_alias("total"),
                            ],
                            None  // No HAVING clause
                        )
                        .build()
                )
                .build()
        )
        .build();

    // Example 3: Join pipeline
    let join_program = ProgramBuilder::new()
        .add_source(
            SourceDefBuilder::new("users")
                .add_field("id", "i64")
                .add_field("name", "String")
                .with_connector("kafka")
                .with_option("topic", "users")
                .build()
        )
        .add_source(
            SourceDefBuilder::new("orders")
                .add_field("id", "i64")
                .add_field("user_id", "i64")
                .add_field("total", "f64")
                .with_connector("kafka")
                .with_option("topic", "orders")
                .build()
        )
        .add_sink(
            SinkDefBuilder::new("enriched_orders")
                .add_field("user_name", "String")
                .add_field("order_total", "f64")
                .with_connector("csv")
                .with_option("path", "/tmp/enriched.csv")
                .build()
        )
        .add_pipeline(
            PipelineBuilder::new("enriched_orders")
                .add_column("user_name")
                .add_column("order_total")
                .with_plan(
                    IrPlanBuilder::join(
                        IrPlanBuilder::source("users").with_alias("u").build(),
                        IrPlanBuilder::source("orders").with_alias("o").build(),
                        JoinType::Inner,
                        vec![
                            JoinConditionBuilder::on(
                                ColumnRefBuilder::qualified("u", "id"),
                                ColumnRefBuilder::qualified("o", "user_id")
                            )
                        ]
                    )
                    .map(vec![
                        ProjectionBuilder::qualified("u", "name").with_alias("user_name"),
                        ProjectionBuilder::qualified("o", "total").with_alias("order_total"),
                    ])
                    .build()
                )
                .build()
        )
        .build();

    println!("Join program created successfully!");
}
