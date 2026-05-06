# Changelog

All notable changes to `rig-mcp` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Versions are managed automatically by [release-plz](https://release-plz.dev/)
from [Conventional Commits](https://www.conventionalcommits.org/).

## [Unreleased]

## [0.1.2](https://github.com/ForeverAngry/rig-mcp/compare/v0.1.1...v0.1.2) - 2026-05-06

### Fixed

- Depend on released rig-compose
- Remove stdio transport mutex

### Fixed

- `StdioTransport` now stores the `rmcp` peer handle directly and drops
  its `tokio::sync::Mutex`, eliminating cross-await lock contention and
  letting concurrent `tools/list` / `tools/call` RPCs proceed in parallel.

## [0.1.1](https://github.com/ForeverAngry/rig-mcp/compare/v0.1.0...v0.1.1) - 2026-05-04

### Fixed

- Identify server as rig-mcp instead of rig-compose
- Depend on rig-compose from crates.io (drop sibling path)

## [0.1.0] - Unreleased

### Added

- Initial release of the Model Context Protocol bridge for `rig-compose`.
- `McpTransport` trait abstracting MCP client transports.
- `LoopbackTransport` for in-process round-trips against a local
  `ToolRegistry` (testing, embedding).
- `StdioTransport::spawn` and `StdioClient` for newline-framed JSON-RPC
  over a child process's stdio, backed by the official `rmcp` SDK.
- `serve_stdio()` for exposing any `ToolRegistry` as an MCP server endpoint.
- `McpTool` adapter so MCP-served tools are indistinguishable from local
  `rig_compose::Tool` implementations.
