# rig-mcp Roadmap

This roadmap is the crate-local operating plan for `rig-mcp`. The cross-crate coordination summary lives in [`rig-ecosystem/docs/roadmap.md`](../rig-ecosystem/docs/roadmap.md).

## Role

`rig-mcp` bridges Model Context Protocol endpoints into `rig-compose` tool registries. It delegates protocol mechanics to the official `rmcp` SDK and keeps the public surface focused on Rig-shaped transports and tools.

## Landed

- `McpTransport` trait for endpoint, tool discovery, and tool calls.
- `LoopbackTransport` for deterministic in-process MCP-like round trips.
- `McpTool` adapter that wraps remote MCP schemas as `rig_compose::Tool` values.
- `StdioTransport::spawn` for child-process stdio MCP clients backed by `rmcp`.
- `serve_stdio` for exposing a local `ToolRegistry` as a tools-only MCP server.
- Deterministic loopback harness test that records endpoint, discovered tools, normalized invocation, adapted dispatch result, final answer, and assertions.
- Cloneable `rmcp` peer handling without a transport-level async mutex around concurrent RPC calls.
- Stdio fixture coverage for discovery, tool calls, invalid args, child exit,
  malformed responses, and service teardown
  ([tests/stdio_failures.rs](tests/stdio_failures.rs)).
- Result-envelope coverage for oversized stdio payloads: MCP transports
    preserve structured results, and callers can apply
    `rig_compose::bound_tool_result` for deterministic truncation metadata
    ([tests/result_envelope.rs](tests/result_envelope.rs)).
- Shared local / loopback / stdio harness coverage for the same tool task,
    proving registry callers get equivalent dispatch semantics across paths
    ([tests/harness.rs](tests/harness.rs)).

## Prototype Grade

- Tool-result bounding is validated for stdio outputs via the shared
    `rig-compose` envelope. Cached paging, result search, schema projection,
    and release lifecycle are not designed yet.
- Transport tracing exists mostly through tests/logs, not a shared cross-crate trace envelope.
- No alternate production transports are exposed; this is intentional until a concrete need appears.

## Next Work

1. Design the next result-governor layer beyond `ToolResultEnvelope`: cached
     large results, page/search/schema/release follow-up tools, and explicit
     release lifecycle.
2. Add structured `tracing` spans around spawn/list/call/server dispatch so
     host apps can capture MCP lifecycle without a hard dependency on `rig-tap`.
3. Add timeout, heartbeat, and background-job shapes for long-running MCP tools once host requirements are clearer.
4. Keep the `rmcp` feature surface tight; add new transports only behind a feature and a documented use case.

## Maturity Bar

- MCP-adapted tools are indistinguishable from local `rig-compose` tools for registry callers.
- Large tool outputs never enter a model window unbounded.
- Every transport failure maps to deterministic `KernelError` behavior.
- Local, loopback, and stdio paths share the same fixture semantics.

## Non-Goals

- Do not reimplement JSON-RPC framing, capability negotiation, or MCP protocol semantics outside `rmcp`.
- Do not add HTTP/TLS/proxy transports by default without a concrete host need.
- Do not make `rig-mcp` own policy, memory, or product approval workflows.
