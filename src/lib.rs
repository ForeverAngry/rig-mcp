//! # rig-mcp
//!
//! Model Context Protocol bridge for [`rig-compose`](https://crates.io/crates/rig-compose)
//! tool registries, backed by the official [`rmcp`] SDK.
//!
//! Skills cannot tell an [`McpTool`] apart from a `rig_compose::LocalTool`
//! — both implement the same [`Tool`](rig_compose::tool::Tool) trait.
//! The same registry can be exposed as an MCP server via [`serve_stdio`]
//! and consumed by another process via [`StdioTransport::spawn`], with
//! all spec-level concerns (JSON-RPC framing, capability handshakes,
//! protocol-version negotiation) delegated to `rmcp`.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::panic_in_result_fn,
    )
)]

pub mod cache_tools;
pub mod replay;
pub mod result_cache;
#[cfg(feature = "stdio")]
pub mod stdio;
pub mod transport;

pub use cache_tools::{
    CACHE_PAGE_TOOL, CACHE_RELEASE_TOOL, CachedResultsConfig, CachedResultsTransport,
    cache_page_tool, cache_release_tool, cache_tools, register_cache_tools,
};
pub use replay::{RegistrationReplayPolicy, RegistrationSnapshot};
pub use result_cache::{
    CachedResultEnvelope, CachedResultHandle, MemoryResultCache, ResultCache, cache_if_large,
};
#[cfg(feature = "stdio")]
pub use stdio::{StdioTransport, serve_stdio};
pub use transport::{LoopbackTransport, McpTool, McpTransport};
