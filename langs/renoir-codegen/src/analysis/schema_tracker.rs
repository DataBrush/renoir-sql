use renoir_ir::{ColumnRef, FieldDef, IrPlan, Program, SourceDef};
use std::collections::HashMap;
use std::sync::Arc;

/// Tracks column positions and schemas throughout query execution
#[derive(Debug, Clone)]
pub struct SchemaTracker {
    /// Maps source names to their schemas
    source_schemas: HashMap<String, Vec<FieldDef>>,
    /// Maps plan node pointers to their output schemas
    node_schemas: HashMap<usize, Vec<String>>,
}

impl SchemaTracker {
    pub fn new(program: &Program) -> Self {
        let mut source_schemas = HashMap::new();
        for source in &program.sources {
            source_schemas.insert(source.name.clone(), source.schema.clone());
        }

        SchemaTracker {
            source_schemas,
            node_schemas: HashMap::new(),
        }
    }

    /// Get the output schema (column names) for a plan node
    pub fn get_schema(&mut self, plan: &Arc<IrPlan>) -> Vec<String> {
        let node_id = Arc::as_ptr(plan) as usize;

        // Check cache first
        if let Some(schema) = self.node_schemas.get(&node_id) {
            return schema.clone();
        }

        // Compute schema based on plan type
        let schema = self.compute_schema(plan);
        self.node_schemas.insert(node_id, schema.clone());
        schema
    }

    fn compute_schema(&mut self, plan: &Arc<IrPlan>) -> Vec<String> {
        match plan.as_ref() {
            IrPlan::Source { source_name, .. } => {
                if let Some(source_schema) = self.source_schemas.get(source_name) {
                    source_schema.iter().map(|f| f.name.clone()).collect()
                } else {
                    vec![]
                }
            }

            IrPlan::Filter { input, .. } | IrPlan::Limit { input, .. } | IrPlan::Distinct { input } => {
                // These operations preserve the schema
                self.get_schema(input)
            }

            IrPlan::Map { input, projections } => {
                // Map changes the schema based on projections
                projections
                    .iter()
                    .enumerate()
                    .map(|(i, proj)| match proj {
                        renoir_ir::ProjectionColumn::Column(col_ref, alias) => {
                            alias.clone().unwrap_or_else(|| col_ref.column.clone())
                        }
                        renoir_ir::ProjectionColumn::Aggregate(_, alias) => {
                            alias.clone().unwrap_or_else(|| format!("agg_{}", i))
                        }
                        renoir_ir::ProjectionColumn::ComplexValue(_, alias) => {
                            alias.clone().unwrap_or_else(|| format!("expr_{}", i))
                        }
                        renoir_ir::ProjectionColumn::StringLiteral(_, alias) => {
                            alias.clone().unwrap_or_else(|| format!("literal_{}", i))
                        }
                        renoir_ir::ProjectionColumn::Subquery(_, alias) => {
                            alias.clone().unwrap_or_else(|| format!("subquery_{}", i))
                        }
                        renoir_ir::ProjectionColumn::SubqueryVec(name, alias) => {
                            alias.clone().unwrap_or_else(|| name.clone())
                        }
                    })
                    .collect()
            }

            IrPlan::Join { left, right, .. } => {
                // Join concatenates schemas from both sides
                let mut schema = self.get_schema(left);
                schema.extend(self.get_schema(right));
                schema
            }

            IrPlan::GroupBy {
                keys, aggregations, ..
            } => {
                // Output schema: group keys + aggregation results
                let mut schema: Vec<String> = keys.iter().map(|k| k.column.clone()).collect();

                for (i, proj) in aggregations.iter().enumerate() {
                    let col_name = match proj {
                        renoir_ir::ProjectionColumn::Aggregate(agg, alias) => alias
                            .clone()
                            .unwrap_or_else(|| format!("{:?}_{}", agg.function, i)),
                        _ => format!("col_{}", i),
                    };
                    schema.push(col_name);
                }
                schema
            }

            IrPlan::OrderBy { input, .. } => {
                // OrderBy preserves schema
                self.get_schema(input)
            }

            IrPlan::FlatMap { input, .. } => {
                // FlatMap - for now, preserve input schema
                // TODO: Implement proper schema inference for flatmap
                self.get_schema(input)
            }
        }
    }

