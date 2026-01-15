use std::collections::HashMap;

/// Generates unique variable names for code generation
pub struct NameGenerator {
    counters: HashMap<String, usize>,
}

impl NameGenerator {
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    /// Generate a unique name with a given prefix
    /// Examples: "source" -> "source_0", "source_1", etc.
    pub fn generate(&mut self, prefix: &str) -> String {
        let counter = self.counters.entry(prefix.to_string()).or_insert(0);
        let name = format!("{}_{}", prefix, counter);
        *counter += 1;
        name
    }

    /// Generate a unique subquery result variable name
    pub fn subquery_result(&mut self, id: usize) -> String {
        format!("subquery_{}_result", id)
    }

    /// Generate a unique subquery data variable name
    pub fn subquery_data(&mut self, id: usize) -> String {
        format!("subquery_{}_data", id)
    }

    /// Generate a unique source variable name
    pub fn source(&mut self, source_name: &str) -> String {
        format!("{}_source", source_name)
    }

    /// Generate a unique sink variable name
    pub fn sink(&mut self, sink_name: &str) -> String {
        format!("{}_sink", sink_name)
    }

    /// Generate a unique pipeline variable name
    pub fn pipeline(&mut self) -> String {
        self.generate("pipeline")
    }

    /// Generate a unique temporary variable name
    pub fn temp(&mut self) -> String {
        self.generate("temp")
    }

    /// Reset all counters
    pub fn reset(&mut self) {
        self.counters.clear();
    }
}

impl Default for NameGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unique_names() {
        let mut name_gen = NameGenerator::new();

        assert_eq!(name_gen.generate("var"), "var_0");
        assert_eq!(name_gen.generate("var"), "var_1");
        assert_eq!(name_gen.generate("var"), "var_2");
    }

    #[test]
    fn test_different_prefixes() {
        let mut name_gen = NameGenerator::new();

        assert_eq!(name_gen.generate("foo"), "foo_0");
        assert_eq!(name_gen.generate("bar"), "bar_0");
        assert_eq!(name_gen.generate("foo"), "foo_1");
        assert_eq!(name_gen.generate("bar"), "bar_1");
    }

    #[test]
    fn test_subquery_names() {
        let mut name_gen = NameGenerator::new();

        assert_eq!(name_gen.subquery_result(0), "subquery_0_result");
        assert_eq!(name_gen.subquery_data(0), "subquery_0_data");
        assert_eq!(name_gen.subquery_result(5), "subquery_5_result");
    }

    #[test]
    fn test_source_sink_names() {
        let mut name_gen = NameGenerator::new();

        assert_eq!(name_gen.source("users"), "users_source");
        assert_eq!(name_gen.sink("output"), "output_sink");
    }

    #[test]
    fn test_reset() {
        let mut name_gen = NameGenerator::new();

        assert_eq!(name_gen.generate("var"), "var_0");
        assert_eq!(name_gen.generate("var"), "var_1");

        name_gen.reset();

        assert_eq!(name_gen.generate("var"), "var_0");
    }
}
