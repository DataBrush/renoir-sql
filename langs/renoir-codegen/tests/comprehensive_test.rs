/// Comprehensive end-to-end tests for Phase 5
/// These tests combine multiple SQL features to validate complex query patterns
use renoir_codegen::generate_program_with_context;
use renoir_ir::{
    AggregateFunction, AggregateType, ColumnRef, ComparisonOp, ComplexField, Condition,
    ConnectorConfig, ConnectorOption, DataType, FieldDef, FilterClause, FilterConditionType,
    InCondition, IrPlan, JoinCondition, JoinType, NullCondition, NullOp, OptionValue, OrderByItem,
    OrderDirection, Pipeline, Program, ProjectionColumn, SinkDef, SourceDef,
};
use std::sync::Arc;

fn create_users_source() -> SourceDef {
    SourceDef {
        name: "users".to_string(),
        schema: vec![
            FieldDef {
                name: "id".to_string(),
                data_type: DataType::Integer,
            },
            FieldDef {
                name: "name".to_string(),
                data_type: DataType::String,
            },
            FieldDef {
                name: "age".to_string(),
                data_type: DataType::Integer,
            },
        ],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![
                ConnectorOption {
                    key: "path".to_string(),
                    value: OptionValue::String("users.csv".to_string()),
                },
                ConnectorOption {
                    key: "has_headers".to_string(),
                    value: OptionValue::Boolean(true),
                },
            ],
        },
    }
}

fn create_orders_source() -> SourceDef {
    SourceDef {
        name: "orders".to_string(),
        schema: vec![
            FieldDef {
                name: "id".to_string(),
                data_type: DataType::Integer,
            },
            FieldDef {
                name: "user_id".to_string(),
                data_type: DataType::Integer,
            },
            FieldDef {
                name: "amount".to_string(),
                data_type: DataType::Float,
            },
            FieldDef {
                name: "status".to_string(),
                data_type: DataType::String,
            },
        ],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![
                ConnectorOption {
                    key: "path".to_string(),
                    value: OptionValue::String("orders.csv".to_string()),
                },
                ConnectorOption {
                    key: "has_headers".to_string(),
                    value: OptionValue::Boolean(true),
                },
            ],
        },
    }
}

fn create_products_source() -> SourceDef {
    SourceDef {
        name: "products".to_string(),
        schema: vec![
            FieldDef {
                name: "id".to_string(),
                data_type: DataType::Integer,
            },
            FieldDef {
                name: "name".to_string(),
                data_type: DataType::String,
            },
            FieldDef {
                name: "price".to_string(),
                data_type: DataType::Float,
            },
        ],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![
                ConnectorOption {
                    key: "path".to_string(),
                    value: OptionValue::String("products.csv".to_string()),
                },
                ConnectorOption {
                    key: "has_headers".to_string(),
                    value: OptionValue::Boolean(true),
                },
            ],
        },
    }
}

fn create_output_sink() -> SinkDef {
    SinkDef {
        name: "output".to_string(),
        schema: vec![],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("output.csv".to_string()),
            }],
        },
    }
}

