use pest::Parser;
use pest_derive::Parser;
use renoir_ir::*;
use std::sync::Arc;

#[derive(Parser)]
#[grammar = "sql.pest"]
pub struct SqlParser;

pub type ParseError = pest::error::Error<Rule>;

/// Parse a SQL program (potentially multiple statements) and return IR
pub fn parse_sql(input: &str) -> Result<Program, ParseError> {
    let mut pairs = SqlParser::parse(Rule::program, input)?;
    let program_pair = pairs.next().unwrap();

    parse_program(program_pair)
}

fn parse_program(pair: pest::iterators::Pair<Rule>) -> Result<Program, ParseError> {
    let mut sources = Vec::new();
    let mut sinks = Vec::new();
    let mut pipelines = Vec::new();

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::statement => {
                let stmt_result = parse_statement(inner_pair)?;
                match stmt_result {
                    Statement::CreateSource(source_def) => sources.push(source_def),
                    Statement::CreateSink(sink_def) => sinks.push(sink_def),
                    Statement::InsertInto(pipeline) => pipelines.push(pipeline),
                }
            }
            Rule::EOI => {}
            _ => {}
        }
    }

    Ok(Program {
        sources,
        sinks,
        pipelines,
    })
}

enum Statement {
    CreateSource(SourceDef),
    CreateSink(SinkDef),
    InsertInto(Pipeline),
}

fn parse_statement(pair: pest::iterators::Pair<Rule>) -> Result<Statement, ParseError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::query => {
            // Standalone query - this is an error in our streaming context
            // In a streaming engine, queries must be part of INSERT INTO
            Err(pest::error::Error::new_from_pos(
                pest::error::ErrorVariant::CustomError {
                    message: "Standalone queries not supported. Use INSERT INTO <sink> SELECT ..."
                        .to_string(),
                },
                pest::Position::from_start(""),
            ))
        }
        Rule::create_source => {
            let source_def = parse_create_source(inner)?;
            Ok(Statement::CreateSource(source_def))
        }
        Rule::create_sink => {
            let sink_def = parse_create_sink(inner)?;
            Ok(Statement::CreateSink(sink_def))
        }
        Rule::insert_into => {
            let pipeline = parse_insert_into(inner)?;
            Ok(Statement::InsertInto(pipeline))
        }
        _ => unreachable!("Unexpected statement type: {:?}", inner.as_rule()),
    }
}

fn parse_query(pair: pest::iterators::Pair<Rule>) -> Result<Arc<IrPlan>, ParseError> {
    let mut distinct = false;
    let mut projections = Vec::new();
    let mut from_plan: Option<Arc<IrPlan>> = None;
    let mut where_clause: Option<FilterClause> = None;
    let mut group_by_keys: Option<Vec<ColumnRef>> = None;
    let mut group_aggregations: Option<Vec<ProjectionColumn>> = None;
    let mut having_clause: Option<GroupClause> = None;
    let mut order_by_items: Option<Vec<OrderByItem>> = None;
    let mut limit_value: Option<i64> = None;
    let mut offset_value: Option<i64> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::select => {}
            Rule::distinct_keyword => {
                distinct = true;
            }
            Rule::asterisk => {
                // SELECT * - no projection needed
                projections.clear();
            }
            Rule::column_list => {
                projections = parse_column_list(inner)?;
            }
            Rule::from_expr => {
                from_plan = Some(parse_from_expr(inner)?);
            }
            Rule::where_expr => {
                where_clause = Some(parse_where_expr(inner)?);
            }
            Rule::group_by_expr => {
                let (keys, having) = parse_group_by_expr(inner)?;
                group_by_keys = Some(keys);
                having_clause = having;
                // Separate aggregations from regular columns in projections
                group_aggregations = Some(projections.clone());
            }
            Rule::order_by_expr => {
                order_by_items = Some(parse_order_by_expr(inner)?);
            }
            Rule::limit_expr => {
                let (limit, offset) = parse_limit_expr(inner)?;
                limit_value = Some(limit);
                offset_value = offset;
            }
            _ => {}
        }
    }

    // Build the streaming pipeline bottom-up
    let mut plan = from_plan.expect("FROM clause is required");

    // Apply filter
    if let Some(filter) = where_clause {
        plan = Arc::new(IrPlan::Filter {
            input: plan,
            predicate: filter,
        });
    }

    // Apply grouping with aggregations
    if let Some(keys) = group_by_keys {
        let aggs = group_aggregations.unwrap_or_else(Vec::new);
        plan = Arc::new(IrPlan::GroupBy {
            input: plan,
            keys,
            aggregations: aggs,
            having: having_clause,
        });
    } else if !projections.is_empty() {
        // Apply map (projection) if no grouping
        plan = Arc::new(IrPlan::Map {
            input: plan,
            projections,
        });
    }

    // Apply distinct
    if distinct {
        plan = Arc::new(IrPlan::Distinct { input: plan });
    }

    // Apply ordering
    if let Some(items) = order_by_items {
        plan = Arc::new(IrPlan::OrderBy { input: plan, items });
    }

    // Apply limit
    if let Some(limit) = limit_value {
        plan = Arc::new(IrPlan::Limit {
            input: plan,
            limit,
            offset: offset_value,
        });
    }

    Ok(plan)
}

fn parse_from_expr(pair: pest::iterators::Pair<Rule>) -> Result<Arc<IrPlan>, ParseError> {
    let mut base_plan: Option<Arc<IrPlan>> = None;
    let mut joins = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::from => {}
            Rule::scan_expr => {
                base_plan = Some(parse_scan_expr(inner)?);
            }
            Rule::join_expr => {
                joins.push(inner);
            }
            _ => {}
        }
    }

    let mut plan = base_plan.expect("Base table/scan is required");

    // Process joins left-to-right
    for join_pair in joins {
        plan = parse_join_expr(plan, join_pair)?;
    }

    Ok(plan)
}

