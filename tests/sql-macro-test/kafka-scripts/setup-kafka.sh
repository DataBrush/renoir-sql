#!/bin/bash
# Setup Kafka topics and publish test messages

set -e

echo "Starting Kafka infrastructure..."
docker-compose up -d

echo "Waiting for Kafka to be ready..."
sleep 10

# Check if Kafka is ready
until docker exec kafka /opt/kafka/bin/kafka-broker-api-versions.sh --bootstrap-server localhost:9092 > /dev/null 2>&1; do
    echo "Waiting for Kafka..."
    sleep 2
done

echo "Kafka is ready!"

# Create topics
echo "Creating topics..."
docker exec kafka /opt/kafka/bin/kafka-topics.sh \
    --create \
    --topic user-events \
    --bootstrap-server localhost:9092 \
    --partitions 1 \
    --replication-factor 1 \
    --if-not-exists

docker exec kafka /opt/kafka/bin/kafka-topics.sh \
    --create \
    --topic transactions \
    --bootstrap-server localhost:9092 \
    --partitions 1 \
    --replication-factor 1 \
    --if-not-exists

docker exec kafka /opt/kafka/bin/kafka-topics.sh \
    --create \
    --topic processed-events \
    --bootstrap-server localhost:9092 \
    --partitions 1 \
    --replication-factor 1 \
    --if-not-exists

echo "Topics created successfully"

# List topics to verify
echo "Available topics:"
docker exec kafka /opt/kafka/bin/kafka-topics.sh \
    --list \
    --bootstrap-server localhost:9092

echo ""
echo "Kafka setup complete!"
echo ""
echo "To publish messages:"
echo "  ./kafka-scripts/publish-events.sh"
echo ""
echo "To consume messages:"
echo "  ./kafka-scripts/consume-events.sh <topic-name>"
echo ""
echo "To shutdown:"
echo "  docker-compose down"
