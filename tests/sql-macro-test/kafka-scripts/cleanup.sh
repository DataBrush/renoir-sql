#!/bin/bash
# Cleanup Kafka infrastructure

echo "🧹 Stopping and removing Kafka containers..."
docker-compose down -v

echo "✅ Kafka infrastructure cleaned up"
echo ""
echo "To start again:"
echo "  ./kafka-scripts/setup-kafka.sh"
