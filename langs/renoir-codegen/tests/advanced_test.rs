use renoir_codegen::generate_program_with_context;
use renoir_ir::{
    AggregateFunction, AggregateType, ColumnRef, ConnectorConfig, ConnectorOption, DataType,
    FieldDef, IrPlan, JoinCondition, JoinType, OrderByItem, OrderDirection, OptionValue,
    Pipeline, Program, ProjectionColumn, SinkDef, SourceDef,
};
use std::sync::Arc;

fn create_test_program(plan: Arc<IrPlan>) -> Program {
    let source1 = SourceDef {
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
        ],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("users.csv".to_string()),
            }],
        },
    };

    let source2 = SourceDef {
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
        ],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("orders.csv".to_string()),
            }],
        },
    };

    let source3 = SourceDef {
        name: "products".to_string(),
        schema: vec![
            FieldDef {
                name: "id".to_string(),
                data_type: DataType::Integer,
            },
            FieldDef {
                name: "price".to_string(),
                data_type: DataType::Float,
            },
        ],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("products.csv".to_string()),
            }],
        },
    };

    let source4 = SourceDef {
        name: "table1".to_string(),
        schema: vec![
            FieldDef {
                name: "col1".to_string(),
                data_type: DataType::Integer,
            },
            FieldDef {
                name: "col2".to_string(),
                data_type: DataType::Integer,
            },
        ],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("table1.csv".to_string()),
            }],
        },
    };

    let source5 = SourceDef {
        name: "table2".to_string(),
        schema: vec![
            FieldDef {
                name: "col1".to_string(),
                data_type: DataType::Integer,
            },
            FieldDef {
                name: "col2".to_string(),
                data_type: DataType::Integer,
            },
        ],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("table2.csv".to_string()),
            }],
        },
    };

    let sink = SinkDef {
        name: "output".to_string(),
        schema: vec![],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("output.csv".to_string()),
            }],
        },
    };

    let pipeline = Pipeline {
        sink_name: "output".to_string(),
        sink_columns: vec![],
        plan,
    };

    Program {
        sources: vec![source1, source2, source3, source4, source5],
        sinks: vec![sink],
        pipelines: vec![pipeline],
    }
}

#[test]
fn test_inner_join() {
    let left = Arc::new(IrPlan::Source {
        source_name: "users".to_string(),
        alias: None,
    });
    let right = Arc::new(IrPlan::Source {
        source_name: "orders".to_string(),
        alias: None,
    });

    let condition = vec![JoinCondition {
        left_col: ColumnRef {
            table: Some("users".to_string()),
            column: "id".to_string(),
        },
        right_col: ColumnRef {
            table: Some("orders".to_string()),
            column: "user_id".to_string(),
        },
    }];

    let join_plan = Arc::new(IrPlan::Join {
        left,
        right,
        condition,
        join_type: JoinType::Inner,
    });

    let program = create_test_program(join_plan);
    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    // Check for join operation
    assert!(code_str.contains("join"));
}

#[test]
fn test_left_join() {
    let left = Arc::new(IrPlan::Source {
        source_name: "users".to_string(),
        alias: None,
    });
    let right = Arc::new(IrPlan::Source {
        source_name: "orders".to_string(),
        alias: None,
    });

    let condition = vec![JoinCondition {
        left_col: ColumnRef {
            table: Some("users".to_string()),
            column: "id".to_string(),
        },
        right_col: ColumnRef {
            table: Some("orders".to_string()),
            column: "user_id".to_string(),
        },
    }];

    let join_plan = Arc::new(IrPlan::Join {
        left,
        right,
        condition,
        join_type: JoinType::Left,
    });

    let program = create_test_program(join_plan);
    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    // Check for left_join operation
    assert!(code_str.contains("left_join"));
}

