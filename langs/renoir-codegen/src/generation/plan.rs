use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use renoir_ir::{
    BinaryOp, ComparisonOp, ComplexField, Condition, FilterClause, FilterConditionType,
    IrLiteral, IrPlan, NullCondition, NullOp, ProjectionColumn,
};
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

        // Placeholder for other operations (Phase 3)
        _ => {
            quote! {
                // TODO: Implement other plan nodes
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
        // Placeholders for IN and EXISTS (will be implemented in Phase 3 with subqueries)
        FilterConditionType::In(_) => quote! { |_item| true },
        FilterConditionType::Exists(_) => quote! { |_item| true },
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
