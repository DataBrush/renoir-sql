use renoir_codegen::generate_program_with_context;
use renoir_ir::*;
use std::sync::Arc;

#[test]
fn test_simple_in_subquery() {
    // CREATE SOURCE users (id i64, name String, age i32)
    let users_source = SourceDef {
        name: "users".to_string(),
        schema: vec![
            FieldDef {
                name: "id".to_string(),
                data_type: DataType::BigInt,
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
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("users.csv".to_string()),
            }],
        },
    };

    // CREATE SOURCE orders (user_id i64, total f64)
    let orders_source = SourceDef {
        name: "orders".to_string(),
        schema: vec![
            FieldDef {
                name: "user_id".to_string(),
                data_type: DataType::BigInt,
            },
            FieldDef {
                name: "total".to_string(),
                data_type: DataType::Double,
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

    // CREATE SINK output (id i64, name String)
    let sink = SinkDef {
        name: "output".to_string(),
        schema: vec![
            FieldDef {
                name: "id".to_string(),
                data_type: DataType::BigInt,
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
                value: OptionValue::String("output.csv".to_string()),
            }],
        },
    };

    // Subquery: SELECT user_id FROM orders WHERE total > 100
    let subquery_plan = Arc::new(IrPlan::Map {
        input: Arc::new(IrPlan::Filter {
            input: Arc::new(IrPlan::Source {
                source_name: "orders".to_string(),
                alias: None,
            }),
            predicate: FilterClause::Base(FilterConditionType::Comparison(Condition {
                left_field: ComplexField {
                    aggregate: None,
                    column_ref: Some(ColumnRef {
                        table: None,
                        column: "total".to_string(),
                    }),
                    literal: None,
                    nested_expr: None,
                    subquery: None,
                    subquery_vec: None,
                },
                operator: ComparisonOp::GreaterThan,
                right_field: ComplexField {
                    aggregate: None,
                    column_ref: None,
                    literal: Some(IrLiteral::Float(100.0)),
                    nested_expr: None,
                    subquery: None,
                    subquery_vec: None,
                },
            })),
        }),
        projections: vec![ProjectionColumn::Column(
            ColumnRef {
                table: None,
                column: "user_id".to_string(),
            },
            None,
        )],
    });

    // Main query: SELECT id, name FROM users WHERE id IN (subquery)
    let main_plan = Arc::new(IrPlan::Filter {
        input: Arc::new(IrPlan::Source {
            source_name: "users".to_string(),
            alias: None,
        }),
        predicate: FilterClause::Base(FilterConditionType::In(InCondition::Subquery {
            field: ComplexField {
                    aggregate: None,
                column_ref: Some(ColumnRef {
                    table: None,
                    column: "id".to_string(),
                }),
                literal: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            },
            subquery: subquery_plan,
            negated: false,
        })),
    });

    let pipeline = Pipeline {
        sink_name: "output".to_string(),
        sink_columns: vec!["id".to_string(), "name".to_string()],
        plan: main_plan,
    };

    let program = Program {
        sources: vec![users_source, orders_source],
        sinks: vec![sink],
        pipelines: vec![pipeline],
    };

    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    println!("Generated code:\n{}", code_str);

    // Verify subquery execution code is generated
    assert!(
        code_str.contains("subquery"),
        "Should contain subquery execution code"
    );
    assert!(
        code_str.contains("collect_all"),
        "Should use collect_all for subquery"
    );
    assert!(
        code_str.contains("execute_blocking"),
        "Should have execute_blocking call"
    );
    assert!(code_str.contains("contains"), "Should check if value is in subquery results");
}

#[test]
fn test_exists_subquery() {
    // Test EXISTS subquery
    let users_source = SourceDef {
        name: "users".to_string(),
        schema: vec![FieldDef {
            name: "id".to_string(),
            data_type: DataType::BigInt,
        }],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("users.csv".to_string()),
            }],
        },
    };

    let orders_source = SourceDef {
        name: "orders".to_string(),
        schema: vec![FieldDef {
            name: "user_id".to_string(),
            data_type: DataType::BigInt,
        }],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("orders.csv".to_string()),
            }],
        },
    };

    let sink = SinkDef {
        name: "output".to_string(),
        schema: vec![FieldDef {
            name: "id".to_string(),
            data_type: DataType::BigInt,
        }],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("output.csv".to_string()),
            }],
        },
    };

    // Subquery: SELECT user_id FROM orders
    let subquery_plan = Arc::new(IrPlan::Source {
        source_name: "orders".to_string(),
        alias: None,
    });

    // Main query: SELECT id FROM users WHERE EXISTS (subquery)
    let main_plan = Arc::new(IrPlan::Filter {
        input: Arc::new(IrPlan::Source {
            source_name: "users".to_string(),
            alias: None,
        }),
        predicate: FilterClause::Base(FilterConditionType::Exists(
            ExistsCondition::Subquery {
                subquery: subquery_plan,
                negated: false,
            },
        )),
    });

    let pipeline = Pipeline {
        sink_name: "output".to_string(),
        sink_columns: vec!["id".to_string()],
        plan: main_plan,
    };

    let program = Program {
        sources: vec![users_source, orders_source],
        sinks: vec![sink],
        pipelines: vec![pipeline],
    };

    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    println!("Generated code for EXISTS:\n{}", code_str);

    // Verify EXISTS logic is generated
    assert!(
        code_str.contains("subquery"),
        "Should contain subquery execution"
    );
    assert!(
        code_str.contains("is_empty"),
        "Should check if subquery result is empty for EXISTS"
    );
}