#[test]
fn test_group_by_with_count() {
    let input = Arc::new(IrPlan::Source {
        source_name: "orders".to_string(),
        alias: None,
    });

    let keys = vec![ColumnRef {
        table: Some("orders".to_string()),
        column: "customer_id".to_string(),
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

    let group_by_plan = Arc::new(IrPlan::GroupBy {
        input,
        keys,
        aggregations,
        having: None,
    });

    let program = create_test_program(group_by_plan);
    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    // Check for group_by operation
    assert!(code_str.contains("group_by"));
    assert!(code_str.contains("fold"));
}

#[test]
fn test_group_by_with_sum() {
    let input = Arc::new(IrPlan::Source {
        source_name: "orders".to_string(),
        alias: None,
    });

    let keys = vec![ColumnRef {
        table: Some("orders".to_string()),
        column: "customer_id".to_string(),
    }];

    let aggregations = vec![ProjectionColumn::Aggregate(
        AggregateFunction {
            function: AggregateType::Sum,
            column: ColumnRef {
                table: Some("orders".to_string()),
                column: "amount".to_string(),
            },
        },
        Some("total_amount".to_string()),
    )];

    let group_by_plan = Arc::new(IrPlan::GroupBy {
        input,
        keys,
        aggregations,
        having: None,
    });

    let program = create_test_program(group_by_plan);
    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    // Check for group_by and fold operations
    assert!(code_str.contains("group_by"));
    assert!(code_str.contains("fold"));
}

#[test]
fn test_group_by_with_avg() {
    let input = Arc::new(IrPlan::Source {
        source_name: "orders".to_string(),
        alias: None,
    });

    let keys = vec![ColumnRef {
        table: Some("orders".to_string()),
        column: "customer_id".to_string(),
    }];

    let aggregations = vec![ProjectionColumn::Aggregate(
        AggregateFunction {
            function: AggregateType::Avg,
            column: ColumnRef {
                table: Some("orders".to_string()),
                column: "amount".to_string(),
            },
        },
        Some("avg_amount".to_string()),
    )];

    let group_by_plan = Arc::new(IrPlan::GroupBy {
        input,
        keys,
        aggregations,
        having: None,
    });

    let program = create_test_program(group_by_plan);
    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    // Check for group_by and averaging logic
    assert!(code_str.contains("group_by"));
    assert!(code_str.contains("fold"));
}

#[test]
fn test_order_by_asc() {
    let input = Arc::new(IrPlan::Source {
        source_name: "products".to_string(),
        alias: None,
    });

    let items = vec![OrderByItem {
        column: ColumnRef {
            table: Some("products".to_string()),
            column: "price".to_string(),
        },
        direction: OrderDirection::Asc,
        nulls_first: None,
    }];

    let order_by_plan = Arc::new(IrPlan::OrderBy { input, items });

    let program = create_test_program(order_by_plan);
    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    // Check for sorting operations
    assert!(code_str.contains("sort_by"));
    assert!(code_str.contains("collect_vec"));
}

#[test]
fn test_order_by_desc() {
    let input = Arc::new(IrPlan::Source {
        source_name: "products".to_string(),
        alias: None,
    });

    let items = vec![OrderByItem {
        column: ColumnRef {
            table: Some("products".to_string()),
            column: "price".to_string(),
        },
        direction: OrderDirection::Desc,
        nulls_first: None,
    }];

    let order_by_plan = Arc::new(IrPlan::OrderBy { input, items });

    let program = create_test_program(order_by_plan);
    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    // Check for sorting operations
    assert!(code_str.contains("sort_by"));
    assert!(code_str.contains("collect_vec"));
}

#[test]
fn test_complex_query_join_and_group_by() {
    // SELECT customer_id, COUNT(*) 
    // FROM users JOIN orders ON users.id = orders.user_id
    // GROUP BY customer_id

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
        join_type: JoinType::Inner,
    });

    let keys = vec![ColumnRef {
        table: Some("orders".to_string()),
        column: "customer_id".to_string(),
    }];

    let aggregations = vec![ProjectionColumn::Aggregate(
        AggregateFunction {
            function: AggregateType::Count,
            column: ColumnRef {
                table: Some("orders".to_string()),
                column: "id".to_string(),
            },
        },
        Some("count".to_string()),
    )];

    let group_by_plan = Arc::new(IrPlan::GroupBy {
        input: joined,
        keys,
        aggregations,
        having: None,
    });

    let program = create_test_program(group_by_plan);
    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    // Check for both join and group_by operations
    assert!(code_str.contains("join"));
    assert!(code_str.contains("group_by"));
    assert!(code_str.contains("fold"));
}

#[test]
fn test_multi_column_join() {
    let left = Arc::new(IrPlan::Source {
        source_name: "table1".to_string(),
        alias: None,
    });
    let right = Arc::new(IrPlan::Source {
        source_name: "table2".to_string(),
        alias: None,
    });

    let conditions = vec![
        JoinCondition {
            left_col: ColumnRef {
                table: Some("table1".to_string()),
                column: "col1".to_string(),
            },
            right_col: ColumnRef {
                table: Some("table2".to_string()),
                column: "col1".to_string(),
            },
        },
        JoinCondition {
            left_col: ColumnRef {
                table: Some("table1".to_string()),
                column: "col2".to_string(),
            },
            right_col: ColumnRef {
                table: Some("table2".to_string()),
                column: "col2".to_string(),
            },
        },
    ];

    let join_plan = Arc::new(IrPlan::Join {
        left,
        right,
        condition: conditions,
        join_type: JoinType::Inner,
    });

    let program = create_test_program(join_plan);
    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    // Check for join with composite key (tuple)
    assert!(code_str.contains("join"));
}
