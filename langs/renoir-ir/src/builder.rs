use crate::*;
use std::sync::Arc;

/// Builder for constructing streaming programs
pub struct ProgramBuilder {
    program: Program,
}

impl ProgramBuilder {
    pub fn new() -> Self {
        Self {
            program: Program::new(),
        }
    }

    pub fn add_source(mut self, source: SourceDef) -> Self {
        self.program.add_source(source);
        self
    }

    pub fn add_sink(mut self, sink: SinkDef) -> Self {
        self.program.add_sink(sink);
        self
    }

    pub fn add_pipeline(mut self, pipeline: Pipeline) -> Self {
        self.program.add_pipeline(pipeline);
        self
    }

    pub fn build(self) -> Program {
        self.program
    }
}

/// Builder for constructing IrPlan (streaming operations)
pub struct IrPlanBuilder {
    plan: Arc<IrPlan>,
}

impl IrPlanBuilder {
    /// Start building from a source reference
    pub fn source(source_name: impl Into<String>, alias: Option<String>) -> Self {
        Self {
            plan: Arc::new(IrPlan::Source {
                source_name: source_name.into(),
                alias,
            }),
        }
    }

    /// Add a filter operation to the plan
    pub fn filter(self, predicate: FilterClause) -> Self {
        Self {
            plan: Arc::new(IrPlan::Filter {
                input: self.plan,
                predicate,
            }),
        }
    }

    /// Add a map operation to the plan
    pub fn map(self, projections: Vec<ProjectionColumn>) -> Self {
        Self {
            plan: Arc::new(IrPlan::Map {
                input: self.plan,
                projections,
            }),
        }
    }

    /// Add a flat_map operation to the plan
    pub fn flat_map(self, projection: ProjectionColumn) -> Self {
        Self {
            plan: Arc::new(IrPlan::FlatMap {
                input: self.plan,
                projection,
            }),
        }
    }

    /// Add a group by operation to the plan
    pub fn group_by(
        self,
        keys: Vec<ColumnRef>,
        aggregations: Vec<ProjectionColumn>,
        having: Option<GroupClause>,
    ) -> Self {
        Self {
            plan: Arc::new(IrPlan::GroupBy {
                input: self.plan,
                keys,
                aggregations,
                having,
            }),
        }
    }

    /// Add a join operation to the plan
    pub fn join(
        self,
        right: Arc<IrPlan>,
        condition: Vec<JoinCondition>,
        join_type: JoinType,
    ) -> Self {
        Self {
            plan: Arc::new(IrPlan::Join {
                left: self.plan,
                right,
                condition,
                join_type,
            }),
        }
    }

    /// Add an order by operation to the plan
    pub fn order_by(self, items: Vec<OrderByItem>) -> Self {
        Self {
            plan: Arc::new(IrPlan::OrderBy {
                input: self.plan,
                items,
            }),
        }
    }

    /// Add a limit operation to the plan
    pub fn limit(self, limit: i64, offset: Option<i64>) -> Self {
        Self {
            plan: Arc::new(IrPlan::Limit {
                input: self.plan,
                limit,
                offset,
            }),
        }
    }

    /// Add a distinct operation to the plan
    pub fn distinct(self) -> Self {
        Self {
            plan: Arc::new(IrPlan::Distinct { input: self.plan }),
        }
    }

    /// Build and return the final IrPlan
    pub fn build(self) -> Arc<IrPlan> {
        self.plan
    }
}

/// Builder for constructing FilterClause
pub struct FilterBuilder;

impl FilterBuilder {
    /// Create a base filter condition
    pub fn base(condition_type: FilterConditionType) -> FilterClause {
        FilterClause::Base(condition_type)
    }

    /// Create a comparison filter
    pub fn comparison(
        left_field: ComplexField,
        operator: ComparisonOp,
        right_field: ComplexField,
    ) -> FilterClause {
        FilterClause::Base(FilterConditionType::Comparison(Condition {
            left_field,
            operator,
            right_field,
        }))
    }

    /// Create a null check filter
    pub fn null_check(field: ComplexField, operator: NullOp) -> FilterClause {
        FilterClause::Base(FilterConditionType::NullCheck(NullCondition {
            field,
            operator,
        }))
    }

