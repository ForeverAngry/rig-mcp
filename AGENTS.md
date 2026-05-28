# AGENTS.md

Guidance for AI coding agents working in `rig-mcp`.

## Project

Model Context Protocol bridge for [`rig-compose`](https://crates.io/crates/rig-compose)
tool registries, backed by the official [`rmcp`](https://crates.io/crates/rmcp)
SDK. Surfaces:

- `McpTransport` — trait abstraction over MCP client transports.
- `LoopbackTransport` — in-process round-trip for tests.
- `StdioTransport::spawn` / `serve_stdio()` — newline-framed JSON-RPC over
  child-process stdio. Gated behind the default-on `stdio` Cargo feature.
- `McpTool` — adapts a transport into a `rig_compose::Tool`.
- `CachedResultsTransport` + `CachedResultsConfig` and the `cache.page` /
  `cache.release` tool builders (`register_cache_tools`) for opt-in
  model-boundary paging of oversized array results.
- `RegistrationSnapshot` + `RegistrationReplayPolicy` for adapter-local
  replay of discovered MCP tool registrations after reconnects.
- `ResultCache` + `MemoryResultCache` + `CachedResultEnvelope` +
  `CachedResultHandle` + `cache_if_large` — transport-neutral cache primitives
  shared by the cache tools.

## Rules

- Rust 2024, MSRV 1.88. Library is runtime-agnostic when built with
  `--no-default-features`; the `stdio` feature is what pulls `tokio` and the
  matching `rmcp` transport features. Do not add unconditional `tokio` to
  `[dependencies]`.
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
# fmt + clippy --all-features + clippy --no-default-features
# + test --all-features + rustdoc strict
```

CI additionally builds `--no-default-features` to keep the runtime-agnostic
surface honest.

## Scope

Do not vendor `rmcp`. Keep the `rmcp` feature surface tight: the base set is
`client`, `server`, `macros`; the `stdio` feature additionally enables
`transport-io` and `transport-child-process`. New transports should follow the
same opt-in feature pattern rather than expanding the default surface.
Update [README.md](README.md) and [CHANGELOG.md](CHANGELOG.md) for
user-visible changes.