#[test]
fn test_join_with_filter_and_aggregation() {
    // SELECT user_id, COUNT(*), SUM(amount)
    // FROM orders
    // WHERE status = 'completed'
    // GROUP BY user_id

    let orders = Arc::new(IrPlan::Source {
        source_name: "orders".to_string(),
        alias: None,
    });

    // Filter: status = 'completed'
    let filter = FilterClause::Base(FilterConditionType::Comparison(Condition {
        left_field: ComplexField {
            column_ref: Some(ColumnRef {
                table: Some("orders".to_string()),
                column: "status".to_string(),
            }),
            literal: None,
            aggregate: None,
            nested_expr: None,
            subquery: None,
            subquery_vec: None,
        },
        operator: ComparisonOp::Equal,
        right_field: ComplexField {
            column_ref: None,
            literal: Some(renoir_ir::IrLiteral::String("completed".to_string())),
            aggregate: None,
            nested_expr: None,
            subquery: None,
            subquery_vec: None,
        },
    }));

    let filtered = Arc::new(IrPlan::Filter {
        input: orders,
        predicate: filter,
    });

    // GROUP BY user_id with COUNT(*) and SUM(amount)
    let keys = vec![ColumnRef {
        table: Some("orders".to_string()),
        column: "user_id".to_string(),
    }];

    let aggregations = vec![
        ProjectionColumn::Aggregate(
            AggregateFunction {
                function: AggregateType::Count,
                column: ColumnRef {
                    table: Some("orders".to_string()),
                    column: "id".to_string(),
                },
            },
            Some("order_count".to_string()),
        ),
        ProjectionColumn::Aggregate(
            AggregateFunction {
                function: AggregateType::Sum,
                column: ColumnRef {
                    table: Some("orders".to_string()),
                    column: "amount".to_string(),
                },
            },
            Some("total_amount".to_string()),
        ),
    ];

    let group_by = Arc::new(IrPlan::GroupBy {
        input: filtered,
        keys,
        aggregations,
        having: None,
    });

    let program = Program {
        sources: vec![create_orders_source()],
        sinks: vec![create_output_sink()],
        pipelines: vec![Pipeline {
            sink_name: "output".to_string(),
            sink_columns: vec![],
            plan: group_by,
        }],
    };

    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    assert!(code_str.contains("filter"));
    assert!(code_str.contains("group_by"));
    assert!(code_str.contains("fold"));
}

#[test]
fn test_join_group_by_and_order_by() {
    // SELECT users.name, COUNT(*) as order_count
    // FROM users JOIN orders ON users.id = orders.user_id
    // GROUP BY users.name
    // ORDER BY order_count DESC

    let users = Arc::new(IrPlan::Source {
        source_name: "users".to_string(),
        alias: None,
    });

    let orders = Arc::new(IrPlan::Source {
        source_name: "orders".to_string(),
        alias: None,
    });

    // JOIN users and orders
    let join_condition = vec![JoinCondition {
        left_col: ColumnRef {
            table: Some("users".to_string()),
            column: "id".to_string(),
        },
        right_col: ColumnRef {
            table: Some("orders".to_string()),
            column: "user_id".to_string(),
        },
    }];

    let joined = Arc::new(IrPlan::Join {
        left: users,
        right: orders,
        condition: join_condition,
        join_type: JoinType::Inner,
    });

    // GROUP BY users.name
    let keys = vec![ColumnRef {
        table: Some("users".to_string()),
        column: "name".to_string(),
    }];

    let aggregations = vec![ProjectionColumn::Aggregate(
        AggregateFunction {
            function: AggregateType::Count,
            column: ColumnRef {
                table: Some("orders".to_string()),
                column: "id".to_string(),
            },
        },
        Some("order_count".to_string()),
    )];

    let grouped = Arc::new(IrPlan::GroupBy {
        input: joined,
        keys,
        aggregations,
        having: None,
    });

    // ORDER BY order_count DESC
    let order_items = vec![OrderByItem {
        column: ColumnRef {
            table: None,
            column: "order_count".to_string(),
        },
        direction: OrderDirection::Desc,
        nulls_first: None,
    }];

    let ordered = Arc::new(IrPlan::OrderBy {
        input: grouped,
        items: order_items,
    });

    let program = Program {
        sources: vec![create_users_source(), create_orders_source()],
        sinks: vec![create_output_sink()],
        pipelines: vec![Pipeline {
            sink_name: "output".to_string(),
            sink_columns: vec![],
            plan: ordered,
        }],
    };

    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    assert!(code_str.contains("join"));
    assert!(code_str.contains("group_by"));
    assert!(code_str.contains("sort_by"));
}

