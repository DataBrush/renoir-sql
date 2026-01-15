use renoir_ir::{ExistsCondition, FilterClause, FilterConditionType, InCondition, IrPlan, Pipeline, Program};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Information about a detected subquery
#[derive(Debug, Clone)]
pub struct SubqueryInfo {
    /// Unique identifier for this subquery
    pub id: usize,
    /// The IR plan for this subquery
    pub plan: Arc<IrPlan>,
    /// Context where this subquery appears
    pub context: SubqueryContext,
    /// IDs of subqueries this one depends on
    pub dependencies: HashSet<usize>,
}

/// Context where a subquery appears
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubqueryContext {
    /// Subquery in WHERE clause (IN, EXISTS, etc.)
    WhereClause,
    /// Subquery in FROM clause (derived table)
    FromClause,
    /// Subquery in SELECT clause (scalar subquery)
    SelectClause,
    /// Subquery in HAVING clause
    HavingClause,
}

/// Detector for finding and extracting subqueries from an IR tree
pub struct SubqueryDetector {
    next_id: usize,
    subqueries: Vec<SubqueryInfo>,
    /// Map from subquery plan pointer to its ID (for deduplication and dependency tracking)
    plan_to_id: HashMap<usize, usize>,
}

impl SubqueryDetector {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            subqueries: Vec::new(),
            plan_to_id: HashMap::new(),
        }
    }

    /// Detect all subqueries in a program
    pub fn detect_in_program(&mut self, program: &Program) -> Vec<SubqueryInfo> {
        for pipeline in &program.pipelines {
            self.detect_in_pipeline(pipeline);
        }
        self.subqueries.clone()
    }

    /// Detect subqueries in a pipeline
    fn detect_in_pipeline(&mut self, pipeline: &Pipeline) {
        self.detect_in_plan(&pipeline.plan, SubqueryContext::FromClause);
    }

    /// Recursively detect subqueries in an IR plan
    fn detect_in_plan(&mut self, plan: &Arc<IrPlan>, context: SubqueryContext) {
        match plan.as_ref() {
            IrPlan::Source { .. } => {
                // Base case: sources don't contain subqueries
            }
            IrPlan::Filter { input, predicate } => {
                // First check the input
                self.detect_in_plan(input, context.clone());
                // Then check the predicate for subqueries
                self.detect_in_filter(predicate);
            }
            IrPlan::Map { input, .. } => {
                self.detect_in_plan(input, context);
            }
            IrPlan::FlatMap { input, .. } => {
                self.detect_in_plan(input, context);
            }
            IrPlan::GroupBy { input, .. } => {
                self.detect_in_plan(input, context);
            }
            IrPlan::Join { left, right, .. } => {
                self.detect_in_plan(left, context.clone());
                self.detect_in_plan(right, context);
            }
            IrPlan::OrderBy { input, .. } => {
                self.detect_in_plan(input, context);
            }
            IrPlan::Limit { input, .. } => {
                self.detect_in_plan(input, context);
            }
            IrPlan::Distinct { input } => {
                self.detect_in_plan(input, context);
            }
        }
    }

    /// Detect subqueries in a filter clause
    fn detect_in_filter(&mut self, filter: &FilterClause) {
        match filter {
            FilterClause::Base(cond_type) => {
                match cond_type {
                    FilterConditionType::In(in_cond) => {
                        if let InCondition::Subquery { subquery, .. } = in_cond {
                            self.register_subquery(subquery.clone(), SubqueryContext::WhereClause);
                        }
                    }
                    FilterConditionType::Exists(exists_cond) => {
                        if let ExistsCondition::Subquery { subquery, .. } = exists_cond {
                            self.register_subquery(subquery.clone(), SubqueryContext::WhereClause);
                        }
                    }
                    _ => {
                        // Other filter types don't contain subqueries
                    }
                }
            }
            FilterClause::Expression { left, right, .. } => {
                self.detect_in_filter(left);
                self.detect_in_filter(right);
            }
        }
    }

    /// Register a new subquery and return its ID
    /// If the subquery was already registered, return existing ID
    fn register_subquery(
        &mut self,
        plan: Arc<IrPlan>,
        context: SubqueryContext,
    ) -> usize {
        // Use the Arc pointer address as a unique identifier
        let plan_ptr = Arc::as_ptr(&plan) as usize;
        
        // Check if we've already registered this subquery
        if let Some(&existing_id) = self.plan_to_id.get(&plan_ptr) {
            return existing_id;
        }

        let id = self.next_id;
        self.next_id += 1;

        // Detect dependencies by recursively scanning the subquery
        let dependencies = self.find_dependencies(&plan);

        self.subqueries.push(SubqueryInfo {
            id,
            plan: plan.clone(),
            context,
            dependencies,
        });

        self.plan_to_id.insert(plan_ptr, id);
        id
    }

    /// Find all subquery IDs that this plan depends on
    fn find_dependencies(&mut self, plan: &Arc<IrPlan>) -> HashSet<usize> {
        let mut deps = HashSet::new();
        
        match plan.as_ref() {
            IrPlan::Source { .. } => {}
            IrPlan::Filter { input, predicate } => {
                deps.extend(self.find_dependencies(input));
                self.find_dependencies_in_filter(predicate, &mut deps);
            }
            IrPlan::Map { input, .. } => {
                deps.extend(self.find_dependencies(input));
            }
            IrPlan::FlatMap { input, .. } => {
                deps.extend(self.find_dependencies(input));
            }
            IrPlan::GroupBy { input, .. } => {
                deps.extend(self.find_dependencies(input));
            }
            IrPlan::Join { left, right, .. } => {
                deps.extend(self.find_dependencies(left));
                deps.extend(self.find_dependencies(right));
            }
            IrPlan::OrderBy { input, .. } => {
                deps.extend(self.find_dependencies(input));
            }
            IrPlan::Limit { input, .. } => {
                deps.extend(self.find_dependencies(input));
            }
            IrPlan::Distinct { input } => {
                deps.extend(self.find_dependencies(input));
            }
        }
        
        deps
    }

    /// Find subquery dependencies in a filter clause
    fn find_dependencies_in_filter(&self, filter: &FilterClause, deps: &mut HashSet<usize>) {
        match filter {
            FilterClause::Base(cond_type) => {
                match cond_type {
                    FilterConditionType::In(InCondition::Subquery { subquery, .. }) => {
                        let plan_ptr = Arc::as_ptr(subquery) as usize;
                        if let Some(&id) = self.plan_to_id.get(&plan_ptr) {
                            deps.insert(id);
                        }
                    }
                    FilterConditionType::Exists(ExistsCondition::Subquery { subquery, .. }) => {
                        let plan_ptr = Arc::as_ptr(subquery) as usize;
                        if let Some(&id) = self.plan_to_id.get(&plan_ptr) {
                            deps.insert(id);
                        }
                    }
                    _ => {}
                }
            }
            FilterClause::Expression { left, right, .. } => {
                self.find_dependencies_in_filter(left, deps);
                self.find_dependencies_in_filter(right, deps);
            }
        }
    }
}

impl Default for SubqueryDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_program() {
        let program = Program {
            sources: vec![],
            sinks: vec![],
            pipelines: vec![],
        };

        let mut detector = SubqueryDetector::new();
        let subqueries = detector.detect_in_program(&program);

        assert_eq!(subqueries.len(), 0);
    }
}
