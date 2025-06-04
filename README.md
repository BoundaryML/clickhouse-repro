# Repro instructions

Trying to figure out how to do a WHERE clause on an aggregated field.

```
export CLICKHOUSE_HOST=...
export CLICKHOUSE_PASSWORD=...
cargo run
```

yields this error message:

```
Error: bad response: Code: 47. DB::Exception: Missing columns: 'event_data.fizz' while processing: 'SELECT event_id, argMaxMerge(event_data) AS event_data FROM sam_test.event_agg_table WHERE event_data.fizz = 'buzz' GROUP BY event_id', required columns: 'event_id' 'event_data.fizz' 'event_data', maybe you meant: 'event_id' or 'event_data'. (UNKNOWN_IDENTIFIER) (version 25.4.1.37073 (official build))
failed to wait for command termination: exit status 1
```