    /// Create an IN filter with subquery
    pub fn in_subquery(field: ComplexField, subquery: Arc<IrPlan>, negated: bool) -> FilterClause {
        FilterClause::Base(FilterConditionType::In(InCondition::Subquery {
            field,
            subquery,
            negated,
        }))
    }

    /// Create an IN filter with vector
    pub fn in_vec(
        field: ComplexField,
        vector_name: impl Into<String>,
        vector_type: impl Into<String>,
        negated: bool,
    ) -> FilterClause {
        FilterClause::Base(FilterConditionType::In(InCondition::Vec {
            field,
            vector_name: vector_name.into(),
            vector_type: vector_type.into(),
            negated,
        }))
    }

    /// Create an EXISTS filter with subquery
    pub fn exists_subquery(subquery: Arc<IrPlan>, negated: bool) -> FilterClause {
        FilterClause::Base(FilterConditionType::Exists(ExistsCondition::Subquery {
            subquery,
            negated,
        }))
    }

    /// Create an EXISTS filter with vector
    pub fn exists_vec(vector_name: impl Into<String>, negated: bool) -> FilterClause {
        FilterClause::Base(FilterConditionType::Exists(ExistsCondition::Vec {
            vector_name: vector_name.into(),
            negated,
        }))
    }

    /// Create a boolean filter
    pub fn boolean(value: bool) -> FilterClause {
        FilterClause::Base(FilterConditionType::Boolean(value))
    }

    /// Combine two filters with AND
    pub fn and(left: FilterClause, right: FilterClause) -> FilterClause {
        FilterClause::Expression {
            left: Box::new(left),
            binary_op: BinaryOp::And,
            right: Box::new(right),
        }
    }

    /// Combine two filters with OR
    pub fn or(left: FilterClause, right: FilterClause) -> FilterClause {
        FilterClause::Expression {
            left: Box::new(left),
            binary_op: BinaryOp::Or,
            right: Box::new(right),
        }
    }
}

/// Builder for constructing GroupClause
pub struct GroupBuilder;

impl GroupBuilder {
    /// Create a base group condition
    pub fn base(condition: GroupBaseCondition) -> GroupClause {
        GroupClause::Base(condition)
    }

    /// Create a comparison group condition
    pub fn comparison(
        left_field: ComplexField,
        operator: ComparisonOp,
        right_field: ComplexField,
    ) -> GroupClause {
        GroupClause::Base(GroupBaseCondition::Comparison(Condition {
            left_field,
            operator,
            right_field,
        }))
    }

    /// Create a null check group condition
    pub fn null_check(field: ComplexField, operator: NullOp) -> GroupClause {
        GroupClause::Base(GroupBaseCondition::NullCheck(NullCondition {
            field,
            operator,
        }))
    }

    /// Create an IN group condition with subquery
    pub fn in_subquery(field: ComplexField, subquery: Arc<IrPlan>, negated: bool) -> GroupClause {
        GroupClause::Base(GroupBaseCondition::In(InCondition::Subquery {
            field,
            subquery,
            negated,
        }))
    }

    /// Create a boolean group condition
    pub fn boolean(value: bool) -> GroupClause {
        GroupClause::Base(GroupBaseCondition::Boolean(value))
    }

    /// Combine two group conditions with AND
    pub fn and(left: GroupClause, right: GroupClause) -> GroupClause {
        GroupClause::Expression {
            left: Box::new(left),
            op: BinaryOp::And,
            right: Box::new(right),
        }
    }

    /// Combine two group conditions with OR
    pub fn or(left: GroupClause, right: GroupClause) -> GroupClause {
        GroupClause::Expression {
            left: Box::new(left),
            op: BinaryOp::Or,
            right: Box::new(right),
        }
    }
}

/// Builder for constructing ComplexField
pub struct ComplexFieldBuilder;

impl ComplexFieldBuilder {
    /// Create a ComplexField from a column reference
    pub fn column(column_ref: ColumnRef) -> ComplexField {
        ComplexField {
            column_ref: Some(column_ref),
            literal: None,
            aggregate: None,
            nested_expr: None,
            subquery: None,
            subquery_vec: None,
        }
    }

    /// Create a ComplexField from a literal value
    pub fn literal(literal: IrLiteral) -> ComplexField {
        ComplexField {
            column_ref: None,
            literal: Some(literal),
            aggregate: None,
            nested_expr: None,
            subquery: None,
            subquery_vec: None,
        }
    }

