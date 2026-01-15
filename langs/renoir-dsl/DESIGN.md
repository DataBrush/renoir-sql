# Renoir DSL Design Document

## Overview

Renoir DSL is a domain-specific language for defining streaming data pipelines that compile to Renoir IR. The language provides a declarative, human-readable syntax for expressing complex dataflow operations.

## Goals

1. **Declarative**: Express what the pipeline does, not how it does it
2. **Type-safe**: Catch errors at parse/compile time
3. **Readable**: Easy to understand pipeline structure at a glance
4. **Composable**: Build complex pipelines from simple operations
5. **IR-aligned**: Direct mapping to Renoir IR constructs

## Implementation Notes

⚠️ **IR Crate Requirements**: The following features need to be implemented in the `renoir-ir` crate before full DSL support:
- **Windowing operations**: Tumbling, sliding, and session windows for time-based aggregations
- **Split operator**: Ability to clone stream items into multiple parallel paths
- **Config validation**: Connector-specific configuration validation framework

## Language Syntax

### Source Definitions

Sources declare input streams with their schema and connector configuration.

```renoir
source <name> {
  <field_name>: <type>,
  <field_name>: <type>,
  ...
} config {
  connector = '<connector_type>',
  <key> = <value>,
  ...
}
```

**Example - CSV Source:**
```renoir
source users {
  id: bigint,
  name: string,
  email: string,
  age: integer,
  created_at: timestamp
} config {
  connector = 'csv',
  path = 'data/users.csv',
  has_header = true,
  delimiter = ','
}
```

**Example - Kafka Source:**
```renoir
source orders {
  order_id: bigint,
  user_id: bigint,
  product_id: bigint,
  amount: float,
  timestamp: timestamp
} config {
  connector = 'kafka',
  bootstrap_servers = 'localhost:9092',
  topic = 'orders',
  group_id = 'renoir_consumer'
}
```

### Sink Definitions

Sinks declare output destinations with their schema and connector configuration.

```renoir
sink <name> {
  <field_name>: <type>,
  <field_name>: <type>,
  ...
} config {
  connector = '<connector_type>',
  <key> = <value>,
  ...
}
```

**Example - CSV Sink:**
```renoir
sink user_stats {
  name: string,
  total_orders: bigint,
  total_spent: float
} config {
  connector = 'csv',
  path = 'output/user_stats.csv',
  has_header = true
}
```

**Example - Kafka Sink:**
```renoir
sink processed_orders {
  order_id: bigint,
  status: string,
  processed_at: timestamp
} config {
  connector = 'kafka',
  bootstrap_servers = 'localhost:9092',
  topic = 'processed_orders'
}
```

### Pipeline Definitions

Pipelines define the dataflow transformations from sources to sinks.

```renoir
pipeline <name> {
  <source_or_previous_operation>
    .<operation>(<params>)
    .<operation>(<params>)
    ...
    .sink(<sink_name>)
}
```

### Operations

#### Filter
Filter rows based on a predicate.

```renoir
.filter(<condition>)
```

**Examples:**
```renoir
.filter(age > 18)
.filter(status == 'active')
.filter(amount >= 100 && category == 'electronics')
```

#### Map (Select/Project)
Project and transform columns.

```renoir
.map(<column_list>)
.map(<column>, <column> as <alias>, ...)
```

**Examples:**
```renoir
.map(id, name, email)
.map(user_id, total * 1.1 as total_with_tax)
.map(*, age * 2 as double_age)  // keep all columns + new computed column
```

#### FlatMap
Expand one row into multiple rows.

```renoir
.flat_map(<expression>)
```

**Examples:**
```renoir
.flat_map(tags)  // explode array column
.flat_map(split(addresses, ';'))
```

#### Join
Join two streams.

```renoir
.join(<other_source>, <join_condition>, <join_type>)
```

**Examples:**
```renoir
.join(orders, users.id == orders.user_id, inner)
.join(products, orders.product_id == products.id, left)
```

**Join Types:**
- `inner`
- `left`
- `right`
- `full`

#### GroupBy and Aggregate
Group rows and compute aggregations.

```renoir
.group_by(<key_columns>)
  .aggregate(<aggregation_list>)
  .having(<condition>)?
```

**Examples:**
```renoir
.group_by(user_id)
  .aggregate(count(*) as order_count, sum(amount) as total_spent)

.group_by(category, country)
  .aggregate(avg(price) as avg_price, max(price) as max_price)
  .having(avg_price > 100)
```

**Aggregate Functions:**
- `count(<column>)` or `count(*)`
- `sum(<column>)`
- `avg(<column>)`
- `min(<column>)`
- `max(<column>)`

