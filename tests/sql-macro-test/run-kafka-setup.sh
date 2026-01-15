#!/bin/bash
# Quick start guide for Kafka examples

set -e

echo "🚀 Renoir SQL Kafka Examples - Quick Start"
echo "==========================================="
echo ""

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker first."
    exit 1
fi

echo "Step 1: Setting up Kafka..."
./kafka-scripts/setup-kafka.sh

echo ""
echo "Step 2: Publishing test messages..."
./kafka-scripts/publish-events.sh

echo ""
echo "==========================================="
echo "✅ Kafka infrastructure is ready!"
echo ""
echo "Now you can run the examples:"
echo ""
echo "  Example 6 (Filter):       cargo run --example example6_kafka_filter"
echo "  Example 7 (Aggregation):  cargo run --example example7_kafka_aggregation"
echo "  Example 8 (JOIN):         cargo run --example example8_kafka_join"
echo ""
echo "To monitor results:"
echo "  ./kafka-scripts/consume-events.sh processed-events"
echo ""
echo "To cleanup when done:"
echo "  ./kafka-scripts/cleanup.sh"
echo ""
