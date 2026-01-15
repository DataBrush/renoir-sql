use renoir_codegen::generate_program_with_context;
use renoir_ir::*;
use std::sync::Arc;

#[test]
fn test_csv_filter_pipeline() {
    // CREATE SOURCE users (id i64, name String, age i32)
    // WITH (connector = 'csv', path = 'users.csv');
    let source = SourceDef {
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

    // CREATE SINK output (id i64, name String)
    // WITH (connector = 'csv', path = 'output.csv');
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

    // SELECT * FROM users WHERE age > 18
    let filter_plan = Arc::new(IrPlan::Filter {
        input: Arc::new(IrPlan::Source {
            source_name: "users".to_string(),
            alias: None,
        }),
        predicate: FilterClause::Base(FilterConditionType::Comparison(Condition {
            left_field: ComplexField {
                column_ref: Some(ColumnRef {
                    table: None,
                    column: "age".to_string(),
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
                literal: Some(IrLiteral::Integer(18)),
                aggregate: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            },
        })),
    });

    let pipeline = Pipeline {
        sink_name: "output".to_string(),
        sink_columns: vec!["id".to_string(), "name".to_string()],
        plan: filter_plan,
    };

    let program = Program {
        sources: vec![source],
        sinks: vec![sink],
        pipelines: vec![pipeline],
    };

    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    // Verify generated code contains expected elements
    assert!(code_str.contains("use serde"), "Should import serde");
    assert!(
        code_str.contains("struct Users"),
        "Should define Users struct"
    );
    assert!(
        code_str.contains("CsvSource"),
        "Should use CsvSource for source"
    );
    assert!(
        code_str.contains("let source ="),
        "Should create source variable"
    );
    assert!(code_str.contains("filter"), "Should have filter operation");
    assert!(
        code_str.contains("write_csv_seq"),
        "Should write to CSV sink"
    );
    assert!(code_str.contains("age"), "Should reference age field");
    assert!(code_str.contains("18"), "Should have literal value 18");
}

#[test]
fn test_kafka_to_csv_pipeline() {
    let source = SourceDef {
        name: "events".to_string(),
        schema: vec![
            FieldDef {
                name: "event_id".to_string(),
                data_type: DataType::BigInt,
            },
            FieldDef {
                name: "value".to_string(),
                data_type: DataType::Double,
            },
        ],
        connector: ConnectorConfig {
            connector_type: "kafka".to_string(),
            options: vec![
                ConnectorOption {
                    key: "brokers".to_string(),
                    value: OptionValue::String("localhost:9092".to_string()),
                },
                ConnectorOption {
                    key: "topic".to_string(),
                    value: OptionValue::String("input_events".to_string()),
                },
                ConnectorOption {
                    key: "group_id".to_string(),
                    value: OptionValue::String("processor".to_string()),
                },
            ],
        },
    };

    let sink = SinkDef {
        name: "results".to_string(),
        schema: vec![FieldDef {
            name: "event_id".to_string(),
            data_type: DataType::BigInt,
        }],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("results.csv".to_string()),
            }],
        },
    };

    let pipeline = Pipeline {
        sink_name: "results".to_string(),
        sink_columns: vec!["event_id".to_string()],
        plan: Arc::new(IrPlan::Source {
            source_name: "events".to_string(),
            alias: None,
        }),
    };

    let program = Program {
        sources: vec![source],
        sinks: vec![sink],
        pipelines: vec![pipeline],
    };

    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    // Verify Kafka source setup
    assert!(
        code_str.contains("ClientConfig"),
        "Should import ClientConfig"
    );
    assert!(
        code_str.contains("consumer_config"),
        "Should create consumer config"
    );
    assert!(
        code_str.contains("localhost:9092"),
        "Should have broker address"
    );
    assert!(
        code_str.contains("input_events"),
        "Should reference topic"
    );
    assert!(
        code_str.contains("stream_kafka"),
        "Should use stream_kafka"
    );

    // Verify CSV sink
    assert!(
        code_str.contains("write_csv_seq"),
        "Should write to CSV"
    );
    assert!(
        code_str.contains("results.csv"),
        "Should have output path"
    );
}

