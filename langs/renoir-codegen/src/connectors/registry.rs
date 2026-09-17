use std::collections::HashMap;

/// Metadata for a connector type
#[derive(Debug, Clone)]
pub struct ConnectorMeta {
    /// Name of the connector (e.g., "csv", "kafka")
    pub name: String,
    /// Import paths needed for this connector
    pub imports: Vec<&'static str>,
    /// Whether this connector requires serde traits
    pub requires_serde: bool,
}

/// Registry of all available connectors
pub struct ConnectorRegistry {
    connectors: HashMap<String, ConnectorMeta>,
}

impl ConnectorRegistry {
    /// Create a new connector registry with default connectors
    pub fn new() -> Self {
        let mut registry = Self {
            connectors: HashMap::new(),
        };

        // Register default connectors
        registry.register_csv();
        registry.register_kafka();
        registry.register_parquet();
        registry.register_file();
        registry.register_iter();

        registry
    }

    /// Register CSV connector
    fn register_csv(&mut self) {
        self.connectors.insert(
            "csv".to_string(),
            ConnectorMeta {
                name: "csv".to_string(),
                imports: vec![],
                requires_serde: true, // CSV uses serde for deserialization
            },
        );
    }

    /// Register Kafka connector
    fn register_kafka(&mut self) {
        self.connectors.insert(
            "kafka".to_string(),
            ConnectorMeta {
                name: "kafka".to_string(),
                imports: vec!["renoir::kafka::{Message, KafkaSourceExt, KafkaSinkExt, ClientConfig}"],
                requires_serde: true,
            },
        );
    }

    /// Register Parquet connector
    fn register_parquet(&mut self) {
        self.connectors.insert(
            "parquet".to_string(),
            ConnectorMeta {
                name: "parquet".to_string(),
                imports: vec![],
                requires_serde: false,
            },
        );
    }

    /// Register file connector (for text files)
    fn register_file(&mut self) {
        self.connectors.insert(
            "file".to_string(),
            ConnectorMeta {
                name: "file".to_string(),
                imports: vec![],
                requires_serde: false,
            },
        );
    }

    /// Register iterator connector (for in-memory data)
    fn register_iter(&mut self) {
        self.connectors.insert(
            "iter".to_string(),
            ConnectorMeta {
                name: "iter".to_string(),
                imports: vec![],
                requires_serde: false,
            },
        );
    }

    /// Get metadata for a connector type
    pub fn get(&self, connector_type: &str) -> Option<&ConnectorMeta> {
        self.connectors.get(connector_type)
    }

    /// Check if a connector type is registered
    pub fn has(&self, connector_type: &str) -> bool {
        self.connectors.contains_key(connector_type)
    }

    /// Get all registered connector types
    pub fn all_types(&self) -> Vec<String> {
        self.connectors.keys().cloned().collect()
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_defaults() {
        let registry = ConnectorRegistry::new();

        assert!(registry.has("csv"));
        assert!(registry.has("kafka"));
        assert!(registry.has("parquet"));
        assert!(registry.has("file"));
        assert!(registry.has("iter"));
    }

    #[test]
    fn test_get_connector_meta() {
        let registry = ConnectorRegistry::new();

        let csv = registry.get("csv").unwrap();
        assert_eq!(csv.name, "csv");
        assert!(csv.requires_serde);

        let kafka = registry.get("kafka").unwrap();
        assert_eq!(kafka.name, "kafka");
        assert!(kafka.requires_serde);
        assert!(!kafka.imports.is_empty());
    }

    #[test]
    fn test_unknown_connector() {
        let registry = ConnectorRegistry::new();

        assert!(!registry.has("unknown"));
        assert!(registry.get("unknown").is_none());
    }
}
