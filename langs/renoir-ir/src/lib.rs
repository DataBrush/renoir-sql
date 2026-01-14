// New IrAST structure following Polars approach
use std::sync::Arc;

pub mod builder;

#[derive(Debug, Clone, PartialEq)]
pub enum IrPlan {
    // Source operations
    Scan {
        input: Arc<IrPlan>,
        stream_name: String,
        alias: Option<String>,
    },

    // Transformation operations
    Filter {
        input: Arc<IrPlan>,
        predicate: FilterClause,
    },

    Project {
        input: Arc<IrPlan>,
        columns: Vec<ProjectionColumn>,
        distinct: bool,
    },

    GroupBy {
        input: Arc<IrPlan>,
        keys: Vec<ColumnRef>,
        group_condition: Option<GroupClause>,
    },

    Join {
        left: Arc<IrPlan>,
        right: Arc<IrPlan>,
        condition: Vec<JoinCondition>,
        join_type: JoinType,
    },

    OrderBy {
        input: Arc<IrPlan>,
        items: Vec<OrderByItem>,
    },

    Limit {
        input: Arc<IrPlan>,
        limit: i64,
        offset: Option<i64>,
    },
    Table {
        table_name: String,
    },
}

// Supporting structures
#[derive(Debug, PartialEq, Clone)]
pub enum FilterClause {
    Base(FilterConditionType),
    Expression {
        left: Box<FilterClause>,
        binary_op: BinaryOp,
        right: Box<FilterClause>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum FilterConditionType {
    Comparison(Condition),
    NullCheck(NullCondition),
    In(InCondition),
    Exists(ExistsCondition),
    Boolean(bool),
}

#[derive(Debug, PartialEq, Clone)]
pub enum GroupClause {
    Base(GroupBaseCondition),
    Expression {
        left: Box<GroupClause>,
        op: BinaryOp,
        right: Box<GroupClause>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum GroupBaseCondition {
    Comparison(Condition),
    NullCheck(NullCondition),
    In(InCondition),
    Exists(ExistsCondition),
    Boolean(bool),
}

#[derive(Debug, PartialEq, Clone)]
pub struct JoinCondition {
    pub left_col: ColumnRef,
    pub right_col: ColumnRef,
}

#[derive(Debug, PartialEq, Clone)]
pub enum JoinType {
    Inner,
    Left,
    Outer,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ProjectionColumn {
    Column(ColumnRef, Option<String>),
    Aggregate(AggregateFunction, Option<String>),
    ComplexValue(ComplexField, Option<String>),
    StringLiteral(String, Option<String>),
    Subquery(Arc<IrPlan>, Option<String>),
    SubqueryVec(String, Option<String>), // name of the result vec and optional alias
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct ColumnRef {
    pub table: Option<String>,
    pub column: String,
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct AggregateFunction {
    pub function: AggregateType,
    pub column: ColumnRef,
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub enum AggregateType {
    Max,
    Min,
    Avg,
    Count,
    Sum,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ComplexField {
    pub column_ref: Option<ColumnRef>,
    pub literal: Option<IrLiteral>,
    pub aggregate: Option<AggregateFunction>,
    pub nested_expr: Option<Box<(ComplexField, String, ComplexField, bool)>>, //bool true if parenthesized
    pub subquery: Option<Arc<IrPlan>>,
    pub subquery_vec: Option<(String, String)>, // <name, type>
}

#[derive(Debug, PartialEq, Clone)]
pub enum IrLiteral {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

#[derive(Debug, PartialEq, Clone)]
pub struct OrderByItem {
    pub column: ColumnRef,
    pub direction: OrderDirection,
    pub nulls_first: Option<bool>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum OrderDirection {
    Asc,
    Desc,
}

// Additional structures for conditions
#[derive(Debug, PartialEq, Clone)]
pub struct Condition {
    pub left_field: ComplexField,
    pub operator: ComparisonOp,
    pub right_field: ComplexField,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NullCondition {
    pub field: ComplexField,
    pub operator: NullOp,
}

#[derive(Debug, PartialEq, Clone)]
pub enum InCondition {
    Subquery {
        field: ComplexField,
        subquery: Arc<IrPlan>,
        negated: bool,
    },
    Vec {
        field: ComplexField,
        vector_name: String,
        vector_type: String,
        negated: bool,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExistsCondition {
    Subquery {
        subquery: Arc<IrPlan>,
        negated: bool,
    },
    Vec {
        vector_name: String,
        negated: bool,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum ComparisonOp {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
    GreaterThanEquals,
    LessThanEquals,
}

#[derive(Debug, PartialEq, Clone)]
pub enum NullOp {
    IsNull,
    IsNotNull,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinaryOp {
    And,
    Or,
}

// Implementation of helper methods for the new structure
impl IrPlan {
    // Convenience method to create a filter operation
    pub(crate) fn filter(input: Arc<IrPlan>, predicate: FilterClause) -> Self {
        IrPlan::Filter { input, predicate }
    }

    // Convenience method to create a project operation
    pub(crate) fn project(
        input: Arc<IrPlan>,
        columns: Vec<ProjectionColumn>,
        distinct: bool,
    ) -> Self {
        IrPlan::Project {
            input,
            columns,
            distinct,
        }
    }

    // Similar convenience methods for other operations
    pub(crate) fn group_by(
        input: Arc<IrPlan>,
        keys: Vec<ColumnRef>,
        group_condition: Option<GroupClause>,
    ) -> Self {
        IrPlan::GroupBy {
            input,
            keys,
            group_condition,
        }
    }

    pub(crate) fn order_by(input: Arc<IrPlan>, items: Vec<OrderByItem>) -> Self {
        IrPlan::OrderBy { input, items }
    }

    pub(crate) fn limit(input: Arc<IrPlan>, limit: i64, offset: Option<i64>) -> Self {
        IrPlan::Limit {
            input,
            limit,
            offset,
        }
    }
}

//implement display for ColumnRef
impl std::fmt::Display for ColumnRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(ref table) = self.table {
            write!(f, "{}.{}", table, self.column)
        } else {
            write!(f, "{}", self.column)
        }
    }
}

//implement display for AggregateType
impl std::fmt::Display for AggregateType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AggregateType::Max => write!(f, "max"),
            AggregateType::Min => write!(f, "min"),
            AggregateType::Avg => write!(f, "avg"),
            AggregateType::Sum => write!(f, "sum"),
            AggregateType::Count => write!(f, "count"),
        }
    }
}

//implement display for ComplexField
impl std::fmt::Display for ComplexField {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(ref nested) = self.nested_expr {
            let (left, op, right, is_par) = &**nested;
            write!(f, "{}{} {} {}{}", is_par, left, op, right, is_par)
        } else if let Some(ref col) = self.column_ref {
            write!(f, "{}", col)
        } else if let Some(ref lit) = self.literal {
            match lit {
                IrLiteral::Integer(i) => write!(f, "{}", i),
                IrLiteral::Float(fl) => write!(f, "{:.2}", fl),
                IrLiteral::String(s) => write!(f, "{}", s.clone()),
                IrLiteral::Boolean(b) => write!(f, "{}", b),
            }
        } else if let Some(ref agg) = self.aggregate {
            write!(
                f,
                "{}({})",
                match agg.function {
                    AggregateType::Max => "max",
                    AggregateType::Min => "min",
                    AggregateType::Avg => "avg",
                    AggregateType::Sum => "sum",
                    AggregateType::Count => "count",
                },
                agg.column
            )
        } else {
            write!(f, "")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ref_display_without_table() {
        let col = ColumnRef {
            table: None,
            column: "name".to_string(),
        };
        assert_eq!(col.to_string(), "name");
    }

    #[test]
    fn test_column_ref_display_with_table() {
        let col = ColumnRef {
            table: Some("users".to_string()),
            column: "name".to_string(),
        };
        assert_eq!(col.to_string(), "users.name");
    }

    #[test]
    fn test_aggregate_type_display() {
        assert_eq!(AggregateType::Max.to_string(), "max");
        assert_eq!(AggregateType::Min.to_string(), "min");
        assert_eq!(AggregateType::Avg.to_string(), "avg");
        assert_eq!(AggregateType::Sum.to_string(), "sum");
        assert_eq!(AggregateType::Count.to_string(), "count");
    }

    #[test]
    fn test_complex_field_display_column() {
        let col = ColumnRef {
            table: None,
            column: "id".to_string(),
        };
        let field = ComplexField {
            column_ref: Some(col),
            literal: None,
            aggregate: None,
            nested_expr: None,
            subquery: None,
            subquery_vec: None,
        };
        assert_eq!(field.to_string(), "id");
    }

    #[test]
    fn test_complex_field_display_integer_literal() {
        let field = ComplexField {
            column_ref: None,
            literal: Some(IrLiteral::Integer(42)),
            aggregate: None,
            nested_expr: None,
            subquery: None,
            subquery_vec: None,
        };
        assert_eq!(field.to_string(), "42");
    }

    #[test]
    fn test_complex_field_display_string_literal() {
        let field = ComplexField {
            column_ref: None,
            literal: Some(IrLiteral::String("hello".to_string())),
            aggregate: None,
            nested_expr: None,
            subquery: None,
            subquery_vec: None,
        };
        assert_eq!(field.to_string(), "hello");
    }

    #[test]
    fn test_complex_field_display_aggregate() {
        let col = ColumnRef {
            table: None,
            column: "salary".to_string(),
        };
        let agg = AggregateFunction {
            function: AggregateType::Max,
            column: col,
        };
        let field = ComplexField {
            column_ref: None,
            literal: None,
            aggregate: Some(agg),
            nested_expr: None,
            subquery: None,
            subquery_vec: None,
        };
        assert_eq!(field.to_string(), "max(salary)");
    }

    #[test]
    fn test_ir_plan_table() {
        let plan = IrPlan::Table {
            table_name: "users".to_string(),
        };
        assert!(matches!(plan, IrPlan::Table { .. }));
    }

    #[test]
    fn test_ir_plan_filter() {
        let table = Arc::new(IrPlan::Table {
            table_name: "users".to_string(),
        });
        let predicate = FilterClause::Base(FilterConditionType::Boolean(true));
        let plan = IrPlan::filter(table, predicate);
        assert!(matches!(plan, IrPlan::Filter { .. }));
    }

    #[test]
    fn test_ir_plan_project() {
        let table = Arc::new(IrPlan::Table {
            table_name: "users".to_string(),
        });
        let columns = vec![ProjectionColumn::Column(
            ColumnRef {
                table: None,
                column: "name".to_string(),
            },
            None,
        )];
        let plan = IrPlan::project(table, columns, false);
        assert!(matches!(plan, IrPlan::Project { .. }));
    }

    #[test]
    fn test_ir_plan_group_by() {
        let table = Arc::new(IrPlan::Table {
            table_name: "orders".to_string(),
        });
        let keys = vec![ColumnRef {
            table: None,
            column: "customer_id".to_string(),
        }];
        let plan = IrPlan::group_by(table, keys, None);
        assert!(matches!(plan, IrPlan::GroupBy { .. }));
    }

    #[test]
    fn test_ir_plan_order_by() {
        let table = Arc::new(IrPlan::Table {
            table_name: "products".to_string(),
        });
        let items = vec![OrderByItem {
            column: ColumnRef {
                table: None,
                column: "price".to_string(),
            },
            direction: OrderDirection::Desc,
            nulls_first: None,
        }];
        let plan = IrPlan::order_by(table, items);
        assert!(matches!(plan, IrPlan::OrderBy { .. }));
    }

    #[test]
    fn test_ir_plan_limit() {
        let table = Arc::new(IrPlan::Table {
            table_name: "users".to_string(),
        });
        let plan = IrPlan::limit(table, 10, Some(5));
        assert!(matches!(plan, IrPlan::Limit { .. }));
        if let IrPlan::Limit { limit, offset, .. } = plan {
            assert_eq!(limit, 10);
            assert_eq!(offset, Some(5));
        }
    }

    #[test]
    fn test_filter_clause_and_expression() {
        let left = FilterClause::Base(FilterConditionType::Boolean(true));
        let right = FilterClause::Base(FilterConditionType::Boolean(false));
        let expr = FilterClause::Expression {
            left: Box::new(left),
            binary_op: BinaryOp::And,
            right: Box::new(right),
        };
        assert!(matches!(expr, FilterClause::Expression { .. }));
    }

    #[test]
    fn test_join_condition() {
        let left = ColumnRef {
            table: Some("users".to_string()),
            column: "id".to_string(),
        };
        let right = ColumnRef {
            table: Some("orders".to_string()),
            column: "user_id".to_string(),
        };
        let condition = JoinCondition {
            left_col: left.clone(),
            right_col: right.clone(),
        };
        assert_eq!(condition.left_col, left);
        assert_eq!(condition.right_col, right);
    }

    #[test]
    fn test_ir_plan_join() {
        let left = Arc::new(IrPlan::Table {
            table_name: "users".to_string(),
        });
        let right = Arc::new(IrPlan::Table {
            table_name: "orders".to_string(),
        });
        let condition = vec![JoinCondition {
            left_col: ColumnRef {
                table: Some("users".to_string()),
                column: "id".to_string(),
            },
            right_col: ColumnRef {
                table: Some("orders".to_string()),
                column: "user_id".to_string(),
            },
        }];
        let plan = IrPlan::Join {
            left,
            right,
            condition,
            join_type: JoinType::Inner,
        };
        assert!(matches!(plan, IrPlan::Join { .. }));
    }

    #[test]
    fn test_comparison_condition() {
        let condition = Condition {
            left_field: ComplexField {
                column_ref: Some(ColumnRef {
                    table: None,
                    column: "age".to_string(),
                }),
                literal: None,
                aggregate: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            },
            operator: ComparisonOp::GreaterThan,
            right_field: ComplexField {
                column_ref: None,
                literal: Some(IrLiteral::Integer(18)),
                aggregate: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            },
        };
        assert_eq!(condition.operator, ComparisonOp::GreaterThan);
    }

    #[test]
    fn test_null_condition() {
        let condition = NullCondition {
            field: ComplexField {
                column_ref: Some(ColumnRef {
                    table: None,
                    column: "email".to_string(),
                }),
                literal: None,
                aggregate: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            },
            operator: NullOp::IsNull,
        };
        assert_eq!(condition.operator, NullOp::IsNull);
    }

    #[test]
    fn test_in_condition_subquery() {
        let subquery = Arc::new(IrPlan::Table {
            table_name: "active_users".to_string(),
        });
        let condition = InCondition::Subquery {
            field: ComplexField {
                column_ref: Some(ColumnRef {
                    table: None,
                    column: "user_id".to_string(),
                }),
                literal: None,
                aggregate: None,
                nested_expr: None,
                subquery: None,
                subquery_vec: None,
            },
            subquery,
            negated: false,
        };
        assert!(matches!(condition, InCondition::Subquery { .. }));
    }

    #[test]
    fn test_exists_condition_vec() {
        let condition = ExistsCondition::Vec {
            vector_name: "user_ids".to_string(),
            negated: true,
        };
        if let ExistsCondition::Vec { vector_name, negated } = condition {
            assert_eq!(vector_name, "user_ids");
            assert_eq!(negated, true);
        }
    }

    #[test]
    fn test_projection_column_variants() {
        let col_ref = ColumnRef {
            table: None,
            column: "name".to_string(),
        };
        
        let proj1 = ProjectionColumn::Column(col_ref.clone(), Some("user_name".to_string()));
        assert!(matches!(proj1, ProjectionColumn::Column(_, Some(_))));

        let proj2 = ProjectionColumn::StringLiteral("test".to_string(), None);
        assert!(matches!(proj2, ProjectionColumn::StringLiteral(_, _)));

        let agg = AggregateFunction {
            function: AggregateType::Count,
            column: col_ref,
        };
        let proj3 = ProjectionColumn::Aggregate(agg, Some("total".to_string()));
        assert!(matches!(proj3, ProjectionColumn::Aggregate(_, _)));
    }
}
