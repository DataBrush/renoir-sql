use proc_macro2::TokenStream;
use quote::quote;
use renoir_ir::{Pipeline, Program};

use super::plan::generate_ir_plan;
use super::sinks::generate_sink_operation;

/// Generate code for all pipelines
pub fn generate_pipelines(
    pipelines: &[Pipeline],
    program: &Program,
    ctx_name: &syn::Ident,
) -> TokenStream {
    let pipeline_code: Vec<TokenStream> = pipelines
        .iter()
        .map(|pipeline| generate_pipeline(pipeline, program, ctx_name))
        .collect();

    quote! {
        #(#pipeline_code)*
    }
}

/// Generate code for a single pipeline
/// A pipeline flows from source through transformations to sink
fn generate_pipeline(pipeline: &Pipeline, program: &Program, ctx_name: &syn::Ident) -> TokenStream {
    // Generate the transformation pipeline
    let plan_code = generate_ir_plan(&pipeline.plan, ctx_name);

    // Find the corresponding sink definition
    let sink = program.sinks.iter().find(|s| s.name == pipeline.sink_name);

    // Generate the sink operation
    let pipeline_with_sink = if let Some(sink_def) = sink {
        generate_sink_operation(sink_def, plan_code)
    } else {
        // Fallback: if no sink found, just print
        quote! {
            #plan_code.for_each(|item| println!("{:?}", item));
        }
    };

    pipeline_with_sink
}

#[cfg(test)]
mod tests {
    use super::*;
    use renoir_ir::{ConnectorConfig, ConnectorOption, FieldDef, IrPlan, OptionValue, SinkDef};
    use std::sync::Arc;

    #[test]
    fn test_generate_simple_pipeline() {
        let ctx_name = syn::Ident::new("ctx", proc_macro2::Span::call_site());

        let program = Program {
            sources: vec![],
            sinks: vec![SinkDef {
                name: "output".to_string(),
                schema: vec![],
                connector: ConnectorConfig {
                    connector_type: "csv".to_string(),
                    options: vec![ConnectorOption {
                        key: "path".to_string(),
                        value: OptionValue::String("output.csv".to_string()),
                    }],
                },
            }],
            pipelines: vec![Pipeline {
                sink_name: "output".to_string(),
                sink_columns: vec![],
                plan: Arc::new(IrPlan::Source {
                    source_name: "users".to_string(),
                    alias: None,
                }),
            }],
        };

        let code = generate_pipelines(&program.pipelines, &program, &ctx_name);
        let code_str = code.to_string();

        assert!(code_str.contains("users_source"));
        assert!(code_str.contains("write_csv_seq"));
    }
}
