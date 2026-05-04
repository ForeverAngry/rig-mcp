# AGENTS.md

Guidance for AI coding agents working in `rig-mcp`.

## Project

Model Context Protocol bridge for [`rig-compose`](https://crates.io/crates/rig-compose)
tool registries, backed by the official [`rmcp`](https://crates.io/crates/rmcp)
SDK. Surfaces:

- `McpTransport` — trait abstraction over MCP client transports.
- `LoopbackTransport` — in-process round-trip for tests.
- `StdioTransport::spawn` / `serve_stdio()` — newline-framed JSON-RPC over
  child-process stdio.
- `McpTool` — adapts a transport into a `rig_compose::Tool`.

## Rules

- Rust 2024, MSRV 1.88.
- Errors: surface `KernelError` from `rig-compose`; do not invent String
  error variants.
- Never `.await` while holding a lock guard.
- No `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!`/`dbg!`/indexing
  in library code (clippy deny/forbid). Allowed in `#[cfg(test)]`.
- Use `tracing` for logs.
- Document new `pub` items with `///` rustdoc.
- Delegate JSON-RPC framing, capability handshakes, and protocol-version
  negotiation to `rmcp`; do not reimplement the spec here.

## Validation

```sh
just check
# fmt + clippy --all-features + test --all-features + rustdoc strict
```

## Scope

Do not vendor `rmcp`. Keep the `rmcp` feature surface tight (currently
`client`, `server`, `macros`, `transport-io`, `transport-child-process`).
Update [README.md](README.md) and [CHANGELOG.md](CHANGELOG.md) for
user-visible changes.