fn parse_scan_expr(pair: pest::iterators::Pair<Rule>) -> Result<Arc<IrPlan>, ParseError> {
    let mut source_name: Option<String> = None;
    let mut alias: Option<String> = None;
    let mut is_subquery = false;
    let mut subquery_plan: Option<Arc<IrPlan>> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::variable => {
                if source_name.is_none() {
                    source_name = Some(inner.as_str().to_string());
                }
            }
            Rule::alias => {
                alias = Some(inner.as_str().to_string());
            }
            Rule::subquery_expr => {
                is_subquery = true;
                subquery_plan = Some(parse_subquery_expr(inner)?);
            }
            Rule::as_keyword => {}
            _ => {}
        }
    }

    if is_subquery {
        // For subqueries, return the plan directly (alias handling TBD)
        Ok(subquery_plan.unwrap())
    } else {
        // Create a Source node referencing the defined source
        Ok(Arc::new(IrPlan::Source {
            source_name: source_name.expect("Source name is required"),
            alias,
        }))
    }
}

fn parse_subquery_expr(pair: pest::iterators::Pair<Rule>) -> Result<Arc<IrPlan>, ParseError> {
    // Subquery contains: l_paren ~ select ~ ... ~ r_paren
    // We need to parse everything inside as if it's a query
    let mut distinct = false;
    let mut projections = Vec::new();
    let mut from_plan: Option<Arc<IrPlan>> = None;
    let mut where_clause: Option<FilterClause> = None;
    let mut group_by_keys: Option<Vec<ColumnRef>> = None;
    let mut group_aggregations: Option<Vec<ProjectionColumn>> = None;
    let mut having_clause: Option<GroupClause> = None;
    let mut order_by_items: Option<Vec<OrderByItem>> = None;
    let mut limit_value: Option<i64> = None;
    let mut offset_value: Option<i64> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::select => {}
            Rule::distinct_keyword => {
                distinct = true;
            }
            Rule::asterisk => {
                projections.clear();
            }
            Rule::column_list => {
                projections = parse_column_list(inner)?;
            }
            Rule::from_expr => {
                from_plan = Some(parse_from_expr(inner)?);
            }
            Rule::where_expr => {
                where_clause = Some(parse_where_expr(inner)?);
            }
            Rule::group_by_expr => {
                let (keys, having) = parse_group_by_expr(inner)?;
                group_by_keys = Some(keys);
                having_clause = having;
                group_aggregations = Some(projections.clone());
            }
            Rule::order_by_expr => {
                order_by_items = Some(parse_order_by_expr(inner)?);
            }
            Rule::limit_expr => {
                let (limit, offset) = parse_limit_expr(inner)?;
                limit_value = Some(limit);
                offset_value = offset;
            }
            _ => {}
        }
    }

    // Build the query plan
    let mut plan = from_plan.expect("FROM clause is required in subquery");

    if let Some(filter) = where_clause {
        plan = Arc::new(IrPlan::Filter {
            input: plan,
            predicate: filter,
        });
    }

    if let Some(keys) = group_by_keys {
        let aggs = group_aggregations.unwrap_or_else(Vec::new);
        plan = Arc::new(IrPlan::GroupBy {
            input: plan,
            keys,
            aggregations: aggs,
            having: having_clause,
        });
    } else if !projections.is_empty() {
        plan = Arc::new(IrPlan::Map {
            input: plan,
            projections,
        });
    }

    if distinct {
        plan = Arc::new(IrPlan::Distinct { input: plan });
    }

    if let Some(items) = order_by_items {
        plan = Arc::new(IrPlan::OrderBy { input: plan, items });
    }

    if let Some(limit) = limit_value {
        plan = Arc::new(IrPlan::Limit {
            input: plan,
            limit,
            offset: offset_value,
        });
    }

    Ok(plan)
}

fn parse_join_expr(
    left: Arc<IrPlan>,
    pair: pest::iterators::Pair<Rule>,
) -> Result<Arc<IrPlan>, ParseError> {
    let mut join_type = JoinType::Inner;
    let mut right: Option<Arc<IrPlan>> = None;
    let mut conditions = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::join_kind => {
                join_type = parse_join_kind(inner)?;
            }
            Rule::join => {}
            Rule::scan_expr => {
                right = Some(parse_scan_expr(inner)?);
            }
            Rule::subquery_expr => {
                right = Some(parse_subquery_expr(inner)?);
            }
            Rule::on => {}
            Rule::join_condition => {
                conditions = parse_join_condition(inner)?;
            }
            _ => {}
        }
    }

    Ok(Arc::new(IrPlan::Join {
        left,
        right: right.expect("Right side of join is required"),
        condition: conditions,
        join_type,
    }))
}

fn parse_join_kind(pair: pest::iterators::Pair<Rule>) -> Result<JoinType, ParseError> {
    let kind_str = pair.as_str().to_uppercase();

    if kind_str.contains("INNER") {
        Ok(JoinType::Inner)
    } else if kind_str.contains("LEFT") {
        Ok(JoinType::Left)
    } else if kind_str.contains("OUTER") {
        Ok(JoinType::Outer)
    } else {
        Ok(JoinType::Inner)
    }
}

fn parse_join_condition(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<JoinCondition>, ParseError> {
    let mut conditions = Vec::new();
    let mut current_left: Option<ColumnRef> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::table_column => {
                let col_ref = parse_table_column(inner)?;
                if current_left.is_none() {
                    current_left = Some(col_ref);
                } else {
                    conditions.push(JoinCondition {
                        left_col: current_left.take().unwrap(),
                        right_col: col_ref,
                    });
                }
            }
            _ => {}
        }
    }

    Ok(conditions)
}

fn parse_column_list(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<ProjectionColumn>, ParseError> {
    let mut columns = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::column_with_alias {
            columns.push(parse_column_with_alias(inner)?);
        }
    }

    Ok(columns)
}

