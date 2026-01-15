use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;

/// Generate import statements based on connector types
pub fn generate_imports(connector_types: &HashSet<String>) -> TokenStream {
    let mut imports = vec![
        quote! { use serde::{Deserialize, Serialize}; },
    ];

    // Add connector-specific imports
    if connector_types.contains("kafka") {
        imports.push(quote! { use rdkafka::ClientConfig; });
    }

    quote! {
        #(#imports)*
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_basic_imports() {
        let connectors = HashSet::new();
        let imports = generate_imports(&connectors);
        let imports_str = imports.to_string();

        assert!(imports_str.contains("serde"));
    }

    #[test]
    fn test_generate_kafka_imports() {
        let mut connectors = HashSet::new();
        connectors.insert("kafka".to_string());

        let imports = generate_imports(&connectors);
        let imports_str = imports.to_string();

        assert!(imports_str.contains("rdkafka"));
    }
}