#### OrderBy
Sort the stream.

```renoir
.order_by(<column> [asc|desc] [nulls first|nulls last], ...)
```

**Examples:**
```renoir
.order_by(created_at desc)
.order_by(country asc, total desc)
.order_by(score desc nulls last)
```

#### Limit
Limit the number of rows.

```renoir
.limit(<count>)
.limit(<count>, offset = <offset>)
```

**Examples:**
```renoir
.limit(10)
.limit(100, offset = 50)
```

#### Distinct
Remove duplicate rows.

```renoir
.distinct()
```

**Example:**
```renoir
.distinct()
```

#### Split
Split the stream into multiple parallel paths where each item is cloned to all paths.

```renoir
.split() {
  path1: <operations> .sink(<sink1>),
  path2: <operations> .sink(<sink2>),
  ...
}
```

**Example:**
```renoir
.split() {
  clicks: .filter(event_type == 'click') .sink(click_events),
  purchases: .filter(event_type == 'purchase') .sink(purchase_events),
  all_events: .sink(archive_events)
}
```

#### Window
Define time-based windows for aggregations in streaming contexts.

```renoir
.window(<window_type>)
  .group_by(<keys>)
  .aggregate(<aggregations>)
```

**Window Types:**

**Tumbling Window:**
Fixed-size, non-overlapping windows.
```renoir
.window(tumbling(5.minutes))
.window(tumbling(1.hour))
.window(tumbling(30.seconds))
```

**Sliding Window:**
Fixed-size, overlapping windows with a slide interval.
```renoir
.window(sliding(10.minutes, slide = 5.minutes))
.window(sliding(1.hour, slide = 15.minutes))
```

**Session Window:**
Dynamic windows based on inactivity gaps.
```renoir
.window(session(30.minutes))
.window(session(5.minutes))
```

**Complete Example:**
```renoir
pipeline windowed_stats {
  events
    .window(tumbling(5.minutes))
    .group_by(user_id)
      .aggregate(
        count(*) as event_count,
        min(timestamp) as window_start,
        max(timestamp) as window_end
      )
    .sink(windowed_user_stats)
}
```

**Time Attributes:**
Windows operate on event time (requires a timestamp field):
```renoir
.window(tumbling(1.hour), event_time = timestamp)
```

### Complete Pipeline Examples

#### Example 1: Simple Filtering and Projection
```renoir
source users {
  id: bigint,
  name: string,
  email: string,
  age: integer,
  status: string
} config {
  connector = 'csv',
  path = 'users.csv'
}

sink active_adults {
  name: string,
  email: string
} config {
  connector = 'csv',
  path = 'output/active_adults.csv'
}

pipeline filter_users {
  users
    .filter(age >= 18 && status == 'active')
    .map(name, email)
    .sink(active_adults)
}
```

#### Example 2: Join and Aggregate
```renoir
source users {
  id: bigint,
  name: string,
  country: string
} config {
  connector = 'csv',
  path = 'users.csv'
}

source orders {
  order_id: bigint,
  user_id: bigint,
  amount: float,
  created_at: timestamp
} config {
  connector = 'kafka',
  bootstrap_servers = 'localhost:9092',
  topic = 'orders'
}

sink user_stats {
  name: string,
  country: string,
  order_count: bigint,
  total_spent: float
} config {
  connector = 'csv',
  path = 'output/user_stats.csv'
}

pipeline calculate_user_stats {
  users
    .join(orders, users.id == orders.user_id, inner)
    .group_by(users.id, users.name, users.country)
      .aggregate(
        count(*) as order_count,
        sum(orders.amount) as total_spent
      )
    .map(name, country, order_count, total_spent)
    .sink(user_stats)
}
```

#### Example 3: Multiple Pipelines with Split
```renoir
source events {
  event_id: bigint,
  user_id: bigint,
  event_type: string,
  timestamp: timestamp
} config {
  connector = 'kafka',
  bootstrap_servers = 'localhost:9092',
  topic = 'events'
}

sink click_events {
  user_id: bigint,
  timestamp: timestamp
} config {
  connector = 'kafka',
  bootstrap_servers = 'localhost:9092',
  topic = 'clicks'
}

sink purchase_events {
  user_id: bigint,
  timestamp: timestamp
} config {
  connector = 'kafka',
  bootstrap_servers = 'localhost:9092',
  topic = 'purchases'
}

sink all_events {
  event_id: bigint,
  user_id: bigint,
  event_type: string,
  timestamp: timestamp
} config {
  connector = 'csv',
  path = 'output/all_events.csv'
}

pipeline process_events {
  events
    .split() {
      clicks: .filter(event_type == 'click')
              .map(user_id, timestamp)
              .sink(click_events),
      purchases: .filter(event_type == 'purchase')
                 .map(user_id, timestamp)
                 .sink(purchase_events),
      archive: .sink(all_events)
    }
}
```