fn parse_column_with_alias(
    pair: pest::iterators::Pair<Rule>,
) -> Result<ProjectionColumn, ParseError> {
    let mut column_item: Option<ProjectionColumn> = None;
    let mut alias: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::column_item => {
                column_item = Some(parse_column_item(inner)?);
            }
            Rule::as_keyword => {}
            Rule::variable => {
                if column_item.is_some() {
                    alias = Some(inner.as_str().to_string());
                }
            }
            _ => {}
        }
    }

    // Apply alias to the column
    let mut col = column_item.expect("Column item is required");
    if let Some(alias_name) = alias {
        col = match col {
            ProjectionColumn::Column(col_ref, _) => {
                ProjectionColumn::Column(col_ref, Some(alias_name))
            }
            ProjectionColumn::Aggregate(agg, _) => {
                ProjectionColumn::Aggregate(agg, Some(alias_name))
            }
            ProjectionColumn::ComplexValue(field, _) => {
                ProjectionColumn::ComplexValue(field, Some(alias_name))
            }
            ProjectionColumn::StringLiteral(s, _) => {
                ProjectionColumn::StringLiteral(s, Some(alias_name))
            }
            ProjectionColumn::Subquery(sq, _) => ProjectionColumn::Subquery(sq, Some(alias_name)),
            other => other,
        };
    }

    Ok(col)
}

fn parse_column_item(pair: pest::iterators::Pair<Rule>) -> Result<ProjectionColumn, ParseError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::select_expr => {
            let field = parse_select_expr(inner)?;
            Ok(ProjectionColumn::ComplexValue(field, None))
        }
        Rule::aggregate_expr => {
            let agg = parse_aggregate_expr(inner)?;
            Ok(ProjectionColumn::Aggregate(agg, None))
        }
        Rule::table_column => {
            let col_ref = parse_table_column(inner)?;
            Ok(ProjectionColumn::Column(col_ref, None))
        }
        Rule::variable => {
            let col_ref = ColumnRef {
                table: None,
                column: inner.as_str().to_string(),
            };
            Ok(ProjectionColumn::Column(col_ref, None))
        }
        Rule::string_literal => {
            let s = parse_string_literal(inner)?;
            Ok(ProjectionColumn::StringLiteral(s, None))
        }
        Rule::subquery_expr => {
            let sq = parse_subquery_expr(inner)?;
            Ok(ProjectionColumn::Subquery(sq, None))
        }
        _ => unreachable!("Unexpected column item: {:?}", inner.as_rule()),
    }
}

fn parse_select_expr(pair: pest::iterators::Pair<Rule>) -> Result<ComplexField, ParseError> {
    let mut operands = Vec::new();
    let mut operators = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::parenthesized_expr => {
                let field = parse_parenthesized_expr(inner)?;
                operands.push(field);
            }
            Rule::column_operand => {
                let field = parse_column_operand(inner)?;
                operands.push(field);
            }
            Rule::symbol => {
                operators.push(inner.as_str().to_string());
            }
            _ => {}
        }
    }

    if operands.is_empty() {
        return Err(pest::error::Error::new_from_pos(
            pest::error::ErrorVariant::CustomError {
                message: "Empty select expression".to_string(),
            },
            pest::Position::from_start(""),
        ));
    }

    // Build nested expression left-to-right
    let mut operands_iter = operands.into_iter();
    let mut result = operands_iter.next().unwrap();
    let mut op_iter = operators.into_iter();

    for operand in operands_iter {
        if let Some(op) = op_iter.next() {
            result = ComplexField {
                column_ref: None,
                literal: None,
                aggregate: None,
                nested_expr: Some(Box::new((result, op, operand, false))),
                subquery: None,
                subquery_vec: None,
            };
        }
    }

    Ok(result)
}

fn parse_parenthesized_expr(pair: pest::iterators::Pair<Rule>) -> Result<ComplexField, ParseError> {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::select_expr {
            let mut field = parse_select_expr(inner)?;
            // Mark as parenthesized by wrapping in nested_expr with same content
            if let Some(nested) = field.nested_expr.take() {
                let (left, op, right, _) = *nested;
                field.nested_expr = Some(Box::new((left, op, right, true)));
            }
            return Ok(field);
        }
    }
    unreachable!("Parenthesized expression must contain select_expr")
}

fn parse_column_operand(pair: pest::iterators::Pair<Rule>) -> Result<ComplexField, ParseError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::aggregate_expr => {
            let agg = parse_aggregate_expr(inner)?;
            Ok(ComplexField {
                column_ref: None,
                literal: None,
                aggregate: Some(agg),
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            })
        }
        Rule::table_column => {
            let col_ref = parse_table_column(inner)?;
            Ok(ComplexField {
                column_ref: Some(col_ref),
                literal: None,
                aggregate: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            })
        }
        Rule::variable => {
            let col_ref = ColumnRef {
                table: None,
                column: inner.as_str().to_string(),
            };
            Ok(ComplexField {
                column_ref: Some(col_ref),
                literal: None,
                aggregate: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            })
        }
        Rule::number => {
            let num = parse_number(inner)?;
            Ok(ComplexField {
                column_ref: None,
                literal: Some(num),
                aggregate: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            })
        }
        Rule::subquery_expr => {
            let sq = parse_subquery_expr(inner)?;
            Ok(ComplexField {
                column_ref: None,
                literal: None,
                aggregate: None,
                nested_expr: None,
                subquery: Some(sq),
                subquery_vec: None,
            })
        }
        _ => unreachable!("Unexpected column operand: {:?}", inner.as_rule()),
    }
}

fn parse_aggregate_expr(
    pair: pest::iterators::Pair<Rule>,
) -> Result<AggregateFunction, ParseError> {
    let mut function: Option<AggregateType> = None;
    let mut column: Option<ColumnRef> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::agg_function => {
                function = Some(parse_agg_function(inner)?);
            }
            Rule::table_column => {
                column = Some(parse_table_column(inner)?);
            }
            Rule::variable => {
                column = Some(ColumnRef {
                    table: None,
                    column: inner.as_str().to_string(),
                });
            }
            Rule::asterisk => {
                // COUNT(*) typically
                column = Some(ColumnRef {
                    table: None,
                    column: "*".to_string(),
                });
            }
            _ => {}
        }
    }

    Ok(AggregateFunction {
        function: function.expect("Aggregate function is required"),
        column: column.expect("Column is required for aggregate"),
    })
}

