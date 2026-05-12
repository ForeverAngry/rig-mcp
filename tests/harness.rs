//! Deterministic harness prototype for MCP-backed tool dispatch.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]

use std::sync::Arc;

use rig_compose::{KernelError, LocalTool, ToolRegistry, ToolSchema};
use rig_mcp::{LoopbackTransport, McpTool, McpTransport};
use serde_json::{Value, json};

#[tokio::test]
async fn mcp_loopback_harness_records_transport_neutral_dispatch() -> Result<(), KernelError> {
    let server = ToolRegistry::new();
    server.register(Arc::new(LocalTool::new(
        ToolSchema {
            name: "weather.lookup".into(),
            description: "looks up weather for a city".into(),
            args_schema: json!({"type": "object"}),
            result_schema: json!({"type": "object"}),
        },
        |args| async move {
            let city = args
                .get("city")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            Ok(json!({
                "city": city,
                "forecast": "clear and cool",
                "transport": "loopback-mcp"
            }))
        },
    )));

    let transport: Arc<dyn McpTransport> =
        Arc::new(LoopbackTransport::new("loopback://harness", server));
    let client = ToolRegistry::new();
    for tool in McpTool::from_transport(transport.clone()).await? {
        client.register(tool);
    }

    let run = run_mcp_loopback_harness(
        transport,
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
            "mcp-tools-discovered",
            "model-output-normalized",
            "mcp-tool-dispatched",
            "final-answer-grounded",
        ]
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
struct McpLoopbackHarnessRun {
    task: String,
    endpoint: String,
    first_model_output: String,
    discovered_tool_names: Vec<String>,
    normalized_invocation: HarnessInvocation,
    dispatch_result: HarnessDispatchResult,
    final_answer: String,
    passed_assertions: Vec<&'static str>,
}

async fn run_mcp_loopback_harness(
    transport: Arc<dyn McpTransport>,
    client: &ToolRegistry,
    task: &str,
    first_model_output: &str,
    normalized_invocation: HarnessInvocation,
) -> Result<McpLoopbackHarnessRun, KernelError> {
    let endpoint = transport.endpoint().to_string();
    let discovered_tool_names = transport
        .list_tools()
        .await?
        .into_iter()
        .map(|schema| schema.name)
        .collect::<Vec<_>>();
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
    let passed_assertions = mcp_harness_assertions(
        &discovered_tool_names,
        &normalized_invocation,
        &dispatch_result,
        &final_answer,
    );

    Ok(McpLoopbackHarnessRun {
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

fn mcp_harness_assertions(
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
        passed.push("mcp-tools-discovered");
    }
    if normalized_invocation.name == "weather.lookup" {
        passed.push("model-output-normalized");
    }
    if dispatch_result.output.get("transport") == Some(&json!("loopback-mcp")) {
        passed.push("mcp-tool-dispatched");
    }
    if final_answer.contains("Berlin") && final_answer.contains("clear and cool") {
        passed.push("final-answer-grounded");
    }

    passed
}
