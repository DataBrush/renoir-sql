#!/bin/bash
# Final verification script - Test all production-ready examples

echo "======================================================================"
echo "Renoir SQL Code Generator - Production Verification"
echo "======================================================================"
echo ""

# Track results
PASSED=0
FAILED=0

# Function to run an example and check results
run_example() {
    local name="$1"
    local example="$2"
    local output_pattern="$3"
    
    echo "Testing: $name"
    echo "----------------------------------------"
    
    # Clean previous outputs
    rm -f test-data/output-*.csv 2>/dev/null
    
    # Run example
    if cargo run --example "$example" --quiet 2>&1 | grep -q "✅"; then
        # Check if output files were created
        if ls test-data/${output_pattern}*.csv 1> /dev/null 2>&1; then
            local count=$(cat test-data/${output_pattern}*.csv | grep -v "^id,\|^product_id" | wc -l)
            echo "PASS - Generated $count output rows"
            PASSED=$((PASSED + 1))
        else
            echo "FAIL - No output files generated"
            FAILED=$((FAILED + 1))
        fi
    else
        echo "FAIL - Compilation or runtime error"
        FAILED=$((FAILED + 1))
    fi
    echo ""
}

# Run all working examples
run_example "Example 1: Basic Filter (Adults)" "example1_basic_filter" "output-adults"
run_example "Example 2: High Earners Filter" "example2_aggregation" "output-high-earners"
run_example "Example 3: Engineering Department" "example3_join" "output-engineers"
run_example "Example 4: Electronics Category" "example4_complex_query" "output-premium-products"
run_example "Example 5: Management Filter" "example5_top_n" "output-managers"

# Summary
echo "======================================================================"
echo "SUMMARY"
echo "======================================================================"
echo ""
echo "Total Tests: $((PASSED + FAILED))"
echo "Passed: $PASSED"
echo "Failed: $FAILED"
echo ""

if [ $FAILED -eq 0 ]; then
    echo "ALL TESTS PASSED - Production Ready!"
    echo ""
    echo "Sample Output:"
    echo "----------------------------------------"
    echo "Adults (age >= 18):"
    cat test-data/output-adults*.csv 2>/dev/null | grep -v "^id,name" | head -3
    echo ""
    echo "Engineers:"
    cat test-data/output-engineers*.csv 2>/dev/null | grep -v "^id,name" | head -3
    echo ""
    echo "Managers:"
    cat test-data/output-managers*.csv 2>/dev/null | grep -v "^id,name" | head -3
    echo ""
    exit 0
else
    echo "Some tests failed - Review errors above"
    exit 1
fi
