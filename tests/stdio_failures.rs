//! Deterministic stdio MCP failure fixtures.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]

use std::time::Duration;

use rig_compose::KernelError;
use rig_mcp::{McpTransport, StdioTransport};
use serde_json::json;
use tokio::time::timeout;

const FIXTURE_BIN: &str = env!("CARGO_BIN_EXE_rig_mcp_stdio_fixture");

#[tokio::test]
async fn stdio_fixture_lists_and_calls_tool() -> Result<(), KernelError> {
    let transport = spawn_fixture().await?;

    let schemas = transport.list_tools().await?;
    assert!(
        schemas
            .iter()
            .any(|schema| schema.name == "math.checked_add")
    );

    let output = transport
        .call_tool("math.checked_add", json!({"a": 20, "b": 22}))
        .await?;
    assert_eq!(output, json!({"sum": 42}));

    Ok(())
}

#[tokio::test]
async fn stdio_unknown_tool_returns_tool_failed() -> Result<(), KernelError> {
    let transport = spawn_fixture().await?;

    let err = transport
        .call_tool("math.missing", json!({"a": 1, "b": 2}))
        .await
        .unwrap_err();

    assert_tool_failed_contains(err, "math.missing");
    Ok(())
}

#[tokio::test]
async fn stdio_missing_required_argument_returns_tool_failed() -> Result<(), KernelError> {
    let transport = spawn_fixture().await?;

    let err = transport
        .call_tool("math.checked_add", json!({"a": 1}))
        .await
        .unwrap_err();

    assert_tool_failed_contains(err, "integer b");
    Ok(())
}

#[tokio::test]
async fn stdio_wrong_argument_type_returns_tool_failed() -> Result<(), KernelError> {
    let transport = spawn_fixture().await?;

    let err = transport
        .call_tool("math.checked_add", json!({"a": "one", "b": 2}))
        .await
        .unwrap_err();

    assert_tool_failed_contains(err, "integer a");
    Ok(())
}

#[tokio::test]
async fn stdio_rejects_non_object_arguments_before_rpc() -> Result<(), KernelError> {
    let transport = spawn_fixture().await?;

    let err = transport
        .call_tool("math.checked_add", json!([1, 2]))
        .await
        .unwrap_err();

    assert!(matches!(err, KernelError::InvalidArgument(_)));
    Ok(())
}

#[tokio::test]
async fn stdio_malformed_child_output_fails_connect() {
    let result = timeout(
        Duration::from_secs(5),
        StdioTransport::spawn(
            "stdio://malformed",
            "/bin/sh",
            &["-c", "printf 'not-json\\n'"],
        ),
    )
    .await
    .expect("malformed child fixture timed out");

    assert!(matches!(result, Err(KernelError::ToolFailed(_))));
}

#[tokio::test]
async fn stdio_child_exit_before_handshake_fails_connect() {
    let result = timeout(
        Duration::from_secs(5),
        StdioTransport::spawn("stdio://early-exit", "/bin/sh", &["-c", "exit 0"]),
    )
    .await
    .expect("early-exit child fixture timed out");

    assert!(matches!(result, Err(KernelError::ToolFailed(_))));
}

/// Server completes the MCP handshake, accepts the `tools/call`, then
/// terminates without sending a response. The client must surface a
/// typed `KernelError::ToolFailed` rather than hanging or panicking.
#[tokio::test]
async fn stdio_child_exit_mid_rpc_surfaces_tool_failed() -> Result<(), KernelError> {
    let transport = spawn_fixture().await?;

    let result = timeout(
        Duration::from_secs(5),
        transport.call_tool("diagnostics.exit_mid_rpc", json!({})),
    )
    .await
    .expect("mid-rpc exit fixture timed out");

    let err = result.expect_err("server vanished mid-rpc; expected an error");
    assert!(matches!(err, KernelError::ToolFailed(_)));
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

fn assert_tool_failed_contains(err: KernelError, expected: &str) {
    let KernelError::ToolFailed(message) = err else {
        panic!("expected KernelError::ToolFailed");
    };
    assert!(
        message.contains(expected),
        "expected `{message}` to contain `{expected}`"
    );
}
