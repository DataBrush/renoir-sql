use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;

/// Generate import statements based on connector types
/// This function deduplicates imports and only generates what's needed
pub fn generate_imports(connector_types: &HashSet<String>) -> TokenStream {
    let mut has_serde = false;
    let mut has_kafka = false;
    let mut has_csv = false;

    // Determine which imports are needed
    for connector_type in connector_types {
        match connector_type.as_str() {
            "csv" => {
                has_serde = true;
                has_csv = true;
            }
            "kafka" => {
                has_serde = true;
                has_kafka = true;
            }
            "parquet" => {
                // Parquet may need serde depending on schema
                has_serde = true;
            }
            _ => {}
        }
    }

    let mut imports = Vec::new();

    // Add serde imports if needed
    if has_serde {
        imports.push(quote! {
            use serde::{Deserialize, Serialize};
        });
    }

    // Add CSV imports if needed
    if has_csv {
        imports.push(quote! {
            use renoir::operator::source::CsvSource;
        });
    }

    // Add Kafka imports if needed
    if has_kafka {
        imports.push(quote! {
            use rdkafka::ClientConfig;
        });
    }

    quote! {
        #(#imports)*
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_no_imports() {
        let connectors = HashSet::new();
        let imports = generate_imports(&connectors);
        let imports_str = imports.to_string();

        // No connectors means no imports
        assert!(imports_str.is_empty() || imports_str.trim().is_empty());
    }

    #[test]
    fn test_generate_csv_imports() {
        let mut connectors = HashSet::new();
        connectors.insert("csv".to_string());

        let imports = generate_imports(&connectors);
        let imports_str = imports.to_string();

        assert!(imports_str.contains("serde"));
        assert!(imports_str.contains("Deserialize"));
        assert!(imports_str.contains("Serialize"));
    }

    #[test]
    fn test_generate_kafka_imports() {
        let mut connectors = HashSet::new();
        connectors.insert("kafka".to_string());

        let imports = generate_imports(&connectors);
        let imports_str = imports.to_string();

        assert!(imports_str.contains("serde"));
        assert!(imports_str.contains("rdkafka"));
    }

    #[test]
    fn test_deduplication() {
        let mut connectors = HashSet::new();
        connectors.insert("csv".to_string());
        connectors.insert("kafka".to_string());

        let imports = generate_imports(&connectors);
        let imports_str = imports.to_string();

        // Should have serde only once
        let serde_count = imports_str.matches("use serde").count();
        assert_eq!(serde_count, 1);
    }
}
