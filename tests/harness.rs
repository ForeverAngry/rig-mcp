//! Deterministic harness prototype for MCP-backed tool dispatch.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]

use std::sync::Arc;
use std::time::Duration;

use rig_compose::{KernelError, LocalTool, ToolRegistry, ToolSchema};
use rig_mcp::{LoopbackTransport, McpTool, McpTransport, StdioTransport};
use serde_json::{Value, json};
use tokio::time::timeout;

const FIXTURE_BIN: &str = env!("CARGO_BIN_EXE_rig_mcp_stdio_fixture");

#[tokio::test]
async fn harness_records_loopback_dispatch() -> Result<(), KernelError> {
    let server = ToolRegistry::new();
    server.register(weather_tool("loopback-mcp"));

    let transport: Arc<dyn McpTransport> =
        Arc::new(LoopbackTransport::new("loopback://harness", server));
    let client = ToolRegistry::new();
    for tool in McpTool::from_transport(transport.clone()).await? {
        client.register(tool);
    }

    let run = run_transport_harness(
        "mcp-loopback",
        Some(transport),
        &client,
        "What is the weather like in Berlin today?",
        "weather.lookup(city='Berlin')",
        HarnessInvocation {
            name: "weather.lookup".into(),
            args: json!({"city": "Berlin"}),
        },
    )
    .await?;

    assert_eq!(run.task, "What is the weather like in Berlin today?");
    assert_eq!(run.endpoint, "loopback://harness");
    assert_eq!(run.path, "mcp-loopback");
    assert_eq!(run.first_model_output, "weather.lookup(city='Berlin')");
    assert_eq!(run.discovered_tool_names, vec!["weather.lookup"]);
    assert_eq!(run.dispatch_result.invocation.name, "weather.lookup");
    assert_eq!(
        run.dispatch_result.output.get("transport"),
        Some(&json!("loopback-mcp"))
    );
    assert!(run.final_answer.contains("Berlin"));
    assert!(run.final_answer.contains("clear and cool"));
    assert_eq!(
        run.passed_assertions,
        vec![
            "tools-discovered",
            "model-output-normalized",
            "tool-dispatched",
            "final-answer-grounded",
        ]
    );

    Ok(())
}