fn parse_agg_function(pair: pest::iterators::Pair<Rule>) -> Result<AggregateType, ParseError> {
    match pair.as_str().to_uppercase().as_str() {
        "MAX" => Ok(AggregateType::Max),
        "MIN" => Ok(AggregateType::Min),
        "AVG" => Ok(AggregateType::Avg),
        "SUM" => Ok(AggregateType::Sum),
        "COUNT" => Ok(AggregateType::Count),
        _ => unreachable!("Unknown aggregate function: {}", pair.as_str()),
    }
}

fn parse_table_column(pair: pest::iterators::Pair<Rule>) -> Result<ColumnRef, ParseError> {
    let mut parts = pair.into_inner();
    let table = parts.next().unwrap().as_str().to_string();
    let column = parts.next().unwrap().as_str().to_string();

    Ok(ColumnRef {
        table: Some(table),
        column,
    })
}

fn parse_where_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterClause, ParseError> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::where_keyword => {}
            Rule::where_conditions => {
                return parse_where_conditions(inner);
            }
            _ => {}
        }
    }
    unreachable!("WHERE expression must contain conditions")
}

fn parse_where_conditions(pair: pest::iterators::Pair<Rule>) -> Result<FilterClause, ParseError> {
    let mut terms = Vec::new();
    let mut operators = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::where_term => {
                terms.push(parse_where_term(inner)?);
            }
            Rule::binary_op => {
                operators.push(parse_binary_op(inner)?);
            }
            _ => {}
        }
    }

    if terms.is_empty() {
        return Err(pest::error::Error::new_from_pos(
            pest::error::ErrorVariant::CustomError {
                message: "Empty where conditions".to_string(),
            },
            pest::Position::from_start(""),
        ));
    }

    // Build expression tree left-to-right
    let mut terms_iter = terms.into_iter();
    let mut result = terms_iter.next().unwrap();
    let mut op_iter = operators.into_iter();

    for term in terms_iter {
        if let Some(op) = op_iter.next() {
            result = FilterClause::Expression {
                left: Box::new(result),
                binary_op: op,
                right: Box::new(term),
            };
        }
    }

    Ok(result)
}

fn parse_where_term(pair: pest::iterators::Pair<Rule>) -> Result<FilterClause, ParseError> {
    // where_term can be: l_paren ~ where_conditions ~ r_paren | condition
    // If we have inner pairs, check what they are
    let mut inner_pairs = pair.into_inner();
    let first = inner_pairs.next().unwrap();

    match first.as_rule() {
        Rule::where_conditions => parse_where_conditions(first),
        Rule::condition => parse_condition(first),
        _ => {
            // If it's something else (like l_paren), this must be the parenthesized case
            // The actual where_conditions should be the next element
            parse_where_conditions(inner_pairs.next().unwrap())
        }
    }
}

fn parse_condition(pair: pest::iterators::Pair<Rule>) -> Result<FilterClause, ParseError> {
    let mut inner_pairs = pair.into_inner();
    let inner = inner_pairs.next().unwrap();

    match inner.as_rule() {
        Rule::exists_expr => parse_exists_expr(inner),
        Rule::in_expr => parse_in_expr(inner),
        Rule::boolean => {
            let val = inner.as_str() == "true";
            Ok(FilterClause::Base(FilterConditionType::Boolean(val)))
        }
        Rule::arithmetic_expr => {
            // This might be part of a comparison or null check
            // We need to look at the siblings to determine
            let first = inner;

            if let Some(second) = inner_pairs.next() {
                match second.as_rule() {
                    Rule::operator => {
                        let left_field = parse_arithmetic_expr(first)?;
                        let op = parse_comparison_op(second)?;
                        let right_field = parse_arithmetic_expr(inner_pairs.next().unwrap())?;

                        Ok(FilterClause::Base(FilterConditionType::Comparison(
                            Condition {
                                left_field,
                                operator: op,
                                right_field,
                            },
                        )))
                    }
                    Rule::null_operator => {
                        let field = parse_arithmetic_expr(first)?;
                        let op = parse_null_op(second)?;

                        Ok(FilterClause::Base(FilterConditionType::NullCheck(
                            NullCondition {
                                field,
                                operator: op,
                            },
                        )))
                    }
                    _ => unreachable!("Unexpected condition part: {:?}", second.as_rule()),
                }
            } else {
                // Just an arithmetic expression - evaluate as boolean
                let field = parse_arithmetic_expr(first)?;
                Ok(FilterClause::Base(FilterConditionType::Comparison(
                    Condition {
                        left_field: field,
                        operator: ComparisonOp::NotEqual,
                        right_field: ComplexField {
                            column_ref: None,
                            literal: Some(IrLiteral::Integer(0)),
                            aggregate: None,
                            nested_expr: None,
                            subquery: None,
                            subquery_vec: None,
                        },
                    },
                )))
            }
        }
        _ => unreachable!("Unexpected condition: {:?}", inner.as_rule()),
    }
}

fn parse_exists_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterClause, ParseError> {
    let mut negated = false;
    let mut subquery: Option<Arc<IrPlan>> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::exists_keyword => {
                negated = inner.as_str().to_uppercase().contains("NOT");
            }
            Rule::subquery_expr => {
                subquery = Some(parse_subquery_expr(inner)?);
            }
            _ => {}
        }
    }

    Ok(FilterClause::Base(FilterConditionType::Exists(
        ExistsCondition::Subquery {
            subquery: subquery.expect("Subquery is required for EXISTS"),
            negated,
        },
    )))
}

fn parse_in_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterClause, ParseError> {
    let mut field: Option<ComplexField> = None;
    let mut negated = false;
    let mut subquery: Option<Arc<IrPlan>> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::arithmetic_expr => {
                if field.is_none() {
                    field = Some(parse_arithmetic_expr(inner)?);
                }
            }
            Rule::subquery_expr => {
                if field.is_none() {
                    // First subquery is the field
                    field = Some(ComplexField {
                        column_ref: None,
                        literal: None,
                        aggregate: None,
                        nested_expr: None,
                        subquery: Some(parse_subquery_expr(inner)?),
                        subquery_vec: None,
                    });
                } else {
                    // Second subquery is the IN list
                    subquery = Some(parse_subquery_expr(inner)?);
                }
            }
            Rule::in_keyword => {
                negated = inner.as_str().to_uppercase().contains("NOT");
            }
            _ => {}
        }
    }

    Ok(FilterClause::Base(FilterConditionType::In(
        InCondition::Subquery {
            field: field.expect("Field is required for IN"),
            subquery: subquery.expect("Subquery is required for IN"),
            negated,
        },
    )))
}