    /// Create a ComplexField from an integer literal
    pub fn int(value: i64) -> ComplexField {
        Self::literal(IrLiteral::Integer(value))
    }

    /// Create a ComplexField from a float literal
    pub fn float(value: f64) -> ComplexField {
        Self::literal(IrLiteral::Float(value))
    }

    /// Create a ComplexField from a string literal
    pub fn string(value: impl Into<String>) -> ComplexField {
        Self::literal(IrLiteral::String(value.into()))
    }

    /// Create a ComplexField from a boolean literal
    pub fn bool(value: bool) -> ComplexField {
        Self::literal(IrLiteral::Boolean(value))
    }

    /// Create a ComplexField from an aggregate function
    pub fn aggregate(aggregate: AggregateFunction) -> ComplexField {
        ComplexField {
            column_ref: None,
            literal: None,
            aggregate: Some(aggregate),
            nested_expr: None,
            subquery: None,
            subquery_vec: None,
        }
    }

    /// Create a ComplexField from a nested expression
    pub fn nested_expr(
        left: ComplexField,
        op: impl Into<String>,
        right: ComplexField,
        is_parenthesized: bool,
    ) -> ComplexField {
        ComplexField {
            column_ref: None,
            literal: None,
            aggregate: None,
            nested_expr: Some(Box::new((left, op.into(), right, is_parenthesized))),
            subquery: None,
            subquery_vec: None,
        }
    }

    /// Create a ComplexField from a subquery
    pub fn subquery(subquery: Arc<IrPlan>) -> ComplexField {
        ComplexField {
            column_ref: None,
            literal: None,
            aggregate: None,
            nested_expr: None,
            subquery: Some(subquery),
            subquery_vec: None,
        }
    }

    /// Create a ComplexField from a subquery vector
    pub fn subquery_vec(name: impl Into<String>, vec_type: impl Into<String>) -> ComplexField {
        ComplexField {
            column_ref: None,
            literal: None,
            aggregate: None,
            nested_expr: None,
            subquery: None,
            subquery_vec: Some((name.into(), vec_type.into())),
        }
    }
}

/// Builder for constructing ColumnRef
pub struct ColumnRefBuilder;

impl ColumnRefBuilder {
    /// Create a column reference with optional table
    pub fn new(column: impl Into<String>, table: Option<String>) -> ColumnRef {
        ColumnRef {
            table,
            column: column.into(),
        }
    }

    /// Create a column reference without table
    pub fn column(column: impl Into<String>) -> ColumnRef {
        ColumnRef {
            table: None,
            column: column.into(),
        }
    }

    /// Create a column reference with table
    pub fn with_table(table: impl Into<String>, column: impl Into<String>) -> ColumnRef {
        ColumnRef {
            table: Some(table.into()),
            column: column.into(),
        }
    }
}

/// Builder for constructing AggregateFunction
pub struct AggregateFunctionBuilder;

impl AggregateFunctionBuilder {
    /// Create an aggregate function
    pub fn new(function: AggregateType, column: ColumnRef) -> AggregateFunction {
        AggregateFunction { function, column }
    }

    /// Create a MAX aggregate
    pub fn max(column: ColumnRef) -> AggregateFunction {
        Self::new(AggregateType::Max, column)
    }

    /// Create a MIN aggregate
    pub fn min(column: ColumnRef) -> AggregateFunction {
        Self::new(AggregateType::Min, column)
    }

    /// Create an AVG aggregate
    pub fn avg(column: ColumnRef) -> AggregateFunction {
        Self::new(AggregateType::Avg, column)
    }

    /// Create a COUNT aggregate
    pub fn count(column: ColumnRef) -> AggregateFunction {
        Self::new(AggregateType::Count, column)
    }

    /// Create a SUM aggregate
    pub fn sum(column: ColumnRef) -> AggregateFunction {
        Self::new(AggregateType::Sum, column)
    }
}

/// Builder for constructing ProjectionColumn
pub struct ProjectionBuilder;

impl ProjectionBuilder {
    /// Create a column projection
    pub fn column(column_ref: ColumnRef, alias: Option<String>) -> ProjectionColumn {
        ProjectionColumn::Column(column_ref, alias)
    }

    /// Create an aggregate projection
    pub fn aggregate(aggregate: AggregateFunction, alias: Option<String>) -> ProjectionColumn {
        ProjectionColumn::Aggregate(aggregate, alias)
    }

