#!/bin/bash
# Publish test events to Kafka topics

set -e

echo "Publishing user events to 'user-events' topic..."

# Publish events from JSON file
cat << 'EOF' | docker exec -i kafka /opt/kafka/bin/kafka-console-producer.sh \
    --bootstrap-server localhost:9092 \
    --topic user-events
{"event_id": 1, "event_type": "login", "user_id": 101, "timestamp": "2026-01-15T10:00:00Z", "session_duration": 3600}
{"event_id": 2, "event_type": "page_view", "user_id": 101, "timestamp": "2026-01-15T10:01:30Z", "session_duration": 120}
{"event_id": 3, "event_type": "click", "user_id": 102, "timestamp": "2026-01-15T10:02:15Z", "session_duration": 45}
{"event_id": 4, "event_type": "login", "user_id": 103, "timestamp": "2026-01-15T10:03:00Z", "session_duration": 1800}
{"event_id": 5, "event_type": "purchase", "user_id": 101, "timestamp": "2026-01-15T10:05:45Z", "session_duration": 300}
{"event_id": 6, "event_type": "page_view", "user_id": 102, "timestamp": "2026-01-15T10:07:20Z", "session_duration": 180}
{"event_id": 7, "event_type": "logout", "user_id": 103, "timestamp": "2026-01-15T10:10:00Z", "session_duration": 60}
{"event_id": 8, "event_type": "click", "user_id": 104, "timestamp": "2026-01-15T10:12:30Z", "session_duration": 90}
{"event_id": 9, "event_type": "purchase", "user_id": 102, "timestamp": "2026-01-15T10:15:00Z", "session_duration": 420}
{"event_id": 10, "event_type": "login", "user_id": 105, "timestamp": "2026-01-15T10:18:45Z", "session_duration": 2400}
{"event_id": 11, "event_type": "page_view", "user_id": 101, "timestamp": "2026-01-15T10:20:00Z", "session_duration": 200}
{"event_id": 12, "event_type": "purchase", "user_id": 103, "timestamp": "2026-01-15T10:25:30Z", "session_duration": 500}
{"event_id": 13, "event_type": "click", "user_id": 105, "timestamp": "2026-01-15T10:30:15Z", "session_duration": 75}
{"event_id": 14, "event_type": "logout", "user_id": 101, "timestamp": "2026-01-15T10:35:00Z", "session_duration": 30}
{"event_id": 15, "event_type": "login", "user_id": 106, "timestamp": "2026-01-15T10:40:45Z", "session_duration": 5400}
EOF

echo "Published 15 events to 'user-events'"

echo ""
echo "Publishing transactions to 'transactions' topic..."

cat << 'EOF' | docker exec -i kafka /opt/kafka/bin/kafka-console-producer.sh \
    --bootstrap-server localhost:9092 \
    --topic transactions
{"transaction_id": 1001, "user_id": 101, "amount": 150.50, "product": "Laptop", "status": "completed"}
{"transaction_id": 1002, "user_id": 102, "amount": 75.00, "product": "Mouse", "status": "completed"}
{"transaction_id": 1003, "user_id": 103, "amount": 220.99, "product": "Keyboard", "status": "pending"}
{"transaction_id": 1004, "user_id": 104, "amount": 399.99, "product": "Monitor", "status": "completed"}
{"transaction_id": 1005, "user_id": 101, "amount": 89.99, "product": "Webcam", "status": "completed"}
{"transaction_id": 1006, "user_id": 105, "amount": 45.50, "product": "Cable", "status": "failed"}
{"transaction_id": 1007, "user_id": 102, "amount": 199.99, "product": "Headphones", "status": "completed"}
{"transaction_id": 1008, "user_id": 106, "amount": 120.00, "product": "Desk Lamp", "status": "completed"}
{"transaction_id": 1009, "user_id": 103, "amount": 599.99, "product": "Chair", "status": "pending"}
{"transaction_id": 1010, "user_id": 104, "amount": 29.99, "product": "Mouse Pad", "status": "completed"}
EOF

echo "Published 10 transactions to 'transactions'"

echo ""
echo "All messages published successfully!"
echo ""
echo "To verify, consume messages:"
echo "  ./kafka-scripts/consume-events.sh user-events"
echo "  ./kafka-scripts/consume-events.sh transactions"
