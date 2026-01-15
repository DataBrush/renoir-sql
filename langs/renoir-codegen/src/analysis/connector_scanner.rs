use renoir_ir::{Program, SinkDef, SourceDef};
use std::collections::HashSet;

/// Scans a program for all connector types used
pub struct ConnectorScanner {
    connector_types: HashSet<String>,
}

impl ConnectorScanner {
    pub fn new() -> Self {
        Self {
            connector_types: HashSet::new(),
        }
    }

    /// Scan a program and collect all unique connector types
    pub fn scan_program(&mut self, program: &Program) -> HashSet<String> {
        // Scan sources
        for source in &program.sources {
            self.scan_source(source);
        }

        // Scan sinks
        for sink in &program.sinks {
            self.scan_sink(sink);
        }

        self.connector_types.clone()
    }

    /// Scan a source definition for connector type
    fn scan_source(&mut self, source: &SourceDef) {
        self.connector_types
            .insert(source.connector.connector_type.clone());
    }

    /// Scan a sink definition for connector type
    fn scan_sink(&mut self, sink: &SinkDef) {
        self.connector_types
            .insert(sink.connector.connector_type.clone());
    }

    /// Get all detected connector types
    pub fn connector_types(&self) -> &HashSet<String> {
        &self.connector_types
    }
}

impl Default for ConnectorScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renoir_ir::{ConnectorConfig, ConnectorOption, DataType, FieldDef, OptionValue};

    #[test]
    fn test_scan_empty_program() {
        let program = Program {
            sources: vec![],
            sinks: vec![],
            pipelines: vec![],
        };

        let mut scanner = ConnectorScanner::new();
        let connectors = scanner.scan_program(&program);

        assert_eq!(connectors.len(), 0);
    }

    #[test]
    fn test_scan_csv_source() {
        let program = Program {
            sources: vec![SourceDef {
                name: "users".to_string(),
                schema: vec![FieldDef {
                    name: "id".to_string(),
                    data_type: DataType::BigInt,
                }],
                connector: ConnectorConfig {
                    connector_type: "csv".to_string(),
                    options: vec![ConnectorOption {
                        key: "path".to_string(),
                        value: OptionValue::String("users.csv".to_string()),
                    }],
                },
            }],
            sinks: vec![],
            pipelines: vec![],
        };

        let mut scanner = ConnectorScanner::new();
        let connectors = scanner.scan_program(&program);

        assert_eq!(connectors.len(), 1);
        assert!(connectors.contains("csv"));
    }

    #[test]
    fn test_scan_multiple_connectors() {
        let program = Program {
            sources: vec![
                SourceDef {
                    name: "users".to_string(),
                    schema: vec![],
                    connector: ConnectorConfig {
                        connector_type: "csv".to_string(),
                        options: vec![],
                    },
                },
                SourceDef {
                    name: "events".to_string(),
                    schema: vec![],
                    connector: ConnectorConfig {
                        connector_type: "kafka".to_string(),
                        options: vec![],
                    },
                },
            ],
            sinks: vec![SinkDef {
                name: "output".to_string(),
                schema: vec![],
                connector: ConnectorConfig {
                    connector_type: "parquet".to_string(),
                    options: vec![],
                },
            }],
            pipelines: vec![],
        };

        let mut scanner = ConnectorScanner::new();
        let connectors = scanner.scan_program(&program);

        assert_eq!(connectors.len(), 3);
        assert!(connectors.contains("csv"));
        assert!(connectors.contains("kafka"));
        assert!(connectors.contains("parquet"));
    }

    #[test]
    fn test_deduplication() {
        let program = Program {
            sources: vec![
                SourceDef {
                    name: "users".to_string(),
                    schema: vec![],
                    connector: ConnectorConfig {
                        connector_type: "csv".to_string(),
                        options: vec![],
                    },
                },
                SourceDef {
                    name: "orders".to_string(),
                    schema: vec![],
                    connector: ConnectorConfig {
                        connector_type: "csv".to_string(),
                        options: vec![],
                    },
                },
            ],
            sinks: vec![],
            pipelines: vec![],
        };

        let mut scanner = ConnectorScanner::new();
        let connectors = scanner.scan_program(&program);

        assert_eq!(connectors.len(), 1);
        assert!(connectors.contains("csv"));
    }
}