#[tokio::test]
async fn harness_records_local_loopback_and_stdio_parity() -> Result<(), KernelError> {
    let local = ToolRegistry::new();
    local.register(weather_tool("local"));
    let local_run = run_transport_harness(
        "local",
        None,
        &local,
        "What is the weather like in Berlin today?",
        "weather.lookup(city='Berlin')",
        weather_invocation(),
    )
    .await?;

    let loopback_server = ToolRegistry::new();
    loopback_server.register(weather_tool("loopback-mcp"));
    let loopback_transport: Arc<dyn McpTransport> = Arc::new(LoopbackTransport::new(
        "loopback://harness-parity",
        loopback_server,
    ));
    let loopback_client = ToolRegistry::new();
    for tool in McpTool::from_transport(loopback_transport.clone()).await? {
        loopback_client.register(tool);
    }
    let loopback_run = run_transport_harness(
        "mcp-loopback",
        Some(loopback_transport),
        &loopback_client,
        "What is the weather like in Berlin today?",
        "weather.lookup(city='Berlin')",
        weather_invocation(),
    )
    .await?;

    let stdio_transport: Arc<dyn McpTransport> = Arc::new(spawn_fixture().await?);
    let stdio_client = ToolRegistry::new();
    for tool in McpTool::from_transport(stdio_transport.clone()).await? {
        stdio_client.register(tool);
    }
    let stdio_run = run_transport_harness(
        "mcp-stdio",
        Some(stdio_transport),
        &stdio_client,
        "What is the weather like in Berlin today?",
        "weather.lookup(city='Berlin')",
        weather_invocation(),
    )
    .await?;

    for run in [&local_run, &loopback_run, &stdio_run] {
        assert_eq!(run.normalized_invocation, weather_invocation());
        assert_eq!(
            run.passed_assertions,
            vec![
                "tools-discovered",
                "model-output-normalized",
                "tool-dispatched",
                "final-answer-grounded",
            ]
        );
        assert!(run.final_answer.contains("Berlin"));
        assert!(run.final_answer.contains("clear and cool"));
    }

    assert_eq!(local_run.endpoint, "local://registry");
    assert_eq!(loopback_run.endpoint, "loopback://harness-parity");
    assert_eq!(stdio_run.endpoint, "stdio://fixture");
    assert_eq!(
        local_run.dispatch_result.output.get("transport"),
        Some(&json!("local"))
    );
    assert_eq!(
        loopback_run.dispatch_result.output.get("transport"),
        Some(&json!("loopback-mcp"))
    );
    assert_eq!(
        stdio_run.dispatch_result.output.get("transport"),
        Some(&json!("stdio-mcp"))
    );

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
struct HarnessInvocation {
    name: String,
    args: Value,
}

#[derive(Debug, Clone, PartialEq)]
struct HarnessDispatchResult {
    invocation: HarnessInvocation,
    output: Value,
}

#[derive(Debug, Clone, PartialEq)]
struct TransportHarnessRun {
    path: String,
    task: String,
    endpoint: String,
    first_model_output: String,
    discovered_tool_names: Vec<String>,
    normalized_invocation: HarnessInvocation,
    dispatch_result: HarnessDispatchResult,
    final_answer: String,
    passed_assertions: Vec<&'static str>,
}

async fn run_transport_harness(
    path: &str,
    transport: Option<Arc<dyn McpTransport>>,
    client: &ToolRegistry,
    task: &str,
    first_model_output: &str,
    normalized_invocation: HarnessInvocation,
) -> Result<TransportHarnessRun, KernelError> {
    let (endpoint, discovered_tool_names) = if let Some(transport) = transport {
        let endpoint = transport.endpoint().to_string();
        let discovered_tool_names = transport
            .list_tools()
            .await?
            .into_iter()
            .map(|schema| schema.name)
            .collect::<Vec<_>>();
        (endpoint, discovered_tool_names)
    } else {
        (
            "local://registry".to_string(),
            client
                .schemas()
                .into_iter()
                .map(|schema| schema.name)
                .collect(),
        )
    };
    let output = client
        .invoke(
            &normalized_invocation.name,
            normalized_invocation.args.clone(),
        )
        .await?;
    let dispatch_result = HarnessDispatchResult {
        invocation: normalized_invocation.clone(),
        output,
    };
    let final_answer = fake_second_model_turn(&dispatch_result);
    let passed_assertions = harness_assertions(
        &discovered_tool_names,
        &normalized_invocation,
        &dispatch_result,
        &final_answer,
    );

    Ok(TransportHarnessRun {
        path: path.to_string(),
        task: task.to_string(),
        endpoint,
        first_model_output: first_model_output.to_string(),
        discovered_tool_names,
        normalized_invocation,
        dispatch_result,
        final_answer,
        passed_assertions,
    })
}

fn fake_second_model_turn(result: &HarnessDispatchResult) -> String {
    let city = result
        .output
        .get("city")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let forecast = result
        .output
        .get("forecast")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    format!("The weather in {city} is {forecast}.")
}

fn harness_assertions(
    discovered_tool_names: &[String],
    normalized_invocation: &HarnessInvocation,
    dispatch_result: &HarnessDispatchResult,
    final_answer: &str,
) -> Vec<&'static str> {
    let mut passed = Vec::new();

    if discovered_tool_names
        .iter()
        .any(|name| name == &normalized_invocation.name)
    {
        passed.push("tools-discovered");
    }
    if normalized_invocation.name == "weather.lookup" {
        passed.push("model-output-normalized");
    }
    if dispatch_result.output.get("transport").is_some() {
        passed.push("tool-dispatched");
    }
    if final_answer.contains("Berlin") && final_answer.contains("clear and cool") {
        passed.push("final-answer-grounded");
    }

    passed
}

fn weather_invocation() -> HarnessInvocation {
    HarnessInvocation {
        name: "weather.lookup".into(),
        args: json!({"city": "Berlin"}),
    }
}

fn weather_tool(transport: &'static str) -> Arc<LocalTool> {
    Arc::new(LocalTool::new(
        ToolSchema {
            name: "weather.lookup".into(),
            description: "looks up weather for a city".into(),
            args_schema: json!({"type": "object"}),
            result_schema: json!({"type": "object"}),
        },
        move |args| async move {
            let city = args.get("city").and_then(Value::as_str).ok_or_else(|| {
                KernelError::InvalidArgument("weather.lookup requires city".into())
            })?;
            Ok(json!({
                "city": city,
                "forecast": "clear and cool",
                "transport": transport
            }))
        },
    ))
}

async fn spawn_fixture() -> Result<StdioTransport, KernelError> {
    timeout(
        Duration::from_secs(5),
        StdioTransport::spawn("stdio://fixture", FIXTURE_BIN, &[]),
    )
    .await
    .map_err(|_| KernelError::ToolFailed("stdio fixture timed out".into()))?
}