fn parse_arithmetic_expr(pair: pest::iterators::Pair<Rule>) -> Result<ComplexField, ParseError> {
    let mut terms = Vec::new();
    let mut operators = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::arithmetic_term => {
                terms.push(parse_arithmetic_term(inner)?);
            }
            Rule::symbol => {
                operators.push(inner.as_str().to_string());
            }
            Rule::subquery_expr => {
                return Ok(ComplexField {
                    column_ref: None,
                    literal: None,
                    aggregate: None,
                    nested_expr: None,
                    subquery: Some(parse_subquery_expr(inner)?),
                    subquery_vec: None,
                });
            }
            _ => {}
        }
    }

    if terms.is_empty() {
        return Err(pest::error::Error::new_from_pos(
            pest::error::ErrorVariant::CustomError {
                message: "Empty arithmetic expression".to_string(),
            },
            pest::Position::from_start(""),
        ));
    }

    // Build nested expression
    let mut terms_iter = terms.into_iter();
    let mut result = terms_iter.next().unwrap();
    let mut op_iter = operators.into_iter();

    for term in terms_iter {
        if let Some(op) = op_iter.next() {
            result = ComplexField {
                column_ref: None,
                literal: None,
                aggregate: None,
                nested_expr: Some(Box::new((result, op, term, false))),
                subquery: None,
                subquery_vec: None,
            };
        }
    }

    Ok(result)
}

fn parse_arithmetic_term(pair: pest::iterators::Pair<Rule>) -> Result<ComplexField, ParseError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::arithmetic_expr => {
            let mut field = parse_arithmetic_expr(inner)?;
            // Mark as parenthesized
            if let Some(nested) = field.nested_expr.take() {
                let (left, op, right, _) = *nested;
                field.nested_expr = Some(Box::new((left, op, right, true)));
            }
            Ok(field)
        }
        Rule::arithmetic_factor => parse_arithmetic_factor(inner),
        _ => unreachable!("Unexpected arithmetic term: {:?}", inner.as_rule()),
    }
}

fn parse_arithmetic_factor(pair: pest::iterators::Pair<Rule>) -> Result<ComplexField, ParseError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::aggregate_expr => {
            let agg = parse_aggregate_expr(inner)?;
            Ok(ComplexField {
                column_ref: None,
                literal: None,
                aggregate: Some(agg),
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            })
        }
        Rule::table_column => {
            let col_ref = parse_table_column(inner)?;
            Ok(ComplexField {
                column_ref: Some(col_ref),
                literal: None,
                aggregate: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            })
        }
        Rule::variable => {
            let col_ref = ColumnRef {
                table: None,
                column: inner.as_str().to_string(),
            };
            Ok(ComplexField {
                column_ref: Some(col_ref),
                literal: None,
                aggregate: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            })
        }
        Rule::number => {
            let num = parse_number(inner)?;
            Ok(ComplexField {
                column_ref: None,
                literal: Some(num),
                aggregate: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            })
        }
        Rule::string_literal => {
            let s = parse_string_literal(inner)?;
            Ok(ComplexField {
                column_ref: None,
                literal: Some(IrLiteral::String(s)),
                aggregate: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            })
        }
        Rule::boolean => {
            let val = inner.as_str() == "true";
            Ok(ComplexField {
                column_ref: None,
                literal: Some(IrLiteral::Boolean(val)),
                aggregate: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            })
        }
        Rule::subquery_expr => {
            let sq = parse_subquery_expr(inner)?;
            Ok(ComplexField {
                column_ref: None,
                literal: None,
                aggregate: None,
                nested_expr: None,
                subquery: Some(sq),
                subquery_vec: None,
            })
        }
        _ => unreachable!("Unexpected arithmetic factor: {:?}", inner.as_rule()),
    }
}

fn parse_comparison_op(pair: pest::iterators::Pair<Rule>) -> Result<ComparisonOp, ParseError> {
    match pair.as_str() {
        ">" => Ok(ComparisonOp::GreaterThan),
        "<" => Ok(ComparisonOp::LessThan),
        "=" => Ok(ComparisonOp::Equal),
        "!=" | "<>" => Ok(ComparisonOp::NotEqual),
        ">=" => Ok(ComparisonOp::GreaterThanEquals),
        "<=" => Ok(ComparisonOp::LessThanEquals),
        _ => unreachable!("Unknown comparison operator: {}", pair.as_str()),
    }
}

fn parse_null_op(pair: pest::iterators::Pair<Rule>) -> Result<NullOp, ParseError> {
    let s = pair.as_str().to_uppercase();
    if s.contains("NOT") {
        Ok(NullOp::IsNotNull)
    } else {
        Ok(NullOp::IsNull)
    }
}

fn parse_binary_op(pair: pest::iterators::Pair<Rule>) -> Result<BinaryOp, ParseError> {
    match pair.as_str().to_uppercase().as_str() {
        "AND" => Ok(BinaryOp::And),
        "OR" => Ok(BinaryOp::Or),
        _ => unreachable!("Unknown binary operator: {}", pair.as_str()),
    }
}

fn parse_number(pair: pest::iterators::Pair<Rule>) -> Result<IrLiteral, ParseError> {
    let s = pair.as_str();
    if s.contains('.') {
        Ok(IrLiteral::Float(s.parse().unwrap()))
    } else {
        Ok(IrLiteral::Integer(s.parse().unwrap()))
    }
}

fn parse_string_literal(pair: pest::iterators::Pair<Rule>) -> Result<String, ParseError> {
    // String literal is atomic and includes the quotes, so we need to extract the content
    let s = pair.as_str();
    // Remove the surrounding single quotes
    if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        Ok(s[1..s.len() - 1].to_string())
    } else {
        Ok(s.to_string())
    }
}

