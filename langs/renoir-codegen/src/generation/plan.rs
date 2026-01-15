use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use renoir_ir::{
    AggregateFunction, AggregateType, BinaryOp, ColumnRef, ComparisonOp, ComplexField, Condition,
    ExistsCondition, FilterClause, FilterConditionType, GroupClause, InCondition, IrLiteral,
    IrPlan, JoinCondition, JoinType, NullCondition, NullOp, OrderByItem, OrderDirection,
    ProjectionColumn,
};
use std::collections::HashMap;
use std::sync::Arc;
/// Generate code for an IR plan node
/// This recursively generates the streaming pipeline code
pub fn generate_ir_plan(plan: &Arc<IrPlan>, ctx_name: &syn::Ident) -> TokenStream {
    match plan.as_ref() {
        IrPlan::Source {
            source_name,
            alias: _,
        } => {
            let source_var = format_ident!("{}_source", source_name);
            quote! { #source_var }
        }

        IrPlan::Filter { input, predicate } => {
            let input_code = generate_ir_plan(input, ctx_name);
            let filter_fn = generate_filter_predicate(predicate);

            quote! {
                #input_code.filter(#filter_fn)
            }
        }

        IrPlan::Map { input, projections } => {
            let input_code = generate_ir_plan(input, ctx_name);
            let map_fn = generate_map_projection(projections);

            quote! {
                #input_code.map(#map_fn)
            }
        }

        IrPlan::Limit { input, limit, offset } => {
            let input_code = generate_ir_plan(input, ctx_name);
            let limit_val = *limit as usize;

            // Use rich_filter_map with state to implement limit on any stream
            // This works around Renoir's requirement that .limit() only works on Source operators
            if let Some(offset_val) = offset {
                let offset_val = *offset_val as usize;
                quote! {
                    #input_code.rich_filter_map({
                        let mut count = 0usize;
                        move |item| {
                            let idx = count;
                            count += 1;
                            if idx >= #offset_val && idx < (#offset_val + #limit_val) {
                                Some(item)
                            } else {
                                None
                            }
                        }
                    })
                }
            } else {
                quote! {
                    #input_code.rich_filter_map({
                        let mut count = 0usize;
                        move |item| {
                            if count < #limit_val {
                                count += 1;
                                Some(item)
                            } else {
                                None
                            }
                        }
                    })
                }
            }
        }

        IrPlan::Distinct { input } => {
            let input_code = generate_ir_plan(input, ctx_name);
            quote! {
                #input_code.group_by(|item| item.clone()).reduce(|acc, _| acc)
            }
        }

        IrPlan::Join {
            left,
            right,
            condition,
            join_type,
        } => generate_join(left, right, condition, join_type, ctx_name),

        IrPlan::GroupBy {
            input,
            keys,
            aggregations,
            having,
        } => generate_group_by(input, keys, aggregations, having, ctx_name),

        IrPlan::OrderBy { input, items } => generate_order_by(input, items, ctx_name),

        IrPlan::FlatMap { input, projection } => {
            let input_code = generate_ir_plan(input, ctx_name);
            let projection_fn = generate_flat_map_projection(projection);
            quote! {
                #input_code.flat_map(#projection_fn)
            }
        }
    }
}

