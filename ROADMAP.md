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

## Prototype Grade

- Loopback has deterministic harness coverage; stdio needs broader fixture coverage against real child processes.
- Tool results can be large or tabular, but there is no result governor, cached paging, or projection API yet.
- Transport tracing exists mostly through tests/logs, not a shared cross-crate trace envelope.
- No alternate production transports are exposed; this is intentional until a concrete need appears.

## Next Work

1. Add stdio fixture coverage for discovery, tool calls, invalid args, child exit, malformed responses, and service teardown.
2. Design an MCP result governor: bounded inline result, cached large result, page/search/schema/release follow-up tools, and explicit truncation reasons.
3. Exercise the same dispatch and trace assertions for local tools, loopback tools, and stdio tools.
4. Add timeout, heartbeat, and background-job shapes for long-running MCP tools once host requirements are clearer.
5. Keep the `rmcp` feature surface tight; add new transports only behind a feature and a documented use case.

## Maturity Bar

- MCP-adapted tools are indistinguishable from local `rig-compose` tools for registry callers.
- Large tool outputs never enter a model window unbounded.
- Every transport failure maps to deterministic `KernelError` behavior.
- Local, loopback, and stdio paths share the same fixture semantics.

## Non-Goals

- Do not reimplement JSON-RPC framing, capability negotiation, or MCP protocol semantics outside `rmcp`.
- Do not add HTTP/TLS/proxy transports by default without a concrete host need.
- Do not make `rig-mcp` own policy, memory, or product approval workflows.
