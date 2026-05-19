use std::sync::Arc;

use rig_compose::{KernelError, LocalTool, ToolRegistry, ToolSchema};
use rig_mcp::serve_stdio;
use serde_json::{Value, json};

#[tokio::main]
async fn main() {
    if let Err(err) = serve_stdio(fixture_registry()).await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn fixture_registry() -> ToolRegistry {
    let registry = ToolRegistry::new();
    registry.register(Arc::new(LocalTool::new(
        ToolSchema {
            name: "weather.lookup".into(),
            description: "looks up weather for a city".into(),
            args_schema: json!({
                "type": "object",
                "required": ["city"],
                "properties": {
                    "city": {"type": "string"}
                }
            }),
            result_schema: json!({
                "type": "object",
                "properties": {
                    "city": {"type": "string"},
                    "forecast": {"type": "string"},
                    "transport": {"type": "string"}
                }
            }),
        },
        |args| async move {
            let city = args.get("city").and_then(Value::as_str).ok_or_else(|| {
                KernelError::InvalidArgument("weather.lookup requires city".into())
            })?;
            Ok(json!({
                "city": city,
                "forecast": "clear and cool",
                "transport": "stdio-mcp"
            }))
        },
    )));
    registry.register(Arc::new(LocalTool::new(
        ToolSchema {
            name: "math.checked_add".into(),
            description: "add two integers with argument validation".into(),
            args_schema: json!({
                "type": "object",
                "required": ["a", "b"],
                "properties": {
                    "a": {"type": "integer"},
                    "b": {"type": "integer"}
                }
            }),
            result_schema: json!({
                "type": "object",
                "properties": {
                    "sum": {"type": "integer"}
                }
            }),
        },
        |args| async move {
            let a = required_i64(&args, "a")?;
            let b = required_i64(&args, "b")?;
            let sum = a.checked_add(b).ok_or_else(|| {
                KernelError::InvalidArgument("math.checked_add overflowed".into())
            })?;
            Ok(json!({ "sum": sum }))
        },
    )));
    // Mid-RPC server exit. Used by tests/stdio_failures.rs to confirm
    // the client surfaces a typed `KernelError` (not a panic) when the
    // server vanishes after accepting the call but before responding.
    registry.register(Arc::new(LocalTool::new(
        ToolSchema {
            name: "diagnostics.exit_mid_rpc".into(),
            description: "terminate the server process before responding".into(),
            args_schema: json!({"type": "object"}),
            result_schema: json!({"type": "object"}),
        },
        |_args| async move {
            // Flush stderr so the test can attribute the exit if needed,
            // then drop the connection by terminating the process.
            std::process::exit(0);
            #[allow(unreachable_code)]
            Ok(json!({}))
        },
    )));
    // Oversized payload. Used by tests/result_envelope.rs to confirm
    // `rig_compose::bound_tool_result` is the supported truncation
    // surface for MCP responses (no per-transport bespoke clamping).
    registry.register(Arc::new(LocalTool::new(
        ToolSchema {
            name: "diagnostics.big_payload".into(),
            description: "return a deterministically oversized payload".into(),
            args_schema: json!({"type": "object"}),
            result_schema: json!({
                "type": "object",
                "properties": {
                    "blob": {"type": "string"},
                    "items": {"type": "array"}
                }
            }),
        },
        |_args| async move {
            let blob: String = std::iter::repeat_n('x', 10_000).collect();
            let items: Vec<Value> = (0..200i64).map(|i| json!(i)).collect();
            Ok(json!({ "blob": blob, "items": items }))
        },
    )));
    registry
}

fn required_i64(args: &Value, name: &str) -> Result<i64, KernelError> {
    args.get(name).and_then(Value::as_i64).ok_or_else(|| {
        KernelError::InvalidArgument(format!("math.checked_add requires integer {name}"))
    })
}
