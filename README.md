# rig-mcp

[![CI](https://github.com/ForeverAngry/rig-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/ForeverAngry/rig-mcp/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/rig-mcp.svg)](https://crates.io/crates/rig-mcp)
[![docs.rs](https://img.shields.io/docsrs/rig-mcp)](https://docs.rs/rig-mcp)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![MSRV](https://img.shields.io/badge/rustc-1.88+-orange.svg)](#rust-version)

[Model Context Protocol](https://modelcontextprotocol.io/) bridge for
[`rig-compose`](https://crates.io/crates/rig-compose) tool registries,
backed by the official [`rmcp`](https://crates.io/crates/rmcp) SDK.

## Install

```toml
[dependencies]
rig-compose = "0.1"
rig-mcp     = "0.1"
```

## What you get

- `McpTransport` — trait abstracting client transports.
- `LoopbackTransport` — in-process round-trip against a local
  `ToolRegistry`. Useful for tests and embedding.
- `StdioTransport::spawn` / `StdioClient` — newline-framed JSON-RPC over a
  child process's stdio.
- `serve_stdio()` — exposes any `ToolRegistry` as an MCP server endpoint.
- `McpTool` — adapts a transport into a `rig_compose::Tool`, so the same
  skill chain works against local and remote tools indistinguishably.

All spec-level concerns (JSON-RPC framing, capability handshakes,
protocol-version negotiation) are delegated to `rmcp`; this crate keeps the
adapter surface small.

## Rust version

The crate targets Rust **1.88** (edition 2024). MSRV bumps follow the
[Rig contributing policy](https://github.com/0xPlaygrounds/rig/blob/main/CONTRIBUTING.md)
and ship as a `feat!:` change.

## License

Dual-licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual-licensed as above, without any additional terms or conditions.
