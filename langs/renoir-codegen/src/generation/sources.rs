use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use renoir_ir::{DataType, FieldDef, OptionValue, SourceDef};
use crate::utils::NameGenerator;

/// Generate source definition code
pub fn generate_sources(sources: &[SourceDef], ctx_name: &syn::Ident) -> TokenStream {
    let mut name_gen = NameGenerator::new();
    
    // First, generate struct definitions for each source
    let struct_defs: Vec<TokenStream> = sources
        .iter()
        .map(|source| generate_source_struct(source))
        .collect();
    
    // Then, generate source initialization code
    let source_inits: Vec<TokenStream> = sources
        .iter()
        .map(|source| generate_source_init(source, ctx_name, &mut name_gen))
        .collect();

    quote! {
        #(#struct_defs)*
        #(#source_inits)*
    }
}

/// Generate a struct definition for a source
fn generate_source_struct(source: &SourceDef) -> TokenStream {
    let struct_name = format_ident!("{}", capitalize(&source.name));
    
    let fields: Vec<TokenStream> = source.schema.iter().map(|field| {
        let field_name = format_ident!("{}", field.name);
        let field_type = rust_type(&field.data_type);
        quote! {
            pub #field_name: #field_type
        }
    }).collect();

    quote! {
        #[derive(Debug, Clone, Deserialize, Serialize)]
        struct #struct_name {
            #(#fields),*
        }
    }
}

/// Generate source initialization code based on connector type
fn generate_source_init(
    source: &SourceDef,
    ctx_name: &syn::Ident,
    name_gen: &mut NameGenerator,
) -> TokenStream {
    let var_name = format_ident!("{}", name_gen.source(&source.name));
    let struct_name = format_ident!("{}", capitalize(&source.name));
    
    match source.connector.connector_type.as_str() {
        "csv" => generate_csv_source(source, &var_name, &struct_name, ctx_name),
        "kafka" => generate_kafka_source(source, &var_name, &struct_name, ctx_name),
        _ => {
            // Fallback for unknown connector types
            quote! {
                // Unknown connector type: #source.connector.connector_type
            }
        }
    }
}

/// Generate CSV source
fn generate_csv_source(
    source: &SourceDef,
    var_name: &syn::Ident,
    struct_name: &syn::Ident,
    ctx_name: &syn::Ident,
) -> TokenStream {
    let path = get_option_value(&source.connector.options, "path")
        .unwrap_or_else(|| "data.csv".to_string());

    // Build CSV source with configuration options
    let mut config_calls = Vec::new();
    
    // has_headers (default: true)
    if let Some(has_headers) = get_option_value(&source.connector.options, "has_headers") {
        let has_headers_val: bool = has_headers.parse().unwrap_or(true);
        config_calls.push(quote! { .has_headers(#has_headers_val) });
    }
    
    // delimiter (default: ',')
    if let Some(delimiter) = get_option_value(&source.connector.options, "delimiter") {
        if let Some(ch) = delimiter.chars().next() {
            let byte_val = ch as u8;
            config_calls.push(quote! { .delimiter(#byte_val) });
        }
    }
    
    // comment (optional)
    if let Some(comment) = get_option_value(&source.connector.options, "comment") {
        if let Some(ch) = comment.chars().next() {
            let byte_val = ch as u8;
            config_calls.push(quote! { .comment(Some(#byte_val)) });
        }
    }
    
    // quote (default: '"')
    if let Some(quote_char) = get_option_value(&source.connector.options, "quote") {
        if let Some(ch) = quote_char.chars().next() {
            let byte_val = ch as u8;
            config_calls.push(quote! { .quote(#byte_val) });
        }
    }
    
    // escape (optional)
    if let Some(escape) = get_option_value(&source.connector.options, "escape") {
        if let Some(ch) = escape.chars().next() {
            let byte_val = ch as u8;
            config_calls.push(quote! { .escape(Some(#byte_val)) });
        }
    }
    
    // double_quote (default: true)
    if let Some(double_quote) = get_option_value(&source.connector.options, "double_quote") {
        let double_quote_val: bool = double_quote.parse().unwrap_or(true);
        config_calls.push(quote! { .double_quote(#double_quote_val) });
    }
    
    // flexible (default: false)
    if let Some(flexible) = get_option_value(&source.connector.options, "flexible") {
        let flexible_val: bool = flexible.parse().unwrap_or(false);
        config_calls.push(quote! { .flexible(#flexible_val) });
    }
    
    // quoting (default: true)
    if let Some(quoting) = get_option_value(&source.connector.options, "quoting") {
        let quoting_val: bool = quoting.parse().unwrap_or(true);
        config_calls.push(quote! { .quoting(#quoting_val) });
    }
    
    // terminator (optional - requires special handling)
    if let Some(terminator) = get_option_value(&source.connector.options, "terminator") {
        match terminator.as_str() {
            "\\n" | "\n" => {
                config_calls.push(quote! { .terminator(csv::Terminator::Any(b'\n')) });
            }
            "\\r\\n" | "\r\n" | "CRLF" => {
                config_calls.push(quote! { .terminator(csv::Terminator::CRLF) });
            }
            _ => {
                if let Some(ch) = terminator.chars().next() {
                    let byte_val = ch as u8;
                    config_calls.push(quote! { .terminator(csv::Terminator::Any(#byte_val)) });
                }
            }
        }
    }
    
    // trim (optional - requires special handling)
    if let Some(trim) = get_option_value(&source.connector.options, "trim") {
        match trim.to_lowercase().as_str() {
            "headers" => {
                config_calls.push(quote! { .trim(csv::Trim::Headers) });
            }
            "fields" => {
                config_calls.push(quote! { .trim(csv::Trim::Fields) });
            }
            "all" => {
                config_calls.push(quote! { .trim(csv::Trim::All) });
            }
            _ => {}
        }
    }

    quote! {
        let source = CsvSource::<#struct_name>::new(#path)
            #(#config_calls)*;
        let #var_name = #ctx_name.stream(source);
    }
}

/// Generate Kafka source
fn generate_kafka_source(
    source: &SourceDef,
    var_name: &syn::Ident,
    struct_name: &syn::Ident,
    ctx_name: &syn::Ident,
) -> TokenStream {
    let brokers = get_option_value(&source.connector.options, "brokers")
        .unwrap_or_else(|| "localhost:9092".to_string());
    let topic = get_option_value(&source.connector.options, "topic")
        .unwrap_or_else(|| "events".to_string());
    let group_id = get_option_value(&source.connector.options, "group_id")
        .unwrap_or_else(|| "consumer_group".to_string());

    quote! {
        let mut consumer_config = ClientConfig::new();
        consumer_config
            .set("group.id", #group_id)
            .set("bootstrap.servers", #brokers)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true");

        let #var_name = #ctx_name.stream_kafka::<#struct_name>(
            consumer_config,
            &[#topic],
            renoir::operator::source::kafka::Replication::Unlimited
        );
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
        DataType::Timestamp => quote! { i64 }, // Unix timestamp
    }
}