#[test]
fn test_subquery_with_join() {
    // SELECT * FROM users
    // WHERE id IN (SELECT user_id FROM orders WHERE amount > 100)

    // Subquery: SELECT user_id FROM orders WHERE amount > 100
    let orders = Arc::new(IrPlan::Source {
        source_name: "orders".to_string(),
        alias: None,
    });

    let subquery_filter = FilterClause::Base(FilterConditionType::Comparison(Condition {
        left_field: ComplexField {
            column_ref: Some(ColumnRef {
                table: Some("orders".to_string()),
                column: "amount".to_string(),
            }),
            literal: None,
            aggregate: None,
            nested_expr: None,
            subquery: None,
            subquery_vec: None,
        },
        operator: ComparisonOp::GreaterThan,
        right_field: ComplexField {
            column_ref: None,
            literal: Some(renoir_ir::IrLiteral::String("100".to_string())),
            aggregate: None,
            nested_expr: None,
            subquery: None,
            subquery_vec: None,
        },
    }));

    let subquery = Arc::new(IrPlan::Filter {
        input: orders,
        predicate: subquery_filter,
    });

    // Main query: users with IN condition
    let users = Arc::new(IrPlan::Source {
        source_name: "users".to_string(),
        alias: None,
    });

    let in_condition = InCondition::Subquery {
        field: ComplexField {
            column_ref: Some(ColumnRef {
                table: Some("users".to_string()),
                column: "id".to_string(),
            }),
            literal: None,
            aggregate: None,
            nested_expr: None,
            subquery: None,
            subquery_vec: None,
        },
        subquery,
        negated: false,
    };

    let main_filter = FilterClause::Base(FilterConditionType::In(in_condition));

    let main_plan = Arc::new(IrPlan::Filter {
        input: users,
        predicate: main_filter,
    });

    let program = Program {
        sources: vec![create_users_source(), create_orders_source()],
        sinks: vec![create_output_sink()],
        pipelines: vec![Pipeline {
            sink_name: "output".to_string(),
            sink_columns: vec![],
            plan: main_plan,
        }],
    };

    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    assert!(code_str.contains("collect_all"));
    assert!(code_str.contains("execute_blocking"));
    assert!(code_str.contains("contains"));
}

#[test]
fn test_multiple_aggregates() {
    // SELECT user_id,
    //        COUNT(*) as cnt,
    //        SUM(amount) as total,
    //        AVG(amount) as avg_amt,
    //        MAX(amount) as max_amt,
    //        MIN(amount) as min_amt
    // FROM orders
    // GROUP BY user_id

    let orders = Arc::new(IrPlan::Source {
        source_name: "orders".to_string(),
        alias: None,
    });

    let keys = vec![ColumnRef {
        table: Some("orders".to_string()),
        column: "user_id".to_string(),
    }];

    let aggregations = vec![
        ProjectionColumn::Aggregate(
            AggregateFunction {
                function: AggregateType::Count,
                column: ColumnRef {
                    table: Some("orders".to_string()),
                    column: "id".to_string(),
                },
            },
            Some("cnt".to_string()),
        ),
        ProjectionColumn::Aggregate(
            AggregateFunction {
                function: AggregateType::Sum,
                column: ColumnRef {
                    table: Some("orders".to_string()),
                    column: "amount".to_string(),
                },
            },
            Some("total".to_string()),
        ),
        ProjectionColumn::Aggregate(
            AggregateFunction {
                function: AggregateType::Avg,
                column: ColumnRef {
                    table: Some("orders".to_string()),
                    column: "amount".to_string(),
                },
            },
            Some("avg_amt".to_string()),
        ),
        ProjectionColumn::Aggregate(
            AggregateFunction {
                function: AggregateType::Max,
                column: ColumnRef {
                    table: Some("orders".to_string()),
                    column: "amount".to_string(),
                },
            },
            Some("max_amt".to_string()),
        ),
        ProjectionColumn::Aggregate(
            AggregateFunction {
                function: AggregateType::Min,
                column: ColumnRef {
                    table: Some("orders".to_string()),
                    column: "amount".to_string(),
                },
            },
            Some("min_amt".to_string()),
        ),
    ];

    let group_by = Arc::new(IrPlan::GroupBy {
        input: orders,
        keys,
        aggregations,
        having: None,
    });

    let program = Program {
        sources: vec![create_orders_source()],
        sinks: vec![create_output_sink()],
        pipelines: vec![Pipeline {
            sink_name: "output".to_string(),
            sink_columns: vec![],
            plan: group_by,
        }],
    };

    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    assert!(code_str.contains("group_by"));
    assert!(code_str.contains("fold"));
    // Should have 5 aggregates in the accumulator
}

