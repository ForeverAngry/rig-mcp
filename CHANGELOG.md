# Changelog

<!-- markdownlint-disable MD024 -->

All notable changes to `rig-mcp` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Versions are managed automatically by [release-plz](https://release-plz.dev/)
from [Conventional Commits](https://www.conventionalcommits.org/).

## [Unreleased]

### Added

- Add `CachedResultsTransport`, `CachedResultsConfig`, and cache page/release
  tools for opt-in model-boundary caching of oversized MCP array results.

## [0.2.1](https://github.com/ForeverAngry/rig-mcp/compare/v0.2.0...v0.2.1) - 2026-05-28

### Added

- Align replay snapshots with compose descriptors

### Changed

- Require `rig-compose` 0.4.1 and use `ToolRegistry::descriptors()` for
  loopback discovery and registry-backed registration snapshots.

## [0.2.0](https://github.com/ForeverAngry/rig-mcp/compare/v0.1.5...v0.2.0) - 2026-05-27

### Added

- Add cached result metadata and replay snapshots

### Added

- Extend `CachedResultEnvelope` with `truncated`, `omitted_items`, and
  `page_token` metadata so MCP cached array previews align with
  `rig-compose` result-envelope semantics while preserving page handles.
- Add `RegistrationSnapshot` and `RegistrationReplayPolicy` for deterministic,
  adapter-local replay of discovered MCP tool registrations after reconnects.

## [0.1.5](https://github.com/ForeverAngry/rig-mcp/compare/v0.1.4...v0.1.5) - 2026-05-27

### Added

- Add result cache primitives

### Fixed

- Make harness example self-contained

### Added

- `result_cache` module: transport-neutral cache for paged large tool
  results. Public surface: `ResultCache` trait, `MemoryResultCache`
  in-memory impl, `CachedResultHandle` / `CachedResultEnvelope` JSON
  types, and a `cache_if_large` helper that envelopes oversized arrays
  with a handle, total item count, page size, and a deterministic first
  page. Wires the foundation for the page/search/release follow-up tools
  called out in ROADMAP "Next Work" item 1; transport-level integration
  remains a separate downstream change.

## [0.1.4](https://github.com/ForeverAngry/rig-mcp/compare/v0.1.3...v0.1.4) - 2026-05-19

### Added

- Enhance stdio transport with weather lookup tool and result bounding tests
- *(stdio)* Add stdio bin + stdio_failures integration tests ([#6](https://github.com/ForeverAngry/rig-mcp/pull/6))
- Add deterministic MCP loopback harness prototype and update documentation

### Documentation

- Normalize quick start section
- Rename coordination references to rig-ecosystem
- Align ecosystem docs with rig-compose 0.3, rig-core 0.37, and rig-model-catalog 0.1 ([#11](https://github.com/ForeverAngry/rig-mcp/pull/11))
- Update ecosystem topology with rig-compose 0.3 and rig-model-catalog ([#10](https://github.com/ForeverAngry/rig-mcp/pull/10))
- Update ecosystem topology with rig-compose 0.3 and rig-model-catalog ([#9](https://github.com/ForeverAngry/rig-mcp/pull/9))
- Update ecosystem topology with rig-compose 0.3 and rig-model-catalog ([#8](https://github.com/ForeverAngry/rig-mcp/pull/8))
- Add mcp roadmap

### Added

- Add crate-local `ROADMAP.md` documenting maturity status, next work, and
  non-goals for the MCP bridge.
- Add `tests/harness.rs`, a deterministic MCP loopback harness prototype that
  records task input, endpoint, discovered tool names, normalized invocation,
  MCP-adapted dispatch result, final answer, and passed assertions.
- Add deterministic stdio fixtures covering successful child-process calls,
  unknown tools, missing arguments, wrong argument types, malformed child
  output, and child exit before handshake.
- Add stdio result-envelope coverage for oversized MCP outputs. The test proves
  `StdioTransport` preserves raw structured output and callers can use
  `rig_compose::bound_tool_result` for deterministic truncation metadata.
- Extend the deterministic harness to exercise the same tool task through local
  `ToolRegistry`, `LoopbackTransport`, and real child-process `StdioTransport`
  paths.

### Deprecated

- `McpTool::new` is deprecated in favour of [`McpTool::from_transport`],
  which is the only constructor exercised by callers. The associated
  function will be removed in the next major release.

## [0.1.3](https://github.com/ForeverAngry/rig-mcp/compare/v0.1.2...v0.1.3) - 2026-05-07

### Documentation

- Remove retired repo references

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