#### Example 4: Windowed Aggregation
```renoir
source sensor_readings {
  sensor_id: bigint,
  temperature: float,
  humidity: float,
  timestamp: timestamp
} config {
  connector = 'kafka',
  bootstrap_servers = 'localhost:9092',
  topic = 'sensors'
}

sink sensor_stats {
  sensor_id: bigint,
  avg_temperature: float,
  max_humidity: float,
  window_start: timestamp,
  window_end: timestamp
} config {
  connector = 'kafka',
  bootstrap_servers = 'localhost:9092',
  topic = 'sensor_stats'
}

pipeline calculate_sensor_stats {
  sensor_readings
    .window(tumbling(5.minutes), event_time = timestamp)
    .group_by(sensor_id)
      .aggregate(
        avg(temperature) as avg_temperature,
        max(humidity) as max_humidity,
        min(timestamp) as window_start,
        max(timestamp) as window_end
      )
    .sink(sensor_stats)
}
```

## Data Types

Supported data types:
- `integer` / `int` - 32-bit integer
- `bigint` / `long` - 64-bit integer
- `float` - 64-bit floating point
- `string` / `text` - UTF-8 string
- `boolean` / `bool` - Boolean value
- `timestamp` / `datetime` - Timestamp
- `date` - Date without time
- `time` - Time without date

## Expressions

### Comparison Operators
- `==` - Equal
- `!=` - Not equal
- `>` - Greater than
- `>=` - Greater than or equal
- `<` - Less than
- `<=` - Less than or equal

### Logical Operators
- `&&` / `and` - Logical AND
- `||` / `or` - Logical OR
- `!` / `not` - Logical NOT

### Arithmetic Operators
- `+` - Addition
- `-` - Subtraction
- `*` - Multiplication
- `/` - Division
- `%` - Modulo

### Null Handling
- `is null` - Check if value is null
- `is not null` - Check if value is not null

### String Operations
- `like` - Pattern matching (SQL-style)
- `contains` - Substring check
- String concatenation with `+` or `concat()`

### Subquery Support
```renoir
.filter(user_id in (select id from premium_users))
.filter(exists (select * from orders where orders.user_id == users.id))
```

## Comments

```renoir
// Single-line comment

/*
  Multi-line comment
  Can span multiple lines
*/
```

## Implementation Plan

### Phase 1: Core Language (MVP)
- [ ] Pest grammar for basic syntax
- [ ] Parser for source/sink definitions
- [ ] Parser for simple pipeline operations (filter, map)
- [ ] IR generation for basic operations
- [ ] Schema validation (integrate existing engine)

### Phase 2: Advanced Operations
- [ ] Join support
- [ ] GroupBy/Aggregate support
- [ ] OrderBy and Limit
- [ ] Distinct operation
- [ ] FlatMap support
- [ ] Split operator (requires IR implementation)
- [ ] Windowing operations (requires IR implementation)

### Phase 3: Enhanced Features
- [ ] Subquery support
- [ ] Complex expressions
- [ ] Type inference
- [ ] Error messages and diagnostics
- [ ] Config validation framework (requires IR implementation)

### Phase 4: Tooling
- [ ] Syntax highlighting (VSCode extension)
- [ ] Language server (LSP)
- [ ] Formatter
- [ ] Linter/Validator

## Open Questions

1. **Schema inference**: Should we support schema-less sources and infer from data?
2. **Time semantics**: Should we support both event time and processing time, or just event time?
3. **Watermarks**: How should we handle late-arriving data and watermark configuration?
4. **State management**: How to express stateful operations beyond windowed aggregations?
5. **UDFs**: Support for user-defined functions?
6. **Imports/Modules**: Should we support splitting definitions across files?
7. **Variables/Constants**: Should we support defining reusable constants?
8. **Window triggers**: Should we support custom firing triggers for windows?
9. **Split naming**: Are named paths in split() mandatory or optional?

## Future Considerations

- **Windowing operations** for time-based aggregations
- **Pattern matching** for complex event processing
- **Watermarks** for handling late-arriving data
- **Stateful operations** beyond simple aggregations
- **Custom functions** and UDFs
- **Schema evolution** handling
- **Multiple output sinks** from a single pipeline
- **Pipeline composition** and reuse
