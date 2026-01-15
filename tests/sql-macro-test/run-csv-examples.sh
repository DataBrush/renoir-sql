#!/bin/bash
# Run all CSV-based examples and display results

set -e

echo "Running Renoir SQL CSV Examples"
echo "===================================="
echo ""

# Clean previous outputs
rm -f test-data/output-*

# Example 1: Basic Filter
echo "Example 1: Basic Filter (adults only)"
echo "----------------------------------------"
cargo run --example example1_basic_filter --quiet 2>&1 | tail -2
echo ""
echo "Sample output (first 5 adults):"
cat test-data/output-adults*.csv 2>/dev/null | grep -v "^id,name" | head -5 || echo "No output generated"
echo ""

# Example 2: High Earners
echo "Example 2: High Earners Filter"
echo "----------------------------------------"
cargo run --example example2_aggregation --quiet 2>&1 | tail -2
echo ""
echo "Sample output:"
cat test-data/output-high-earners*.csv 2>/dev/null | grep -v "^id,name" | head -5 || echo "No output generated"
echo ""

# Example 3: Engineers
echo "Example 3: Senior Engineers Filter"
echo "----------------------------------------"
cargo run --example example3_join --quiet 2>&1 | tail -2
echo ""
echo "Sample output:"
cat test-data/output-engineers*.csv 2>/dev/null | grep -v "^id,name" | head -5 || echo "No output generated"
echo ""

# Example 4: Premium Products
echo "Example 4: Premium Electronics Filter"
echo "----------------------------------------"
cargo run --example example4_complex_query --quiet 2>&1 | tail -2
echo ""
echo "Sample output:"
cat test-data/output-premium-products*.csv 2>/dev/null | grep -v "^product_id" | head -5 || echo "No output generated"
echo ""

# Example 5: Managers
echo "Example 5: Management Team Filter"
echo "----------------------------------------"
cargo run --example example5_top_n --quiet 2>&1 | tail -2
echo ""
echo "Sample output:"
cat test-data/output-managers*.csv 2>/dev/null | grep -v "^id,name" | head -5 || echo "No output generated"
echo ""

echo "===================================="
echo "All CSV examples completed successfully!"
echo ""
echo "Output files generated:"
ls -lh test-data/output-*0000.csv 2>/dev/null || ls -lh test-data/output-*.csv 2>/dev/null
echo ""
echo "To view combined results:"
echo "  cat test-data/output-<name>*.csv | grep -v '^id,name' | sort -u"
