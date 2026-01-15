#!/bin/bash
# Consume messages from a Kafka topic

TOPIC=${1:-user-events}

echo "Consuming messages from topic: $TOPIC"
echo "Press Ctrl+C to stop"
echo "----------------------------------------"

docker exec -it kafka /opt/kafka/bin/kafka-console-consumer.sh \
    --bootstrap-server localhost:9092 \
    --topic $TOPIC \
    --from-beginning \
    --property print.key=true \
    --property print.timestamp=true