fn parse_group_by_expr(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(Vec<ColumnRef>, Option<GroupClause>), ParseError> {
    let mut keys = Vec::new();
    let mut having: Option<GroupClause> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::group_by_keyword => {}
            Rule::group_by_list => {
                keys = parse_group_by_list(inner)?;
            }
            Rule::having_keyword => {}
            Rule::having_expr => {
                having = Some(parse_having_expr(inner)?);
            }
            _ => {}
        }
    }

    Ok((keys, having))
}

fn parse_group_by_list(pair: pest::iterators::Pair<Rule>) -> Result<Vec<ColumnRef>, ParseError> {
    let mut keys = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::table_column => {
                keys.push(parse_table_column(inner)?);
            }
            Rule::variable => {
                keys.push(ColumnRef {
                    table: None,
                    column: inner.as_str().to_string(),
                });
            }
            _ => {}
        }
    }

    Ok(keys)
}

fn parse_having_expr(pair: pest::iterators::Pair<Rule>) -> Result<GroupClause, ParseError> {
    let mut terms = Vec::new();
    let mut operators = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::having_term => {
                terms.push(parse_having_term(inner)?);
            }
            Rule::binary_op => {
                operators.push(parse_binary_op(inner)?);
            }
            _ => {}
        }
    }

    if terms.is_empty() {
        return Err(pest::error::Error::new_from_pos(
            pest::error::ErrorVariant::CustomError {
                message: "Empty having expression".to_string(),
            },
            pest::Position::from_start(""),
        ));
    }

    // Build expression tree
    let mut terms_iter = terms.into_iter();
    let mut result = terms_iter.next().unwrap();
    let mut op_iter = operators.into_iter();

    for term in terms_iter {
        if let Some(op) = op_iter.next() {
            result = GroupClause::Expression {
                left: Box::new(result),
                op,
                right: Box::new(term),
            };
        }
    }

    Ok(result)
}

fn parse_having_term(pair: pest::iterators::Pair<Rule>) -> Result<GroupClause, ParseError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::having_expr => parse_having_expr(inner),
        Rule::condition => {
            // Convert FilterConditionType to GroupBaseCondition
            let filter = parse_condition(inner)?;
            convert_filter_to_group(filter)
        }
        _ => unreachable!("Unexpected having term: {:?}", inner.as_rule()),
    }
}

fn convert_filter_to_group(filter: FilterClause) -> Result<GroupClause, ParseError> {
    match filter {
        FilterClause::Base(cond_type) => {
            let base_cond = match cond_type {
                FilterConditionType::Comparison(c) => GroupBaseCondition::Comparison(c),
                FilterConditionType::NullCheck(n) => GroupBaseCondition::NullCheck(n),
                FilterConditionType::In(i) => GroupBaseCondition::In(i),
                FilterConditionType::Exists(e) => GroupBaseCondition::Exists(e),
                FilterConditionType::Boolean(b) => GroupBaseCondition::Boolean(b),
            };
            Ok(GroupClause::Base(base_cond))
        }
        FilterClause::Expression {
            left,
            binary_op,
            right,
        } => Ok(GroupClause::Expression {
            left: Box::new(convert_filter_to_group(*left)?),
            op: binary_op,
            right: Box::new(convert_filter_to_group(*right)?),
        }),
    }
}

fn parse_order_by_expr(pair: pest::iterators::Pair<Rule>) -> Result<Vec<OrderByItem>, ParseError> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::order_by_keyword => {}
            Rule::order_by_list => {
                return parse_order_by_list(inner);
            }
            _ => {}
        }
    }
    Ok(Vec::new())
}

fn parse_order_by_list(pair: pest::iterators::Pair<Rule>) -> Result<Vec<OrderByItem>, ParseError> {
    let mut items = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::order_item {
            items.push(parse_order_item(inner)?);
        }
    }

    Ok(items)
}

fn parse_order_item(pair: pest::iterators::Pair<Rule>) -> Result<OrderByItem, ParseError> {
    let mut column: Option<ColumnRef> = None;
    let mut direction = OrderDirection::Asc;
    let mut nulls_first: Option<bool> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::table_column => {
                column = Some(parse_table_column(inner)?);
            }
            Rule::variable => {
                column = Some(ColumnRef {
                    table: None,
                    column: inner.as_str().to_string(),
                });
            }
            Rule::order_direction => {
                direction = parse_order_direction(inner)?;
            }
            Rule::nulls_handling => {
                nulls_first = Some(parse_nulls_handling(inner)?);
            }
            _ => {}
        }
    }

    Ok(OrderByItem {
        column: column.expect("Column is required for ORDER BY"),
        direction,
        nulls_first,
    })
}

fn parse_order_direction(pair: pest::iterators::Pair<Rule>) -> Result<OrderDirection, ParseError> {
    match pair.as_str().to_uppercase().as_str() {
        "ASC" => Ok(OrderDirection::Asc),
        "DESC" => Ok(OrderDirection::Desc),
        _ => Ok(OrderDirection::Asc),
    }
}

fn parse_nulls_handling(pair: pest::iterators::Pair<Rule>) -> Result<bool, ParseError> {
    let s = pair.as_str().to_uppercase();
    Ok(s.contains("FIRST"))
}

fn parse_limit_expr(pair: pest::iterators::Pair<Rule>) -> Result<(i64, Option<i64>), ParseError> {
    let mut limit: Option<i64> = None;
    let mut offset: Option<i64> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::limit_clause => {
                limit = Some(parse_limit_clause(inner)?);
            }
            Rule::offset_clause => {
                offset = Some(parse_offset_clause(inner)?);
            }
            _ => {}
        }
    }

    Ok((limit.expect("LIMIT value is required"), offset))
}

fn parse_limit_clause(pair: pest::iterators::Pair<Rule>) -> Result<i64, ParseError> {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::number {
            return Ok(inner.as_str().parse().unwrap());
        }
    }
    Ok(0)
}

fn parse_offset_clause(pair: pest::iterators::Pair<Rule>) -> Result<i64, ParseError> {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::number {
            return Ok(inner.as_str().parse().unwrap());
        }
    }
    Ok(0)
}

// DDL parsing functions