    /// Create a complex value projection
    pub fn complex(complex_field: ComplexField, alias: Option<String>) -> ProjectionColumn {
        ProjectionColumn::ComplexValue(complex_field, alias)
    }

    /// Create a string literal projection
    pub fn string_literal(value: impl Into<String>, alias: Option<String>) -> ProjectionColumn {
        ProjectionColumn::StringLiteral(value.into(), alias)
    }

    /// Create a subquery projection
    pub fn subquery(subquery: Arc<IrPlan>, alias: Option<String>) -> ProjectionColumn {
        ProjectionColumn::Subquery(subquery, alias)
    }

    /// Create a subquery vector projection
    pub fn subquery_vec(name: impl Into<String>, alias: Option<String>) -> ProjectionColumn {
        ProjectionColumn::SubqueryVec(name.into(), alias)
    }
}

/// Builder for constructing OrderByItem
pub struct OrderByBuilder;

impl OrderByBuilder {
    /// Create an order by item
    pub fn new(
        column: ColumnRef,
        direction: OrderDirection,
        nulls_first: Option<bool>,
    ) -> OrderByItem {
        OrderByItem {
            column,
            direction,
            nulls_first,
        }
    }

    /// Create an ascending order by item
    pub fn asc(column: ColumnRef) -> OrderByItem {
        Self::new(column, OrderDirection::Asc, None)
    }

    /// Create a descending order by item
    pub fn desc(column: ColumnRef) -> OrderByItem {
        Self::new(column, OrderDirection::Desc, None)
    }

    /// Create an ascending order by item with nulls first
    pub fn asc_nulls_first(column: ColumnRef) -> OrderByItem {
        Self::new(column, OrderDirection::Asc, Some(true))
    }

    /// Create a descending order by item with nulls first
    pub fn desc_nulls_first(column: ColumnRef) -> OrderByItem {
        Self::new(column, OrderDirection::Desc, Some(true))
    }
}

/// Builder for constructing JoinCondition
pub struct JoinConditionBuilder;

impl JoinConditionBuilder {
    /// Create a join condition
    pub fn new(left_col: ColumnRef, right_col: ColumnRef) -> JoinCondition {
        JoinCondition {
            left_col,
            right_col,
        }
    }
}

/// Builder for constructing SourceDef
pub struct SourceDefBuilder;

impl SourceDefBuilder {
    pub fn new(
        name: impl Into<String>,
        schema: Vec<FieldDef>,
        connector_type: impl Into<String>,
        options: Vec<ConnectorOption>,
    ) -> SourceDef {
        SourceDef {
            name: name.into(),
            schema,
            connector: ConnectorConfig {
                connector_type: connector_type.into(),
                options,
            },
        }
    }
}

/// Builder for constructing SinkDef
pub struct SinkDefBuilder;

impl SinkDefBuilder {
    pub fn new(
        name: impl Into<String>,
        schema: Vec<FieldDef>,
        connector_type: impl Into<String>,
        options: Vec<ConnectorOption>,
    ) -> SinkDef {
        SinkDef {
            name: name.into(),
            schema,
            connector: ConnectorConfig {
                connector_type: connector_type.into(),
                options,
            },
        }
    }
}

/// Builder for constructing Pipeline
pub struct PipelineBuilder;

