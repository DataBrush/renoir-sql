#!/bin/bash
# Utility script to combine sharded Renoir output files

if [ -z "$1" ]; then
    echo "Usage: $0 <output-file-prefix>"
    echo "Example: $0 test-data/output-adults"
    exit 1
fi

OUTPUT_PREFIX="$1"
COMBINED="${OUTPUT_PREFIX}.csv"

# Find all sharded files
SHARDS=("${OUTPUT_PREFIX}"*.csv)

if [ ! -f "${SHARDS[0]}" ]; then
    echo "No sharded files found for: ${OUTPUT_PREFIX}*.csv"
    exit 1
fi

# Combine all shards, keeping only the first header
{
    head -n 1 "${SHARDS[0]}"  # First header
    for shard in "${SHARDS[@]}"; do
        tail -n +2 "$shard"    # Skip headers in subsequent files
    done
} > "$COMBINED"

echo "Combined $(echo "${SHARDS[@]}" | wc -w) shards into: $COMBINED"
echo "Total rows: $(wc -l < "$COMBINED")"