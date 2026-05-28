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
- `result_cache` module providing the cached-paging primitives:
    `ResultCache` trait, `MemoryResultCache` in-memory store, opaque
    `CachedResultHandle`, `CachedResultEnvelope` JSON shape, and
    `cache_if_large` helper for swapping oversized arrays for an
    enveloped handle + deterministic first page
    ([src/result_cache.rs](src/result_cache.rs)).
- `CachedResultsTransport`, `CachedResultsConfig`, `cache.page`, and
    `cache.release` tool builders for opt-in model-boundary caching of
    oversized MCP array results while raw transports remain lossless
    ([src/cache_tools.rs](src/cache_tools.rs)).
- `RegistrationSnapshot` and `RegistrationReplayPolicy` for adapter-local
    reconnect replay of discovered remote tool schemas without adding replay
    state to `rig-compose::ToolRegistry`
    ([src/replay.rs](src/replay.rs)).
- Descriptor parity with `rig-compose` 0.4.1: loopback discovery and
    registry-backed snapshots use `ToolRegistry::descriptors()` instead of
    depending on registry storage details.
- Structured `tracing` spans around stdio spawn, client list/call, server
    list/call, and loopback list/call paths. Hosts can capture MCP lifecycle
    timing and errors through their existing subscriber/OTel setup without a
    hard dependency on `rig-tap`.

## Prototype Grade

- Tool-result bounding is validated for stdio outputs via the shared
    `rig-compose` envelope. Cached-paging primitives and opt-in transport
    integration exist; result search and schema projection are still deferred
    until host requirements are clearer.
- Registration replay snapshots are deterministic, idempotent at registry
    level, and aligned with the published `rig-compose` descriptor surface.
    Transport-specific reconnect loops, heartbeat policies, and in-flight call
    recovery are still host/adapter concerns.
- Transport tracing now emits structured spans for lifecycle timing/errors, but
    reconnect, heartbeat, and in-flight recovery policies remain host/adapter
    concerns.
- No alternate production transports are exposed; this is intentional until a concrete need appears.

## Next Work

1. Build on `result_cache`: add result search and schema/projection helpers if
    real hosts need more than page/release lifecycle tools.
2. Add timeout, heartbeat, in-flight call recovery, and background-job shapes
    for long-running MCP tools once host requirements are clearer.
3. Keep the `rmcp` feature surface tight; add new transports only behind a feature and a documented use case.

## Maturity Bar

- MCP-adapted tools are indistinguishable from local `rig-compose` tools for registry callers.
- Large tool outputs never enter a model window unbounded.
- Every transport failure maps to deterministic `KernelError` behavior.
- Local, loopback, and stdio paths share the same fixture semantics.

## Non-Goals

- Do not reimplement JSON-RPC framing, capability negotiation, or MCP protocol semantics outside `rmcp`.
- Do not add HTTP/TLS/proxy transports by default without a concrete host need.
- Do not make `rig-mcp` own policy, memory, or product approval workflows.