fn parse_create_source(pair: pest::iterators::Pair<Rule>) -> Result<SourceDef, ParseError> {
    let mut source_name: Option<String> = None;
    let mut fields = Vec::new();
    let mut options = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::create_keyword | Rule::source_keyword => {}
            Rule::variable => {
                if source_name.is_none() {
                    source_name = Some(inner.as_str().to_string());
                }
            }
            Rule::field_list => {
                fields = parse_field_list(inner)?;
            }
            Rule::with_clause => {
                options = parse_with_clause(inner)?;
            }
            _ => {}
        }
    }

    // Extract connector type from options
    let connector_type = options
        .iter()
        .find(|opt| opt.key == "connector")
        .and_then(|opt| {
            if let OptionValue::String(s) = &opt.value {
                Some(s.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    Ok(SourceDef {
        name: source_name.expect("Source name is required"),
        schema: fields,
        connector: ConnectorConfig {
            connector_type,
            options,
        },
    })
}

fn parse_create_sink(pair: pest::iterators::Pair<Rule>) -> Result<SinkDef, ParseError> {
    let mut sink_name: Option<String> = None;
    let mut fields = Vec::new();
    let mut options = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::create_keyword | Rule::sink_keyword => {}
            Rule::variable => {
                if sink_name.is_none() {
                    sink_name = Some(inner.as_str().to_string());
                }
            }
            Rule::field_list => {
                fields = parse_field_list(inner)?;
            }
            Rule::with_clause => {
                options = parse_with_clause(inner)?;
            }
            _ => {}
        }
    }

    // Extract connector type from options
    let connector_type = options
        .iter()
        .find(|opt| opt.key == "connector")
        .and_then(|opt| {
            if let OptionValue::String(s) = &opt.value {
                Some(s.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    Ok(SinkDef {
        name: sink_name.expect("Sink name is required"),
        schema: fields,
        connector: ConnectorConfig {
            connector_type,
            options,
        },
    })
}

fn parse_insert_into(pair: pest::iterators::Pair<Rule>) -> Result<Pipeline, ParseError> {
    let mut table_name: Option<String> = None;
    let mut columns = Vec::new();
    let mut query: Option<Arc<IrPlan>> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::insert_keyword | Rule::into_keyword => {}
            Rule::variable => {
                if table_name.is_none() {
                    table_name = Some(inner.as_str().to_string());
                }
            }
            Rule::column_name_list => {
                columns = parse_column_name_list(inner)?;
            }
            Rule::query => {
                query = Some(parse_query(inner)?);
            }
            _ => {}
        }
    }

    Ok(Pipeline {
        sink_name: table_name.expect("Sink name is required"),
        sink_columns: columns,
        plan: query.expect("Query is required for INSERT INTO"),
    })
}

fn parse_field_list(pair: pest::iterators::Pair<Rule>) -> Result<Vec<FieldDef>, ParseError> {
    let mut fields = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::field_def {
            fields.push(parse_field_def(inner)?);
        }
    }

    Ok(fields)
}

fn parse_field_def(pair: pest::iterators::Pair<Rule>) -> Result<FieldDef, ParseError> {
    let pairs_vec: Vec<_> = pair.into_inner().collect();

    if pairs_vec.len() < 2 {
        return Err(pest::error::Error::new_from_pos(
            pest::error::ErrorVariant::CustomError {
                message: "Field definition must have name and type".to_string(),
            },
            pest::Position::from_start(""),
        ));
    }

    let name = pairs_vec[0].as_str().to_string();
    let data_type = parse_data_type(pairs_vec[1].clone())?;

    Ok(FieldDef { name, data_type })
}

fn parse_data_type(pair: pest::iterators::Pair<Rule>) -> Result<DataType, ParseError> {
    match pair.as_str().to_uppercase().as_str() {
        "INTEGER" | "I32" => Ok(DataType::Integer),
        "BIGINT" | "I64" => Ok(DataType::BigInt),
        "FLOAT" | "F32" => Ok(DataType::Float),
        "DOUBLE" | "F64" => Ok(DataType::Double),
        "STRING" => Ok(DataType::String),
        "BOOLEAN" | "BOOL" => Ok(DataType::Boolean),
        "TIMESTAMP" => Ok(DataType::Timestamp),
        _ => unreachable!("Unknown data type: {}", pair.as_str()),
    }
}

fn parse_with_clause(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<ConnectorOption>, ParseError> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::with_keyword => {}
            Rule::option_list => {
                return parse_option_list(inner);
            }
            _ => {}
        }
    }
    Ok(Vec::new())
}

fn parse_option_list(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<ConnectorOption>, ParseError> {
    let mut options = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::option_pair {
            options.push(parse_option_pair(inner)?);
        }
    }

    Ok(options)
}

fn parse_option_pair(pair: pest::iterators::Pair<Rule>) -> Result<ConnectorOption, ParseError> {
    let mut key: Option<String> = None;
    let mut value: Option<OptionValue> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::variable => {
                if key.is_none() {
                    key = Some(inner.as_str().to_string());
                } else {
                    value = Some(OptionValue::Variable(inner.as_str().to_string()));
                }
            }
            Rule::option_value => {
                value = Some(parse_option_value(inner)?);
            }
            _ => {}
        }
    }

    Ok(ConnectorOption {
        key: key.expect("Option key is required"),
        value: value.expect("Option value is required"),
    })
}

fn parse_option_value(pair: pest::iterators::Pair<Rule>) -> Result<OptionValue, ParseError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::string_literal => {
            let s = parse_string_literal(inner)?;
            Ok(OptionValue::String(s))
        }
        Rule::number => {
            let s = inner.as_str();
            Ok(OptionValue::Number(s.parse().unwrap()))
        }
        Rule::boolean => {
            let val = inner.as_str() == "true";
            Ok(OptionValue::Boolean(val))
        }
        Rule::variable => Ok(OptionValue::Variable(inner.as_str().to_string())),
        _ => unreachable!("Unexpected option value: {:?}", inner.as_rule()),
    }
}

