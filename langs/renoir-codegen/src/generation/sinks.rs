use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use renoir_ir::{DataType, FieldDef, OptionValue, SinkDef};

/// Generate sink definition code
/// Note: In Renoir, sinks are terminal operations, not separate objects
/// This function generates struct definitions for sink types
pub fn generate_sinks(sinks: &[SinkDef]) -> TokenStream {
    let sink_structs: Vec<TokenStream> = sinks
        .iter()
        .map(generate_sink_struct)
        .collect();

    quote! {
        #(#sink_structs)*
    }
}

/// Generate a struct definition for a sink
fn generate_sink_struct(sink: &SinkDef) -> TokenStream {
    let struct_name = format_ident!("{}", capitalize(&sink.name));
    
    let fields: Vec<TokenStream> = sink.schema.iter().map(|field| {
        let field_name = format_ident!("{}", field.name);
        let field_type = rust_type(&field.data_type);
        quote! {
            pub #field_name: #field_type
        }
    }).collect();

    quote! {
        #[derive(Debug, Clone, Serialize)]
        struct #struct_name {
            #(#fields),*
        }
    }
}

/// Generate sink terminal operation code
/// This is called from pipeline generation to add the sink operation
pub fn generate_sink_operation(
    sink: &SinkDef,
    pipeline_expr: TokenStream,
) -> TokenStream {
    match sink.connector.connector_type.as_str() {
        "csv" => generate_csv_sink(sink, pipeline_expr),
        "kafka" => generate_kafka_sink(sink, pipeline_expr),
        _ => {
            // Fallback: just print to console
            quote! {
                #pipeline_expr.for_each(|item| println!("{:?}", item));
            }
        }
    }
}

/// Generate CSV sink operation
fn generate_csv_sink(sink: &SinkDef, pipeline_expr: TokenStream) -> TokenStream {
    let path = get_option_value(&sink.connector.options, "path")
        .unwrap_or_else(|| "output.csv".to_string());

    quote! {
        #pipeline_expr.write_csv_seq(std::path::PathBuf::from(#path), false);
    }
}

/// Generate Kafka sink operation
fn generate_kafka_sink(sink: &SinkDef, pipeline_expr: TokenStream) -> TokenStream {
    let brokers = get_option_value(&sink.connector.options, "brokers")
        .unwrap_or_else(|| "localhost:9092".to_string());
    let topic = get_option_value(&sink.connector.options, "topic")
        .unwrap_or_else(|| "output".to_string());

    quote! {
        {
            let mut producer_config = ClientConfig::new();
            producer_config
                .set("bootstrap.servers", #brokers)
                .set("message.timeout.ms", "5000");

            #pipeline_expr
                .map(|record| format!("{:?}", record))
                .write_kafka(producer_config, #topic);
        }
    }
}

/// Helper: Get option value from connector options
fn get_option_value(options: &[renoir_ir::ConnectorOption], key: &str) -> Option<String> {
    options
        .iter()
        .find(|opt| opt.key == key)
        .and_then(|opt| match &opt.value {
            OptionValue::String(s) => Some(s.clone()),
            _ => None,
        })
}

/// Helper: Capitalize first letter
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(chars).collect(),
    }
}

/// Helper: Convert DataType to Rust type TokenStream
fn rust_type(data_type: &DataType) -> TokenStream {
    match data_type {
        DataType::Integer => quote! { i32 },
        DataType::BigInt => quote! { i64 },
        DataType::Float => quote! { f32 },
        DataType::Double => quote! { f64 },
        DataType::String => quote! { String },
        DataType::Boolean => quote! { bool },
        DataType::Timestamp => quote! { i64 },
    }
}