/// Generate filter predicate function
fn generate_filter_predicate(filter: &FilterClause) -> TokenStream {
    match filter {
        FilterClause::Base(cond_type) => generate_base_condition(cond_type),
        FilterClause::Expression {
            left,
            binary_op,
            right,
        } => {
            let left_code = generate_filter_predicate(left);
            let right_code = generate_filter_predicate(right);

            match binary_op {
                BinaryOp::And => quote! {
                    |item| (#left_code)(item) && (#right_code)(item)
                },
                BinaryOp::Or => quote! {
                    |item| (#left_code)(item) || (#right_code)(item)
                },
            }
        }
    }
}

/// Generate base filter condition
fn generate_base_condition(cond_type: &FilterConditionType) -> TokenStream {
    match cond_type {
        FilterConditionType::Comparison(cond) => generate_comparison(cond),
        FilterConditionType::NullCheck(null_cond) => generate_null_check(null_cond),
        FilterConditionType::Boolean(val) => {
            quote! { |_item| #val }
        }
        FilterConditionType::In(in_cond) => generate_in_condition(in_cond),
        FilterConditionType::Exists(exists_cond) => generate_exists_condition(exists_cond),
    }
}

/// Generate comparison condition
fn generate_comparison(cond: &Condition) -> TokenStream {
    let left = generate_field_access(&cond.left_field);
    let right = generate_field_access(&cond.right_field);
    let op = match cond.operator {
        ComparisonOp::Equal => quote! { == },
        ComparisonOp::NotEqual => quote! { != },
        ComparisonOp::GreaterThan => quote! { > },
        ComparisonOp::LessThan => quote! { < },
        ComparisonOp::GreaterThanEquals => quote! { >= },
        ComparisonOp::LessThanEquals => quote! { <= },
    };

    quote! {
        |item| #left #op #right
    }
}

/// Generate null check condition
fn generate_null_check(null_cond: &NullCondition) -> TokenStream {
    let field = generate_field_access(&null_cond.field);

    match null_cond.operator {
        NullOp::IsNull => quote! {
            |item| {
                let val = #field;
                // For Rust, we check if it's an Option and is None
                // This is a simplified version
                false
            }
        },
        NullOp::IsNotNull => quote! {
            |item| {
                let val = #field;
                true
            }
        },
    }
}

/// Generate field access expression
fn generate_field_access(field: &ComplexField) -> TokenStream {
    if let Some(ref col) = field.column_ref {
        let col_name = format_ident!("{}", col.column);
        quote! { item.#col_name }
    } else if let Some(ref lit) = field.literal {
        match lit {
            IrLiteral::Integer(n) => {
                // Generate integer literal without type suffix to let Rust infer the type
                let lit = proc_macro2::Literal::i64_unsuffixed(*n);
                quote! { #lit }
            }
            IrLiteral::Float(f) => quote! { #f },
            IrLiteral::String(s) => quote! { #s.to_string() },
            IrLiteral::Boolean(b) => quote! { #b },
        }
    } else {
        quote! { 0 } // Fallback
    }
}

/// Generate map projection function
fn generate_map_projection(projections: &[ProjectionColumn]) -> TokenStream {
    // For now, we'll generate a simple mapping
    // In Phase 3, we'll implement proper projection with struct creation

    if projections.len() == 1 {
        // Single column projection
        match &projections[0] {
            ProjectionColumn::Column(col_ref, _) => {
                let col_name = format_ident!("{}", col_ref.column);
                quote! { |item| item.#col_name }
            }
            _ => quote! { |item| item },
        }
    } else {
        // Multiple columns - for now, just return the whole item
        // In Phase 3, we'll create a new struct with selected fields
        quote! { |item| item }
    }
}

fn generate_flat_map_projection(projection: &ProjectionColumn) -> TokenStream {
    // FlatMap is used for operations that produce 0 or more results per input
    // For now, implement a simple version that returns an iterator
    match projection {
        ProjectionColumn::Column(col_ref, _) => {
            // Return the column value as a single-element iterator
            let col_name = format_ident!("{}", col_ref.column);
            quote! { |item| std::iter::once(item.#col_name) }
        }
        _ => {
            // Default: return the item as-is in a single-element iterator
            quote! { |item| std::iter::once(item) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renoir_ir::BinaryOp;

    #[test]
    fn test_generate_source() {
        let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
        let plan = Arc::new(IrPlan::Source {
            source_name: "users".to_string(),
            alias: None,
        });

        let code = generate_ir_plan(&plan, &ctx_name);
        let code_str = code.to_string();

        assert!(code_str.contains("users_source"));
    }

    #[test]
    fn test_generate_limit() {
        let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
        let source = Arc::new(IrPlan::Source {
            source_name: "users".to_string(),
            alias: None,
        });
        let plan = Arc::new(IrPlan::Limit {
            input: source,
            limit: 10,
            offset: None,
        });

        let code = generate_ir_plan(&plan, &ctx_name);
        let code_str = code.to_string();

        // After changes, limit is implemented using rich_filter_map
        assert!(code_str.contains("rich_filter_map"));
        assert!(code_str.contains("10"));
    }
}

/// Generate IN condition
fn generate_in_condition(in_cond: &InCondition) -> TokenStream {
    match in_cond {
        InCondition::Subquery {
            field,
            subquery,
            negated,
        } => {
            let field_access = generate_field_access(field);
            
            // Get the subquery variable name
            // We use the plan pointer as a unique identifier
            let subquery_ptr = Arc::as_ptr(subquery) as usize;
            let subquery_var = format_ident!("subquery_{}_data", subquery_ptr);
            
            if *negated {
                quote! {
                    move |item| !#subquery_var.contains(&#field_access)
                }
            } else {
                quote! {
                    move |item| #subquery_var.contains(&#field_access)
                }
            }
        }
        InCondition::Vec {
            field,
            vector_name,
            negated,
            ..
        } => {
            let field_access = generate_field_access(field);
            let vec_var = format_ident!("{}", vector_name);
            
            if *negated {
                quote! {
                    |item| !#vec_var.contains(&#field_access)
                }
            } else {
                quote! {
                    |item| #vec_var.contains(&#field_access)
                }
            }
        }
    }
}

/// Generate EXISTS condition
fn generate_exists_condition(exists_cond: &ExistsCondition) -> TokenStream {
    match exists_cond {
        ExistsCondition::Subquery { subquery, negated } => {
            // For EXISTS, we just check if the subquery returned any results
            let subquery_ptr = Arc::as_ptr(subquery) as usize;
            let subquery_var = format_ident!("subquery_{}_data", subquery_ptr);
            
            if *negated {
                quote! {
                    move |_item| #subquery_var.is_empty()
                }
            } else {
                quote! {
                    move |_item| !#subquery_var.is_empty()
                }
            }
        }
        ExistsCondition::Vec { vector_name, negated, .. } => {
            // For EXISTS with a vector variable
            let vec_var = format_ident!("{}", vector_name);
            
            if *negated {
                quote! {
                    |_item| #vec_var.is_empty()
                }
            } else {
                quote! {
                    |_item| !#vec_var.is_empty()
                }
            }
        }
    }
}
fn generate_join(
    left: &Arc<IrPlan>,
    right: &Arc<IrPlan>,
    conditions: &[JoinCondition],
    join_type: &JoinType,
    ctx_name: &syn::Ident,
) -> TokenStream {
    let left_code = generate_ir_plan(left, ctx_name);
    let right_code = generate_ir_plan(right, ctx_name);

    // Generate join key extraction functions
    let left_key_fn = generate_join_key_fn(conditions, true);
    let right_key_fn = generate_join_key_fn(conditions, false);

    match join_type {
        JoinType::Inner => {
            quote! {
                #left_code
                    .join(#right_code, #left_key_fn, #right_key_fn)
                    .map(|(left, right)| {
                        let mut result = left.clone();
                        result.extend(right.clone());
                        result
                    })
            }
        }
        JoinType::Left => {
            quote! {
                #left_code
                    .left_join(#right_code, #left_key_fn, #right_key_fn)
                    .map(|(left, right_opt)| {
                        let mut result = left.clone();
                        if let Some(right) = right_opt {
                            result.extend(right.clone());
                        } else {
                            // Pad with NULLs for missing right side
                            // TODO: determine right side column count
                            result.push("NULL".to_string());
                        }
                        result
                    })
            }
        }
        JoinType::Outer => {
            quote! {
                #left_code
                    .outer_join(#right_code, #left_key_fn, #right_key_fn)
                    .map(|(left_opt, right_opt)| {
                        let mut result = Vec::new();
                        if let Some(left) = left_opt {
                            result.extend(left.clone());
                        } else {
                            // Pad with NULLs for missing left side
                            result.push("NULL".to_string());
                        }
                        if let Some(right) = right_opt {
                            result.extend(right.clone());
                        } else {
                            // Pad with NULLs for missing right side
                            result.push("NULL".to_string());
                        }
                        result
                    })
            }
        }
    }
}

fn generate_join_key_fn(conditions: &[JoinCondition], is_left: bool) -> TokenStream {
    if conditions.len() == 1 {
        let col = if is_left {
            &conditions[0].left_col
        } else {
            &conditions[0].right_col
        };
        let column_access = generate_column_access(col);
        quote! {
            move |item: &Vec<String>| #column_access.clone()
        }
    } else {
        // Multi-column join key: create tuple
        let key_parts: Vec<_> = conditions
            .iter()
            .map(|cond| {
                let col = if is_left { &cond.left_col } else { &cond.right_col };
                generate_column_access(col)
            })
            .collect();

        quote! {
            move |item: &Vec<String>| (#(#key_parts.clone()),*)
        }
    }
}

fn generate_group_by(
    input: &Arc<IrPlan>,
    keys: &[ColumnRef],
    aggregations: &[ProjectionColumn],
    having: &Option<GroupClause>,
    ctx_name: &syn::Ident,
) -> TokenStream {
    let input_code = generate_ir_plan(input, ctx_name);

    // Extract AggregateFunction from ProjectionColumn::Aggregate variants
    let agg_functions: Vec<&AggregateFunction> = aggregations
        .iter()
        .filter_map(|proj| {
            if let ProjectionColumn::Aggregate(agg, _) = proj {
                Some(agg)
            } else {
                None
            }
        })
        .collect();

    // Generate group key extraction function
    let key_fn = if keys.len() == 1 {
        let col_access = generate_column_access(&keys[0]);
        quote! {
            move |item: &Vec<String>| #col_access.clone()
        }
    } else {
        let key_parts: Vec<_> = keys.iter().map(generate_column_access).collect();
        quote! {
            move |item: &Vec<String>| (#(#key_parts.clone()),*)
        }
    };

    // Generate aggregation logic
    let agg_init = generate_aggregation_init(&agg_functions);
    let agg_fold = generate_aggregation_fold(&agg_functions);
    let agg_result = generate_aggregation_result(&agg_functions, keys);

    let mut result = quote! {
        #input_code
            .group_by(#key_fn)
            .fold(
                #agg_init,
                #agg_fold
            )
            .map(#agg_result)
    };

    // Apply HAVING filter if present
    if let Some(having_clause) = having {
        let having_filter = generate_having_filter(having_clause);
        result = quote! {
            #result.filter(#having_filter)
        };
    }

    result
}

fn generate_aggregation_init(aggregations: &[&AggregateFunction]) -> TokenStream {
    // Initialize accumulator: (count, sums, maxs, mins)
    let init_values: Vec<_> = aggregations
        .iter()
        .map(|agg| match agg.function {
            AggregateType::Count => quote! { 0.0f64 },
            AggregateType::Sum | AggregateType::Avg => quote! { 0.0f64 },
            AggregateType::Max => quote! { f64::NEG_INFINITY },
            AggregateType::Min => quote! { f64::INFINITY },
        })
        .collect();

    quote! {
        || vec![#(#init_values),*]
    }
}

fn generate_aggregation_fold(aggregations: &[&AggregateFunction]) -> TokenStream {
    let fold_ops: Vec<_> = aggregations
        .iter()
        .enumerate()
        .map(|(i, agg)| {
            let idx = syn::Index::from(i);
            let column_access = generate_column_access(&agg.column);

            match agg.function {
                AggregateType::Count => quote! {
                    acc[#idx] += 1.0;
                },
                AggregateType::Sum | AggregateType::Avg => quote! {
                    if let Ok(val) = #column_access.parse::<f64>() {
                        acc[#idx] += val;
                    }
                },
                AggregateType::Max => quote! {
                    if let Ok(val) = #column_access.parse::<f64>() {
                        if val > acc[#idx] {
                            acc[#idx] = val;
                        }
                    }
                },
                AggregateType::Min => quote! {
                    if let Ok(val) = #column_access.parse::<f64>() {
                        if val < acc[#idx] {
                            acc[#idx] = val;
                        }
                    }
                },
            }
        })
        .collect();

    quote! {
        move |mut acc: Vec<f64>, item: &Vec<String>| {
            #(#fold_ops)*
            acc
        }
    }
}

fn generate_aggregation_result(
    aggregations: &[&AggregateFunction],
    keys: &[ColumnRef],
) -> TokenStream {
    let result_parts: Vec<_> = aggregations
        .iter()
        .enumerate()
        .map(|(i, agg)| {
            let idx = syn::Index::from(i);
            match agg.function {
                AggregateType::Avg => quote! {
                    if agg_values[0] > 0.0 {
                        (agg_values[#idx] / agg_values[0]).to_string()
                    } else {
                        "0".to_string()
                    }
                },
                _ => quote! {
                    agg_values[#idx].to_string()
                },
            }
        })
        .collect();

    if keys.is_empty() {
        // No grouping keys, just return aggregates
        quote! {
            move |(_key, agg_values): ((), Vec<f64>)| {
                vec![#(#result_parts),*]
            }
        }
    } else if keys.len() == 1 {
        // Single grouping key
        quote! {
            move |(key, agg_values): (String, Vec<f64>)| {
                let mut result = vec![key];
                result.extend(vec![#(#result_parts),*]);
                result
            }
        }
    } else {
        // Multiple grouping keys (tuple)
        let key_count = keys.len();
        let key_indices: Vec<_> = (0..key_count).map(syn::Index::from).collect();
        quote! {
            move |(key_tuple, agg_values): (_, Vec<f64>)| {
                let mut result = vec![#(key_tuple.#key_indices),*];
                result.extend(vec![#(#result_parts),*]);
                result
            }
        }
    }
}

fn generate_having_filter(having: &GroupClause) -> TokenStream {
    // For now, implement basic HAVING support
    // TODO: Full expression evaluation
    quote! {
        |_item: &Vec<String>| true
    }
}

fn generate_order_by(
    input: &Arc<IrPlan>,
    items: &[OrderByItem],
    ctx_name: &syn::Ident,
) -> TokenStream {
    let input_code = generate_ir_plan(input, ctx_name);

    // ORDER BY requires collecting all data for global sort
    // This is expensive in distributed mode
    let sort_key_fn = generate_sort_key_fn(items);

    quote! {
        {
            let mut collected = #input_code.collect_vec();
            collected.sort_by(#sort_key_fn);
            env.stream_iter(collected)
        }
    }
}

fn generate_sort_key_fn(items: &[OrderByItem]) -> TokenStream {
    // Generate comparison function for sorting
    let comparisons: Vec<_> = items
        .iter()
        .map(|item| {
            let col_access_a = generate_column_access_with_var(&item.column, "a");
            let col_access_b = generate_column_access_with_var(&item.column, "b");
            
            match item.direction {
                OrderDirection::Asc => quote! {
                    match #col_access_a.cmp(&#col_access_b) {
                        std::cmp::Ordering::Equal => {},
                        other => return other,
                    }
                },
                OrderDirection::Desc => quote! {
                    match #col_access_b.cmp(&#col_access_a) {
                        std::cmp::Ordering::Equal => {},
                        other => return other,
                    }
                },
            }
        })
        .collect();

    quote! {
        |a: &Vec<String>, b: &Vec<String>| {
            #(#comparisons)*
            std::cmp::Ordering::Equal
        }
    }
}

fn generate_column_access_with_var(col: &ColumnRef, var_name: &str) -> TokenStream {
    let var = format_ident!("{}", var_name);
    // For now, use positional access based on column name
    // TODO: Track actual column positions from schema
    quote! { &#var[0] }
}

fn generate_column_access(col: &ColumnRef) -> TokenStream {
    // For now, use positional access based on column name
    // TODO: Track actual column positions from schema
    quote! { &item[0] }
}