use anyhow::Context;
use serde_json::json;

use anyhow::Result;
use serde::{Deserialize, Serialize};

async fn test_where_on_aggregated_field() -> Result<()> {
    let client = clickhouse::Client::default()
        .with_url(
            std::env::var("CLICKHOUSE_HOST")
                .context("CLICKHOUSE_HOST must be set")?,
        )
        .with_user("default")
        .with_password(
            std::env::var("CLICKHOUSE_PASSWORD")
                .context("CLICKHOUSE_PASSWORD must be set")?,
        )
        // All these settings can instead be applied on the query or insert level with the same `with_option` method.
        // Enable new JSON type usage
        .with_option("enable_json_type", "1")
        // Enable inserting JSON columns as a string
        .with_option("input_format_binary_read_json_as_string", "1")
        // Enable selecting JSON columns as a string
        .with_option("output_format_binary_write_json_as_string", "1")
        .with_database("sam_test");

    let ddl = r#"
CREATE DATABASE IF NOT EXISTS sam_test;

DROP VIEW IF EXISTS sam_test.event_view;
DROP VIEW IF EXISTS sam_test.event_mv;
DROP TABLE IF EXISTS sam_test.event_agg_table;
DROP TABLE IF EXISTS sam_test.event_insert_table;

CREATE TABLE sam_test.event_insert_table (
    event_id String,
    event_data Json,
)
ENGINE = MergeTree()
ORDER BY event_id 
SETTINGS enable_json_type = 1;

CREATE TABLE sam_test.event_agg_table (
    event_id String,
    event_data AggregateFunction(argMax, Json, String),
)
ENGINE = AggregatingMergeTree()
ORDER BY event_id 
SETTINGS enable_json_type = 1;


CREATE MATERIALIZED VIEW sam_test.event_mv
TO sam_test.event_agg_table AS
SELECT
    event_id,
    argMaxState(event_data, event_id) AS event_data
FROM sam_test.event_insert_table
GROUP BY event_id;

CREATE VIEW sam_test.event_view AS
SELECT
    event_id,
    argMaxMerge(event_data) AS event_data
FROM sam_test.event_agg_table
GROUP BY event_id;

    "#;
    for statement in ddl.split(";") {
        let statement = statement.trim();
        if statement.is_empty() {
            continue;
        }
        tracing::info!("Executing statement: {}", statement);
        client.query(statement).execute().await?;
    }

    #[derive(Debug, Deserialize, Serialize, clickhouse::Row)]
    struct EventRow {
        event_id: String,
        event_data: String,
    }

    let row = EventRow {
        event_id: "event_id1".to_string(),
        event_data: serde_json::to_string(&json!({
            "fizz": "buzz",
            "foo": "bar",
        }))
        .unwrap(),
    };

    // This insert succeeds.
    let mut insert = client.insert("sam_test.event_insert_table")?;
    insert.write(&row).await?;
    insert.end().await?;

    // This query succeeds.
    let query = client.query("SELECT event_id, event_data FROM sam_test.event_view");
    tracing::info!("query: {}", query.sql_display().to_string());
    let events = query.fetch_all::<EventRow>().await?;
    tracing::info!("rows: {}", serde_json::to_string_pretty(&events)?);

    // This query fails due to the WHERE clause. Is there a way to make this work?
    let query = client.query(
        "SELECT event_id, event_data FROM sam_test.event_view WHERE event_data.fizz = 'buzz'",
    );
    tracing::info!("query: {}", query.sql_display().to_string());
    let events = query.fetch_all::<EventRow>().await?;
    tracing::info!("rows: {}", serde_json::to_string_pretty(&events)?);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug")),
        )
        .init();

    test_where_on_aggregated_field().await?;

    Ok(())
}