fn parse_column_name_list(pair: pest::iterators::Pair<Rule>) -> Result<Vec<String>, ParseError> {
    let mut columns = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::variable {
            columns.push(inner.as_str().to_string());
        }
    }

    Ok(columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let sql = "INSERT INTO output (name, age) SELECT name, age FROM users;";
        let result = parse_sql(sql);
        assert!(result.is_ok());
        if let Ok(program) = result {
            assert_eq!(program.pipelines.len(), 1);
            // Verify the pipeline has a Source node in its plan
            let plan = &program.pipelines[0].plan;
            if let IrPlan::Map { input, .. } = plan.as_ref() {
                assert!(matches!(input.as_ref(), IrPlan::Source { .. }));
            }
        }
    }

    #[test]
    fn test_select_with_where() {
        let sql = "INSERT INTO output (name) SELECT name FROM users WHERE age > 18;";
        let result = parse_sql(sql);
        assert!(result.is_ok());
        if let Ok(program) = result {
            assert_eq!(program.pipelines.len(), 1);
            // Verify the pipeline has a Filter node
            let plan = &program.pipelines[0].plan;
            if let IrPlan::Map { input, .. } = plan.as_ref() {
                assert!(matches!(input.as_ref(), IrPlan::Filter { .. }));
            }
        }
    }

    #[test]
    fn test_select_with_join() {
        let sql = "INSERT INTO output (name, total) SELECT u.name, o.total FROM users u JOIN orders o ON u.id = o.user_id;";
        let result = parse_sql(sql);
        assert!(result.is_ok());
        if let Ok(program) = result {
            assert_eq!(program.pipelines.len(), 1);
            // Verify the pipeline has a Join node
            let plan = &program.pipelines[0].plan;
            if let IrPlan::Map { input, .. } = plan.as_ref() {
                assert!(matches!(input.as_ref(), IrPlan::Join { .. }));
            }
        }
    }

    #[test]
    fn test_select_with_aggregates() {
        let sql = "INSERT INTO output (country, count) SELECT country, COUNT(*) FROM users GROUP BY country;";
        let result = parse_sql(sql);
        assert!(result.is_ok());
        if let Ok(program) = result {
            assert_eq!(program.pipelines.len(), 1);
            // Verify the pipeline has a GroupBy node
            let plan = &program.pipelines[0].plan;
            assert!(matches!(plan.as_ref(), IrPlan::GroupBy { .. }));
        }
    }

    #[test]
    fn test_select_with_order_limit() {
        let sql = "INSERT INTO output (name) SELECT name FROM users ORDER BY name DESC LIMIT 10 OFFSET 5;";
        let result = parse_sql(sql);
        assert!(result.is_ok());
        if let Ok(program) = result {
            assert_eq!(program.pipelines.len(), 1);
            // Verify the pipeline has Limit and OrderBy nodes
            let plan = &program.pipelines[0].plan;
            assert!(matches!(plan.as_ref(), IrPlan::Limit { .. }));
        }
    }

    #[test]
    fn test_create_source() {
        let sql = "CREATE SOURCE users (id i64, name String, age i32) WITH (connector = 'kafka', topic = 'users');";
        let result = parse_sql(sql);
        assert!(result.is_ok());
        if let Ok(program) = result {
            assert_eq!(program.sources.len(), 1);
            assert_eq!(program.sources[0].name, "users");
            assert_eq!(program.sources[0].connector.connector_type, "kafka");
        }
    }

    #[test]
    fn test_create_sink() {
        let sql = "CREATE SINK output (id i64, result String) WITH (connector = 'postgres', table = 'results');";
        let result = parse_sql(sql);
        assert!(result.is_ok());
        if let Ok(program) = result {
            assert_eq!(program.sinks.len(), 1);
            assert_eq!(program.sinks[0].name, "output");
            assert_eq!(program.sinks[0].connector.connector_type, "postgres");
        }
    }

    #[test]
    fn test_insert_into() {
        let sql = "INSERT INTO results (id, value) SELECT user_id, COUNT(*) FROM orders GROUP BY user_id;";
        let result = parse_sql(sql);
        assert!(result.is_ok());
        if let Ok(program) = result {
            assert_eq!(program.pipelines.len(), 1);
            assert_eq!(program.pipelines[0].sink_name, "results");
            assert_eq!(program.pipelines[0].sink_columns.len(), 2);
        }
    }

    #[test]
    fn test_multi_statement_program() {
        let sql = r#"
            CREATE SOURCE users (id i64, name String) WITH (connector = 'kafka', topic = 'users');
            CREATE SINK output (id i64, name String) WITH (connector = 'postgres', table = 'output');
            INSERT INTO output (id, name) SELECT id, name FROM users WHERE id > 100;
        "#;
        let result = parse_sql(sql);
        assert!(result.is_ok());
        if let Ok(program) = result {
            assert_eq!(program.sources.len(), 1);
            assert_eq!(program.sinks.len(), 1);
            assert_eq!(program.pipelines.len(), 1);
            assert_eq!(program.sources[0].name, "users");
            assert_eq!(program.sinks[0].name, "output");
            assert_eq!(program.pipelines[0].sink_name, "output");
        }
    }

    #[test]
    fn test_subquery() {
        let sql = "INSERT INTO output (name) SELECT name FROM users WHERE id IN (SELECT user_id FROM active_users);";
        let result = parse_sql(sql);
        assert!(result.is_ok());
        if let Ok(program) = result {
            assert_eq!(program.pipelines.len(), 1);
        }
    }

    #[test]
    fn test_exists_clause() {
        let sql = "INSERT INTO output (name) SELECT name FROM users WHERE EXISTS (SELECT 1 FROM orders WHERE orders.user_id = users.id);";
        let result = parse_sql(sql);
        assert!(result.is_ok());
        if let Ok(program) = result {
            assert_eq!(program.pipelines.len(), 1);
        }
    }

    #[test]
    fn test_complex_where() {
        let sql = "INSERT INTO output SELECT * FROM users WHERE (age > 18 AND status = 'active') OR premium = true;";
        let result = parse_sql(sql);
        if let Err(ref e) = result {
            eprintln!("Parse error: {:?}", e);
        }
        assert!(result.is_ok());
        if let Ok(program) = result {
            assert_eq!(program.pipelines.len(), 1);
        }
    }
}
