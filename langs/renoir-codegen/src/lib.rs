use proc_macro2::TokenStream;
use quote::quote;
use renoir_ir::Program;
use syn::Ident;

// Phase 1: Core Infrastructure
mod analysis;
mod connectors;
mod generation;
mod utils;

use analysis::{ConnectorScanner, DependencyGraph, SchemaTracker, SubqueryDetector, SubqueryInfo};
use connectors::ConnectorRegistry;
use generation::{
    generate_imports, generate_pipelines, generate_sinks, generate_sources,
    generate_subquery_execution,
};
use utils::NameGenerator;

/// Generate Renoir code from a Program IR using a provided context
/// 
/// This is the main entry point for code generation. It follows this process:
/// 1. Analysis Phase: Scan for connectors and subqueries, build dependency graph
/// 2. Import Generation: Generate necessary use statements
/// 3. Source/Sink Definition: Generate source and sink setup code
/// 4. Subquery Execution: Generate code to execute subqueries in correct order
/// 5. Main Pipeline: Generate the main query pipeline
pub fn generate_program_with_context(program: &Program, ctx_name: &Ident) -> TokenStream {
    // Phase 1: Analysis
    let mut connector_scanner = ConnectorScanner::new();
    let connector_types = connector_scanner.scan_program(program);

    let mut subquery_detector = SubqueryDetector::new();
    let subqueries = subquery_detector.detect_in_program(program);

    let _dependency_graph = DependencyGraph::build(&subqueries);
    let _registry = ConnectorRegistry::new();
    let _name_gen = NameGenerator::new();

    // Phase 2: Code Generation
    let imports = generate_imports(&connector_types);
    let sources = generate_sources(&program.sources, ctx_name);
    let sinks = generate_sinks(&program.sinks);
    
    // Phase 3: Subquery Execution (if any)
    let subquery_exec = generate_subquery_execution(&subqueries, ctx_name);
    
    // Main Pipeline
    let pipelines = generate_pipelines(&program.pipelines, program, ctx_name);

    quote! {
        {
            #imports
            #sources
            #sinks
            #subquery_exec
            #pipelines
        }
    }
}

/// Legacy function that creates its own context (kept for backward compatibility)
pub fn generate_program(program: &Program) -> TokenStream {
    let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());
    
    let program_code = generate_program_with_context(program, &ctx_name);
    
    quote! {
        {
            use renoir::prelude::*;
            
            let config = RuntimeConfig::local(4).unwrap();
            let ctx = StreamContext::new(config);
            
            #program_code
            
            ctx.execute_blocking();
        }
    }
}