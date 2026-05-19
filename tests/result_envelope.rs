//! Validates result bounding for MCP stdio tool outputs.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used,
    clippy::indexing_slicing
)]

use std::time::Duration;

use rig_compose::{KernelError, ToolResultEnvelope, ToolResultEnvelopeConfig};
use rig_mcp::{McpTransport, StdioTransport};
use serde_json::json;
use tokio::time::timeout;

const FIXTURE_BIN: &str = env!("CARGO_BIN_EXE_rig_mcp_stdio_fixture");

#[tokio::test]
async fn stdio_result_can_be_bounded_after_mcp_round_trip() -> Result<(), KernelError> {
    let transport = spawn_fixture().await?;

    let raw = transport
        .call_tool("diagnostics.big_payload", json!({}))
        .await?;
    let raw_chars = raw["blob"].as_str().expect("blob").chars().count();
    let raw_items = raw["items"].as_array().expect("items").len();
    assert_eq!(raw_chars, 10_000, "stdio transport should preserve raw structured MCP output");
    assert_eq!(raw_items, 200);

    let config = ToolResultEnvelopeConfig::default();
    let envelope = ToolResultEnvelope::bound(raw, &config);
    assert!(envelope.truncated);
    assert!(envelope.omitted_chars > 0);
    assert!(envelope.omitted_items > 0);
    assert!(envelope.page_token.is_some());

    let bounded_chars = envelope.payload["blob"].as_str().expect("bounded blob").chars().count();
    let bounded_items = envelope.payload["items"].as_array().expect("bounded items").len();
    assert!(bounded_chars < raw_chars, "blob should be truncated from {raw_chars} chars");
    assert!(bounded_items < raw_items, "items should be truncated from {raw_items} items");

    Ok(())
}

#[tokio::test]
async fn custom_envelope_config_bounds_stdio_result() -> Result<(), KernelError> {
    let transport = spawn_fixture().await?;
    let raw = transport
        .call_tool("diagnostics.big_payload", json!({}))
        .await?;

    let config = ToolResultEnvelopeConfig::new(128).with_max_array_items(8);
    let envelope = ToolResultEnvelope::bound(raw, &config);

    assert!(envelope.truncated);
    assert_eq!(
        envelope.payload["blob"]
            .as_str()
            .expect("bounded blob")
            .chars()
            .count(),
        128
    );
    assert_eq!(
        envelope.payload["items"]
            .as_array()
            .expect("bounded items")
            .len(),
        8
    );

    let encoded = serde_json::to_string(&envelope).expect("serialize envelope");
    let decoded: ToolResultEnvelope = serde_json::from_str(&encoded).expect("deserialize envelope");
    assert_eq!(decoded, envelope);

    Ok(())
}

async fn spawn_fixture() -> Result<StdioTransport, KernelError> {
    timeout(
        Duration::from_secs(5),
        StdioTransport::spawn("stdio://fixture", FIXTURE_BIN, &[]),
    )
    .await
    .map_err(|_| KernelError::ToolFailed("stdio fixture timed out".into()))?
}