#[test]
fn test_limit_operation() {
    let source = SourceDef {
        name: "data".to_string(),
        schema: vec![FieldDef {
            name: "value".to_string(),
            data_type: DataType::Integer,
        }],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("data.csv".to_string()),
            }],
        },
    };

    let sink = SinkDef {
        name: "limited".to_string(),
        schema: vec![FieldDef {
            name: "value".to_string(),
            data_type: DataType::Integer,
        }],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("limited.csv".to_string()),
            }],
        },
    };

    // SELECT * FROM data LIMIT 100 OFFSET 50
    let plan = Arc::new(IrPlan::Limit {
        input: Arc::new(IrPlan::Source {
            source_name: "data".to_string(),
            alias: None,
        }),
        limit: 100,
        offset: Some(50),
    });

    let pipeline = Pipeline {
        sink_name: "limited".to_string(),
        sink_columns: vec!["value".to_string()],
        plan,
    };

    let program = Program {
        sources: vec![source],
        sinks: vec![sink],
        pipelines: vec![pipeline],
    };

    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    // After refactoring, limit with offset is implemented using rich_filter_map
    assert!(code_str.contains("rich_filter_map"), "Should use rich_filter_map for limit");
    assert!(code_str.contains("50"), "Should skip 50 records");
    assert!(code_str.contains("100"), "Should limit to 100 records");
}

#[test]
fn test_csv_source_with_options() {
    // Test CSV source with custom configuration options
    let source = SourceDef {
        name: "data".to_string(),
        schema: vec![
            FieldDef {
                name: "id".to_string(),
                data_type: DataType::Integer,
            },
            FieldDef {
                name: "value".to_string(),
                data_type: DataType::String,
            },
        ],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![
                ConnectorOption {
                    key: "path".to_string(),
                    value: OptionValue::String("data.csv".to_string()),
                },
                ConnectorOption {
                    key: "delimiter".to_string(),
                    value: OptionValue::String(";".to_string()),
                },
                ConnectorOption {
                    key: "has_headers".to_string(),
                    value: OptionValue::String("false".to_string()),
                },
                ConnectorOption {
                    key: "quote".to_string(),
                    value: OptionValue::String("'".to_string()),
                },
            ],
        },
    };

    let sink = SinkDef {
        name: "output".to_string(),
        schema: vec![FieldDef {
            name: "value".to_string(),
            data_type: DataType::String,
        }],
        connector: ConnectorConfig {
            connector_type: "csv".to_string(),
            options: vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("output.csv".to_string()),
            }],
        },
    };

    let plan = Arc::new(IrPlan::Source {
        source_name: "data".to_string(),
        alias: None,
    });

    let pipeline = Pipeline {
        sink_name: "output".to_string(),
        sink_columns: vec!["value".to_string()],
        plan,
    };

    let program = Program {
        sources: vec![source],
        sinks: vec![sink],
        pipelines: vec![pipeline],
    };

    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    let code = generate_program_with_context(&program, &ctx_name);
    let code_str = code.to_string();

    // Verify CSV options are applied
    assert!(code_str.contains("CsvSource"), "Should use CsvSource");
    assert!(code_str.contains("delimiter"), "Should have delimiter configuration");
    assert!(code_str.contains("has_headers"), "Should have has_headers configuration");
    assert!(code_str.contains("quote"), "Should have quote configuration");
    assert!(code_str.contains("59"), "Delimiter ';' should be converted to byte 59");
    assert!(code_str.contains("false"), "has_headers should be false");
    assert!(code_str.contains("39"), "Quote '\'' should be converted to byte 39");
}