impl PipelineBuilder {
    pub fn new(
        sink_name: impl Into<String>,
        sink_columns: Vec<String>,
        plan: Arc<IrPlan>,
    ) -> Pipeline {
        Pipeline {
            sink_name: sink_name.into(),
            sink_columns,
            plan,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_source_plan() {
        let plan = IrPlanBuilder::source("users", None).build();
        assert!(matches!(plan.as_ref(), IrPlan::Source { .. }));
    }

    #[test]
    fn test_source_with_filter() {
        let filter = FilterBuilder::comparison(
            ComplexFieldBuilder::column(ColumnRefBuilder::column("age")),
            ComparisonOp::GreaterThan,
            ComplexFieldBuilder::int(18),
        );

        let plan = IrPlanBuilder::source("users", None).filter(filter).build();

        assert!(matches!(plan.as_ref(), IrPlan::Filter { .. }));
    }

    #[test]
    fn test_source_with_map() {
        let projections = vec![
            ProjectionBuilder::column(ColumnRefBuilder::column("name"), None),
            ProjectionBuilder::column(
                ColumnRefBuilder::column("email"),
                Some("user_email".to_string()),
            ),
        ];

        let plan = IrPlanBuilder::source("users", None)
            .map(projections)
            .build();

        assert!(matches!(plan.as_ref(), IrPlan::Map { .. }));
    }

    #[test]
    fn test_complex_plan_with_join() {
        let right_plan = IrPlanBuilder::source("orders", None).build();

        let join_condition = vec![JoinConditionBuilder::new(
            ColumnRefBuilder::with_table("users", "id"),
            ColumnRefBuilder::with_table("orders", "user_id"),
        )];

        let plan = IrPlanBuilder::source("users", None)
            .join(right_plan, join_condition, JoinType::Inner)
            .build();

        assert!(matches!(plan.as_ref(), IrPlan::Join { .. }));
    }

    #[test]
    fn test_aggregate_with_group_by() {
        let aggregations = vec![
            ProjectionBuilder::column(ColumnRefBuilder::column("country"), None),
            ProjectionBuilder::aggregate(
                AggregateFunctionBuilder::count(ColumnRefBuilder::column("id")),
                Some("total_users".to_string()),
            ),
        ];

        let plan = IrPlanBuilder::source("users", None)
            .group_by(
                vec![ColumnRefBuilder::column("country")],
                aggregations,
                None,
            )
            .build();

        assert!(matches!(plan.as_ref(), IrPlan::GroupBy { .. }));
    }

    #[test]
    fn test_order_and_limit() {
        let order_items = vec![OrderByBuilder::desc(ColumnRefBuilder::column("created_at"))];

        let plan = IrPlanBuilder::source("users", None)
            .order_by(order_items)
            .limit(10, Some(5))
            .build();

        assert!(matches!(plan.as_ref(), IrPlan::Limit { .. }));
    }

    #[test]
    fn test_distinct() {
        let plan = IrPlanBuilder::source("users", None).distinct().build();

        assert!(matches!(plan.as_ref(), IrPlan::Distinct { .. }));
    }

    #[test]
    fn test_complex_filter_with_and_or() {
        let age_filter = FilterBuilder::comparison(
            ComplexFieldBuilder::column(ColumnRefBuilder::column("age")),
            ComparisonOp::GreaterThan,
            ComplexFieldBuilder::int(18),
        );

        let status_filter = FilterBuilder::comparison(
            ComplexFieldBuilder::column(ColumnRefBuilder::column("status")),
            ComparisonOp::Equal,
            ComplexFieldBuilder::string("active"),
        );

        let combined_filter = FilterBuilder::and(age_filter, status_filter);

        let plan = IrPlanBuilder::source("users", None)
            .filter(combined_filter)
            .build();

        assert!(matches!(plan.as_ref(), IrPlan::Filter { .. }));
    }

    #[test]
    fn test_program_with_sources_and_pipelines() {
        let source = SourceDefBuilder::new(
            "sales",
            vec![
                FieldDef {
                    name: "product_id".to_string(),
                    data_type: DataType::BigInt,
                },
                FieldDef {
                    name: "quantity".to_string(),
                    data_type: DataType::Integer,
                },
            ],
            "csv",
            vec![ConnectorOption {
                key: "path".to_string(),
                value: OptionValue::String("data/sales.csv".to_string()),
            }],
        );

        let sink = SinkDefBuilder::new(
            "filtered_sales",
            vec![FieldDef {
                name: "product_id".to_string(),
                data_type: DataType::BigInt,
            }],
            "csv",
            vec![],
        );

        let plan = IrPlanBuilder::source("sales", None)
            .filter(FilterBuilder::comparison(
                ComplexFieldBuilder::column(ColumnRefBuilder::column("quantity")),
                ComparisonOp::GreaterThan,
                ComplexFieldBuilder::int(10),
            ))
            .build();

        let pipeline = PipelineBuilder::new("filtered_sales", vec!["product_id".to_string()], plan);

        let program = ProgramBuilder::new()
            .add_source(source)
            .add_sink(sink)
            .add_pipeline(pipeline)
            .build();

        assert_eq!(program.sources.len(), 1);
        assert_eq!(program.sinks.len(), 1);
        assert_eq!(program.pipelines.len(), 1);
    }
}
