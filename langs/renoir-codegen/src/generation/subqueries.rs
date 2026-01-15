use crate::analysis::{DependencyGraph, SubqueryInfo};
use crate::generation::plan::generate_ir_plan;
use crate::utils::NameGenerator;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use renoir_ir::IrPlan;
use std::collections::HashMap;

/// Generate code for executing subqueries
/// Subqueries are executed in dependency order, with ctx.execute_blocking() between stages
pub fn generate_subquery_execution(
    subqueries: &[SubqueryInfo],
    ctx_name: &syn::Ident,
) -> TokenStream {
    if subqueries.is_empty() {
        return quote! {};
    }

    // Build dependency graph and get execution order
    let mut dep_graph = DependencyGraph::new();
    for subquery in subqueries {
        dep_graph.add_node(subquery.id, subquery.dependencies.clone());
    }

    let execution_order = dep_graph
        .topological_sort()
        .expect("Circular dependency detected in subqueries");

    // Generate code for each subquery in execution order
    let mut subquery_code = Vec::new();
    let mut name_gen = NameGenerator::new();

    // Map from subquery ID to result variable name
    let mut subquery_vars: HashMap<usize, syn::Ident> = HashMap::new();

    for &subquery_id in &execution_order {
        let subquery = subqueries
            .iter()
            .find(|s| s.id == subquery_id)
            .expect("Subquery not found");

        let result_var = format_ident!("subquery_{}_result", subquery_id);
        let data_var = format_ident!("subquery_{}_data", subquery_id);

        subquery_vars.insert(subquery_id, data_var.clone());

        // Generate the pipeline for this subquery
        let pipeline_code = generate_ir_plan(&subquery.plan, ctx_name);

        // Generate execution code with collect_all() for distributed correctness
        let subquery_exec = quote! {
            // Execute subquery #subquery_id
            let #result_var = #pipeline_code.collect_all();
            
            #ctx_name.execute_blocking();
            
            let #data_var = #result_var.get().unwrap();
        };

        subquery_code.push(subquery_exec);
    }

    quote! {
        #(#subquery_code)*
    }
}

/// Get the variable name for a subquery result
/// This is used when generating references to subquery results in filters
pub fn get_subquery_var_name(subquery_id: usize) -> syn::Ident {
    format_ident!("subquery_{}_data", subquery_id)
}

/// Check if a plan contains a specific subquery
pub fn plan_contains_subquery(plan: &IrPlan, target_plan: &IrPlan) -> bool {
    // Compare using pointer equality
    std::ptr::eq(plan, target_plan)
}
