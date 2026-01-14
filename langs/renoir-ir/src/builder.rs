use crate::*;
use std::sync::Arc;

/// Builder for constructing IrPlan using a fluent API
pub struct IrPlanBuilder {
    plan: Arc<IrPlan>,
}

impl IrPlanBuilder {
    /// Start building from a table source
    pub fn table(table_name: impl Into<String>) -> Self {
        Self {
            plan: Arc::new(IrPlan::Table {
                table_name: table_name.into(),
            }),
        }
    }

    /// Start building from a scan operation
    pub fn scan(
        input: Arc<IrPlan>,
        stream_name: impl Into<String>,
        alias: Option<String>,
    ) -> Self {
        Self {
            plan: Arc::new(IrPlan::Scan {
                input,
                stream_name: stream_name.into(),
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

    /// Add a projection operation to the plan
    pub fn project(self, columns: Vec<ProjectionColumn>, distinct: bool) -> Self {
        Self {
            plan: Arc::new(IrPlan::Project {
                input: self.plan,
                columns,
                distinct,
            }),
        }
    }

    /// Add a group by operation to the plan
    pub fn group_by(self, keys: Vec<ColumnRef>, group_condition: Option<GroupClause>) -> Self {
        Self {
            plan: Arc::new(IrPlan::GroupBy {
                input: self.plan,
                keys,
                group_condition,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_table_plan() {
        let plan = IrPlanBuilder::table("users").build();
        assert!(matches!(plan.as_ref(), IrPlan::Table { .. }));
    }

    #[test]
    fn test_table_with_filter() {
        let filter = FilterBuilder::comparison(
            ComplexFieldBuilder::column(ColumnRefBuilder::column("age")),
            ComparisonOp::GreaterThan,
            ComplexFieldBuilder::int(18),
        );

        let plan = IrPlanBuilder::table("users").filter(filter).build();

        assert!(matches!(plan.as_ref(), IrPlan::Filter { .. }));
    }

    #[test]
    fn test_table_with_projection() {
        let columns = vec![
            ProjectionBuilder::column(ColumnRefBuilder::column("name"), None),
            ProjectionBuilder::column(ColumnRefBuilder::column("email"), Some("user_email".to_string())),
        ];

        let plan = IrPlanBuilder::table("users")
            .project(columns, false)
            .build();

        assert!(matches!(plan.as_ref(), IrPlan::Project { .. }));
    }

    #[test]
    fn test_complex_plan_with_join() {
        let left_plan = IrPlanBuilder::table("users").build();
        let right_plan = IrPlanBuilder::table("orders").build();

        let join_condition = vec![JoinConditionBuilder::new(
            ColumnRefBuilder::with_table("users", "id"),
            ColumnRefBuilder::with_table("orders", "user_id"),
        )];

        let plan = IrPlanBuilder::table("users")
            .join(right_plan, join_condition, JoinType::Inner)
            .build();

        assert!(matches!(plan.as_ref(), IrPlan::Join { .. }));
    }

    #[test]
    fn test_aggregate_projection() {
        let columns = vec![
            ProjectionBuilder::column(ColumnRefBuilder::column("country"), None),
            ProjectionBuilder::aggregate(
                AggregateFunctionBuilder::count(ColumnRefBuilder::column("id")),
                Some("total_users".to_string()),
            ),
        ];

        let plan = IrPlanBuilder::table("users")
            .group_by(vec![ColumnRefBuilder::column("country")], None)
            .project(columns, false)
            .build();

        assert!(matches!(plan.as_ref(), IrPlan::Project { .. }));
    }

    #[test]
    fn test_order_and_limit() {
        let order_items = vec![OrderByBuilder::desc(ColumnRefBuilder::column("created_at"))];

        let plan = IrPlanBuilder::table("users")
            .order_by(order_items)
            .limit(10, Some(5))
            .build();

        assert!(matches!(plan.as_ref(), IrPlan::Limit { .. }));
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

        let plan = IrPlanBuilder::table("users").filter(combined_filter).build();

        assert!(matches!(plan.as_ref(), IrPlan::Filter { .. }));
    }
}