    /// Get the column position for a column reference
    /// Returns None if column not found
    pub fn get_column_position(&mut self, plan: &Arc<IrPlan>, col_ref: &ColumnRef) -> Option<usize> {
        let schema = self.get_schema(plan);

        // Try to find by exact column name
        schema
            .iter()
            .position(|name| name == &col_ref.column)
            .or_else(|| {
                // If table is specified, try table.column format
                if let Some(table) = &col_ref.table {
                    let qualified_name = format!("{}.{}", table, col_ref.column);
                    schema.iter().position(|name| name == &qualified_name)
                } else {
                    None
                }
            })
    }

    /// Get all column names for a source
    pub fn get_source_columns(&self, source_name: &str) -> Option<Vec<String>> {
        self.source_schemas
            .get(source_name)
            .map(|schema| schema.iter().map(|f| f.name.clone()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renoir_ir::{ConnectorConfig, ConnectorOption, DataType, OptionValue, Pipeline};

    #[test]
    fn test_source_schema() {
        let program = Program {
            sources: vec![SourceDef {
                name: "users".to_string(),
                schema: vec![
                    FieldDef {
                        name: "id".to_string(),
                        data_type: DataType::Integer,
                    },
                    FieldDef {
                        name: "name".to_string(),
                        data_type: DataType::String,
                    },
                ],
                connector: ConnectorConfig {
                    connector_type: "csv".to_string(),
                    options: vec![],
                },
            }],
            sinks: vec![],
            pipelines: vec![],
        };

        let mut tracker = SchemaTracker::new(&program);
        let plan = Arc::new(IrPlan::Source {
            source_name: "users".to_string(),
            alias: None,
        });

        let schema = tracker.get_schema(&plan);
        assert_eq!(schema, vec!["id", "name"]);
    }

    #[test]
    fn test_column_position() {
        let program = Program {
            sources: vec![SourceDef {
                name: "users".to_string(),
                schema: vec![
                    FieldDef {
                        name: "id".to_string(),
                        data_type: DataType::Integer,
                    },
                    FieldDef {
                        name: "name".to_string(),
                        data_type: DataType::String,
                    },
                    FieldDef {
                        name: "email".to_string(),
                        data_type: DataType::String,
                    },
                ],
                connector: ConnectorConfig {
                    connector_type: "csv".to_string(),
                    options: vec![],
                },
            }],
            sinks: vec![],
            pipelines: vec![],
        };

        let mut tracker = SchemaTracker::new(&program);
        let plan = Arc::new(IrPlan::Source {
            source_name: "users".to_string(),
            alias: None,
        });

        let col_ref = ColumnRef {
            table: Some("users".to_string()),
            column: "email".to_string(),
        };

        let pos = tracker.get_column_position(&plan, &col_ref);
        assert_eq!(pos, Some(2));
    }

    #[test]
    fn test_filter_preserves_schema() {
        let program = Program {
            sources: vec![SourceDef {
                name: "users".to_string(),
                schema: vec![
                    FieldDef {
                        name: "id".to_string(),
                        data_type: DataType::Integer,
                    },
                    FieldDef {
                        name: "name".to_string(),
                        data_type: DataType::String,
                    },
                ],
                connector: ConnectorConfig {
                    connector_type: "csv".to_string(),
                    options: vec![],
                },
            }],
            sinks: vec![],
            pipelines: vec![],
        };

        let mut tracker = SchemaTracker::new(&program);
        let source = Arc::new(IrPlan::Source {
            source_name: "users".to_string(),
            alias: None,
        });

        let filter = Arc::new(IrPlan::Filter {
            input: source,
            predicate: renoir_ir::FilterClause::Base(renoir_ir::FilterConditionType::Boolean(
                true,
            )),
        });

        let schema = tracker.get_schema(&filter);
        assert_eq!(schema, vec!["id", "name"]);
    }
}
