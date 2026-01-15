use super::SubqueryInfo;
use std::collections::{HashMap, HashSet, VecDeque};

/// Builds a dependency graph for subqueries and determines execution order
pub struct DependencyGraph {
    /// Adjacency list: subquery_id -> list of subqueries it depends on
    dependencies: HashMap<usize, HashSet<usize>>,
    /// Reverse adjacency list: subquery_id -> list of subqueries that depend on it
    dependents: HashMap<usize, HashSet<usize>>,
}

impl DependencyGraph {
    /// Build a dependency graph from detected subqueries
    pub fn build(subqueries: &[SubqueryInfo]) -> Self {
        let mut dependencies = HashMap::new();
        let mut dependents = HashMap::new();

        for subquery in subqueries {
            dependencies.insert(subquery.id, subquery.dependencies.clone());

            // Build reverse graph
            for &dep_id in &subquery.dependencies {
                dependents
                    .entry(dep_id)
                    .or_insert_with(HashSet::new)
                    .insert(subquery.id);
            }

            // Ensure all nodes exist in dependents map
            dependents.entry(subquery.id).or_insert_with(HashSet::new);
        }

        Self {
            dependencies,
            dependents,
        }
    }

    /// Topologically sort subqueries to get execution order
    /// Returns None if there's a cycle
    pub fn topological_sort(&self) -> Option<Vec<usize>> {
        let mut in_degree = HashMap::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Calculate in-degree for each node
        for &node in self.dependencies.keys() {
            in_degree.insert(node, self.dependencies[&node].len());
            if self.dependencies[&node].is_empty() {
                queue.push_back(node);
            }
        }

        // Kahn's algorithm for topological sort
        while let Some(node) = queue.pop_front() {
            result.push(node);

            // Reduce in-degree for all dependents
            if let Some(deps) = self.dependents.get(&node) {
                for &dependent in deps {
                    if let Some(degree) = in_degree.get_mut(&dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dependent);
                        }
                    }
                }
            }
        }

        // If we processed all nodes, no cycle exists
        if result.len() == self.dependencies.len() {
            Some(result)
        } else {
            None // Cycle detected
        }
    }

    /// Group subqueries into levels where queries in the same level
    /// can be executed in parallel
    pub fn execution_levels(&self) -> Option<Vec<Vec<usize>>> {
        let mut in_degree = HashMap::new();
        let mut levels = Vec::new();

        // Calculate in-degree for each node
        for &node in self.dependencies.keys() {
            in_degree.insert(node, self.dependencies[&node].len());
        }

        while !in_degree.is_empty() {
            // Find all nodes with in-degree 0
            let level: Vec<usize> = in_degree
                .iter()
                .filter(|(_, degree)| **degree == 0)
                .map(|(&node, _)| node)
                .collect();

            if level.is_empty() {
                // Cycle detected
                return None;
            }

            // Remove these nodes from in_degree
            for &node in &level {
                in_degree.remove(&node);

                // Reduce in-degree for dependents
                if let Some(deps) = self.dependents.get(&node) {
                    for &dependent in deps {
                        if let Some(degree) = in_degree.get_mut(&dependent) {
                            *degree -= 1;
                        }
                    }
                }
            }

            levels.push(level);
        }

        Some(levels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::subquery_detector::SubqueryContext;
    use std::sync::Arc;

    fn make_subquery(id: usize, deps: Vec<usize>) -> SubqueryInfo {
        SubqueryInfo {
            id,
            plan: Arc::new(renoir_ir::IrPlan::Source {
                source_name: "test".to_string(),
                alias: None,
            }),
            context: SubqueryContext::WhereClause,
            dependencies: deps.into_iter().collect(),
        }
    }

    #[test]
    fn test_simple_chain() {
        // 0 -> 1 -> 2
        let subqueries = vec![
            make_subquery(0, vec![]),
            make_subquery(1, vec![0]),
            make_subquery(2, vec![1]),
        ];

        let graph = DependencyGraph::build(&subqueries);
        let order = graph.topological_sort().unwrap();

        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn test_parallel_queries() {
        // 0 -> 2
        // 1 -> 2
        let subqueries = vec![
            make_subquery(0, vec![]),
            make_subquery(1, vec![]),
            make_subquery(2, vec![0, 1]),
        ];

        let graph = DependencyGraph::build(&subqueries);
        let levels = graph.execution_levels().unwrap();

        assert_eq!(levels.len(), 2);
        assert!(levels[0].contains(&0) && levels[0].contains(&1));
        assert_eq!(levels[1], vec![2]);
    }

    #[test]
    fn test_cycle_detection() {
        // This shouldn't happen in practice, but test cycle detection
        // 0 -> 1 -> 2 -> 0 (cycle)
        let subqueries = vec![
            make_subquery(0, vec![2]),
            make_subquery(1, vec![0]),
            make_subquery(2, vec![1]),
        ];

        let graph = DependencyGraph::build(&subqueries);
        assert!(graph.topological_sort().is_none());
    }
}