#[test]
fn test_null_check_filter() {
    // SELECT * FROM users WHERE age IS NOT NULL

    let users = Arc::new(IrPlan::Source {
        source_name: "users".to_string(),
        alias: None,
    });

    let null_check = FilterClause::Base(FilterConditionType::NullCheck(NullCondition {
        field: ComplexField {
            column_ref: Some(ColumnRef {
                table: Some("users".to_string()),
                column: "age".to_string(),
            }),
            literal: None,
            aggregate: None,
            nested_expr: None,
            subquery: None,
            subquery_vec: None,
        },
        operator: NullOp::IsNotNull,
    }));

    let filtered = Arc::new(IrPlan::Filter {
        input: users,
        predicate: null_check,
    });

    let program = Program {
        sources: vec![create_users_source()],
        sinks: vec![create_output_sink()],
        pipelines: vec![Pipeline {
            sink_name: "output".to_string(),
            sink_columns: vec![],
            plan: filtered,
        }],
    };

    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    assert!(code_str.contains("filter"));
}

#[test]
fn test_distinct_with_limit() {
    // SELECT DISTINCT user_id FROM orders LIMIT 10

    let orders = Arc::new(IrPlan::Source {
        source_name: "orders".to_string(),
        alias: None,
    });

    let distinct = Arc::new(IrPlan::Distinct { input: orders });

    let limited = Arc::new(IrPlan::Limit {
        input: distinct,
        limit: 10,
        offset: None,
    });

    let program = Program {
        sources: vec![create_orders_source()],
        sinks: vec![create_output_sink()],
        pipelines: vec![Pipeline {
            sink_name: "output".to_string(),
            sink_columns: vec![],
            plan: limited,
        }],
    };

    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    assert!(code_str.contains("group_by")); // DISTINCT uses group_by
    assert!(code_str.contains("rich_filter_map")); // LIMIT uses rich_filter_map
}

#[test]
fn test_left_join_with_null_handling() {
    // SELECT * FROM users LEFT JOIN orders ON users.id = orders.user_id

    let users = Arc::new(IrPlan::Source {
        source_name: "users".to_string(),
        alias: None,
    });

    let orders = Arc::new(IrPlan::Source {
        source_name: "orders".to_string(),
        alias: None,
    });

    let join_condition = vec![JoinCondition {
        left_col: ColumnRef {
            table: Some("users".to_string()),
            column: "id".to_string(),
        },
        right_col: ColumnRef {
            table: Some("orders".to_string()),
            column: "user_id".to_string(),
        },
    }];

    let joined = Arc::new(IrPlan::Join {
        left: users,
        right: orders,
        condition: join_condition,
        join_type: JoinType::Left,
    });

    let program = Program {
        sources: vec![create_users_source(), create_orders_source()],
        sinks: vec![create_output_sink()],
        pipelines: vec![Pipeline {
            sink_name: "output".to_string(),
            sink_columns: vec![],
            plan: joined,
        }],
    };

    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    assert!(code_str.contains("left_join"));
    assert!(code_str.contains("NULL")); // Should pad with NULL for missing right side
}
