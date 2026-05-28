//! Model-boundary result-cache integration for MCP transports.

use std::sync::Arc;

use async_trait::async_trait;
use rig_compose::registry::{KernelError, ToolRegistry};
use rig_compose::tool::{LocalTool, Tool, ToolSchema};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::result_cache::{CachedResultEnvelope, CachedResultHandle, ResultCache, cache_if_large};
use crate::transport::McpTransport;

/// Default registry name for the cached result page tool.
pub const CACHE_PAGE_TOOL: &str = "cache.page";

/// Default registry name for the cached result release tool.
pub const CACHE_RELEASE_TOOL: &str = "cache.release";

/// Configuration for model-boundary cached result envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedResultsConfig {
    /// Minimum serialized array size before a result is cached.
    pub threshold_bytes: usize,
    /// Number of items exposed in the first page and default follow-up pages.
    pub page_size: usize,
}

impl Default for CachedResultsConfig {
    fn default() -> Self {
        Self {
            threshold_bytes: 64 * 1024,
            page_size: 64,
        }
    }
}

impl CachedResultsConfig {
    /// Build a config with an explicit size threshold and otherwise default
    /// page settings.
    #[must_use]
    pub fn new(threshold_bytes: usize) -> Self {
        Self {
            threshold_bytes,
            ..Self::default()
        }
    }

    /// Set the page size used for first-page and follow-up slices.
    #[must_use]
    pub fn with_page_size(mut self, page_size: usize) -> Self {
        self.page_size = page_size;
        self
    }
}

/// MCP transport wrapper that caches oversized array results at the
/// model-facing boundary.
///
/// The wrapped transport still owns protocol mechanics and raw tool execution.
/// This adapter only rewrites oversized array results into
/// [`CachedResultEnvelope`] JSON values after the remote call completes.
pub struct CachedResultsTransport {
    inner: Arc<dyn McpTransport>,
    cache: Arc<dyn ResultCache>,
    config: CachedResultsConfig,
}

impl CachedResultsTransport {
    /// Wrap `inner` with the default cached-result policy.
    pub fn new(inner: Arc<dyn McpTransport>, cache: Arc<dyn ResultCache>) -> Self {
        Self::with_config(inner, cache, CachedResultsConfig::default())
    }

    /// Wrap `inner` with an explicit cached-result policy.
    pub fn with_config(
        inner: Arc<dyn McpTransport>,
        cache: Arc<dyn ResultCache>,
        config: CachedResultsConfig,
    ) -> Self {
        Self {
            inner,
            cache,
            config,
        }
    }

    /// Shared cache backing this transport wrapper.
    #[must_use]
    pub fn cache(&self) -> Arc<dyn ResultCache> {
        self.cache.clone()
    }

    /// Cached-result policy used by this wrapper.
    #[must_use]
    pub fn config(&self) -> CachedResultsConfig {
        self.config
    }
}

#[async_trait]
impl McpTransport for CachedResultsTransport {
    fn endpoint(&self) -> &str {
        self.inner.endpoint()
    }

    async fn list_tools(&self) -> Result<Vec<ToolSchema>, KernelError> {
        self.inner.list_tools().await
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<Value, KernelError> {
        let value = self.inner.call_tool(name, args).await?;
        Ok(cache_if_large(
            value,
            self.cache.as_ref(),
            self.config.threshold_bytes,
            self.config.page_size,
        ))
    }
}

/// Register the default cached-result page and release tools into `registry`.
pub fn register_cache_tools(registry: &ToolRegistry, cache: Arc<dyn ResultCache>) {
    registry.register(cache_page_tool(cache.clone()));
    registry.register(cache_release_tool(cache));
}

/// Build the default cached-result page and release tools.
#[must_use]
pub fn cache_tools(cache: Arc<dyn ResultCache>) -> Vec<Arc<dyn Tool>> {
    vec![cache_page_tool(cache.clone()), cache_release_tool(cache)]
}

/// Build a tool that returns a page from a cached result handle.
#[must_use]
pub fn cache_page_tool(cache: Arc<dyn ResultCache>) -> Arc<dyn Tool> {
    Arc::new(LocalTool::new(
        ToolSchema {
            name: CACHE_PAGE_TOOL.into(),
            description: "Return a page from a cached MCP result handle".into(),
            args_schema: json!({
                "type": "object",
                "properties": {
                    "handle": {"type": "string"},
                    "page_token": {"type": "string"},
                    "offset": {"type": "integer", "minimum": 0},
                    "limit": {"type": "integer", "minimum": 0}
                },
                "additionalProperties": false
            }),
            result_schema: json!({
                "type": "object",
                "properties": {
                    "handle": {"type": "string"},
                    "offset": {"type": "integer"},
                    "limit": {"type": "integer"},
                    "total_items": {"type": "integer"},
                    "items": {"type": "array"},
                    "next_page_token": {"type": "string"}
                }
            }),
        },
        move |args| {
            let cache = cache.clone();
            async move { page_cached_result(cache.as_ref(), args) }
        },
    ))
}

/// Build a tool that releases a cached result handle.
#[must_use]
pub fn cache_release_tool(cache: Arc<dyn ResultCache>) -> Arc<dyn Tool> {
    Arc::new(LocalTool::new(
        ToolSchema {
            name: CACHE_RELEASE_TOOL.into(),
            description: "Release a cached MCP result handle".into(),
            args_schema: json!({
                "type": "object",
                "required": ["handle"],
                "properties": {
                    "handle": {"type": "string"}
                },
                "additionalProperties": false
            }),
            result_schema: json!({
                "type": "object",
                "properties": {
                    "handle": {"type": "string"},
                    "released": {"type": "boolean"}
                }
            }),
        },
        move |args| {
            let cache = cache.clone();
            async move { release_cached_result(cache.as_ref(), args) }
        },
    ))
}

fn page_cached_result(cache: &dyn ResultCache, args: Value) -> Result<Value, KernelError> {
    let page_request = PageRequest::from_args(args)?;
    let total_items = cache
        .len(&page_request.handle)
        .ok_or_else(|| KernelError::InvalidArgument("unknown cached result handle".into()))?;
    let items = cache
        .page(
            &page_request.handle,
            page_request.offset,
            page_request.limit,
        )
        .ok_or_else(|| KernelError::InvalidArgument("unknown cached result handle".into()))?;
    let next_offset = page_request.offset.saturating_add(items.len());
    let next_page_token = (next_offset < total_items)
        .then(|| CachedResultEnvelope::page_token(&page_request.handle, next_offset));

    Ok(json!({
        "handle": page_request.handle.0,
        "offset": page_request.offset,
        "limit": page_request.limit,
        "total_items": total_items,
        "items": items,
        "next_page_token": next_page_token,
    }))
}

fn release_cached_result(cache: &dyn ResultCache, args: Value) -> Result<Value, KernelError> {
    let handle = required_handle(&args)?;
    let released = cache.release(&handle);
    Ok(json!({
        "handle": handle.0,
        "released": released,
    }))
}

struct PageRequest {
    handle: CachedResultHandle,
    offset: usize,
    limit: usize,
}

impl PageRequest {
    fn from_args(args: Value) -> Result<Self, KernelError> {
        let token_parts = optional_page_token(&args)?;
        let handle = match token_parts.as_ref() {
            Some((handle, _)) => handle.clone(),
            None => required_handle(&args)?,
        };
        let offset = match token_parts {
            Some((_, offset)) => offset,
            None => optional_usize(&args, "offset")?.unwrap_or(0),
        };
        let limit = optional_usize(&args, "limit")?.unwrap_or(64);
        Ok(Self {
            handle,
            offset,
            limit,
        })
    }
}

fn required_handle(args: &Value) -> Result<CachedResultHandle, KernelError> {
    let text = required_string(args, "handle")?;
    Ok(CachedResultHandle(text))
}

fn required_string(args: &Value, field: &str) -> Result<String, KernelError> {
    args.get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| KernelError::InvalidArgument(format!("missing `{field}` string")))
}

fn optional_usize(args: &Value, field: &str) -> Result<Option<usize>, KernelError> {
    let Some(value) = args.get(field) else {
        return Ok(None);
    };
    let number = value
        .as_u64()
        .ok_or_else(|| KernelError::InvalidArgument(format!("`{field}` must be an integer")))?;
    usize::try_from(number)
        .map(Some)
        .map_err(|_| KernelError::InvalidArgument(format!("`{field}` is too large")))
}

fn optional_page_token(args: &Value) -> Result<Option<(CachedResultHandle, usize)>, KernelError> {
    let Some(token) = args.get("page_token").and_then(Value::as_str) else {
        return Ok(None);
    };
    let (handle, offset) = token
        .rsplit_once(":offset:")
        .ok_or_else(|| KernelError::InvalidArgument("invalid `page_token`".into()))?;
    let offset = offset
        .parse::<usize>()
        .map_err(|_| KernelError::InvalidArgument("invalid `page_token` offset".into()))?;
    Ok(Some((CachedResultHandle(handle.to_string()), offset)))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::result_cache::{CachedResultEnvelope, MemoryResultCache};
    use crate::transport::LoopbackTransport;
    use rig_compose::tool::LocalTool;
    use serde_json::json;

    fn schema(name: &str) -> ToolSchema {
        ToolSchema {
            name: name.into(),
            description: "test tool".into(),
            args_schema: json!({"type": "object"}),
            result_schema: json!({"type": "array"}),
        }
    }

    fn array_registry() -> ToolRegistry {
        let registry = ToolRegistry::new();
        registry.register(Arc::new(LocalTool::new(
            schema("search.many"),
            |_args| async {
                let items: Vec<Value> = (0..20).map(|id| json!({"id": id})).collect();
                Ok(Value::Array(items))
            },
        )));
        registry.register(Arc::new(LocalTool::new(
            schema("search.small"),
            |_args| async { Ok(json!([{"id": 1}])) },
        )));
        registry.register(Arc::new(LocalTool::new(
            schema("search.object"),
            |_args| async { Ok(json!({"items": [1, 2, 3]})) },
        )));
        registry
    }

    #[tokio::test]
    async fn cached_transport_envelopes_oversized_arrays() {
        let cache = Arc::new(MemoryResultCache::new());
        let inner: Arc<dyn McpTransport> =
            Arc::new(LoopbackTransport::new("loopback://cache", array_registry()));
        let transport = CachedResultsTransport::with_config(
            inner,
            cache.clone(),
            CachedResultsConfig::new(8).with_page_size(5),
        );

        let output = transport.call_tool("search.many", json!({})).await.unwrap();
        let envelope: CachedResultEnvelope = serde_json::from_value(output).unwrap();

        assert_eq!(envelope.total_items, 20);
        assert_eq!(envelope.first_page.len(), 5);
        assert_eq!(envelope.omitted_items, 15);
        assert_eq!(envelope.page_token.as_deref(), Some("mcp-cache-0:offset:5"));
        assert_eq!(cache.live_handles(), 1);
    }

    #[tokio::test]
    async fn cached_transport_preserves_small_and_non_array_results() {
        let cache = Arc::new(MemoryResultCache::new());
        let inner: Arc<dyn McpTransport> =
            Arc::new(LoopbackTransport::new("loopback://cache", array_registry()));
        let transport = CachedResultsTransport::with_config(
            inner,
            cache.clone(),
            CachedResultsConfig::new(1024).with_page_size(5),
        );

        let small = transport
            .call_tool("search.small", json!({}))
            .await
            .unwrap();
        let object = transport
            .call_tool("search.object", json!({}))
            .await
            .unwrap();

        assert_eq!(small, json!([{"id": 1}]));
        assert_eq!(object, json!({"items": [1, 2, 3]}));
        assert_eq!(cache.live_handles(), 0);
    }

    #[tokio::test]
    async fn cache_tools_page_and_release_handles() {
        let cache = Arc::new(MemoryResultCache::new());
        let inner: Arc<dyn McpTransport> =
            Arc::new(LoopbackTransport::new("loopback://cache", array_registry()));
        let transport = CachedResultsTransport::with_config(
            inner,
            cache.clone(),
            CachedResultsConfig::new(8).with_page_size(5),
        );
        let registry = ToolRegistry::new();
        register_cache_tools(&registry, cache.clone());

        let output = transport.call_tool("search.many", json!({})).await.unwrap();
        let envelope: CachedResultEnvelope = serde_json::from_value(output).unwrap();
        let page = registry
            .invoke(
                CACHE_PAGE_TOOL,
                json!({"page_token": envelope.page_token, "limit": 4}),
            )
            .await
            .unwrap();

        assert_eq!(page["offset"], json!(5));
        assert_eq!(page["limit"], json!(4));
        assert_eq!(page["items"].as_array().unwrap().len(), 4);
        assert_eq!(page["items"][0], json!({"id": 5}));
        assert_eq!(page["next_page_token"], json!("mcp-cache-0:offset:9"));

        let released = registry
            .invoke(CACHE_RELEASE_TOOL, json!({"handle": envelope.handle.0}))
            .await
            .unwrap();
        assert_eq!(released["released"], json!(true));
        assert_eq!(cache.live_handles(), 0);
    }

    #[tokio::test]
    async fn page_tool_rejects_unknown_handles() {
        let cache = Arc::new(MemoryResultCache::new());
        let page_tool = cache_page_tool(cache);
        let error = page_tool
            .invoke(json!({"handle": "missing", "offset": 0, "limit": 1}))
            .await
            .unwrap_err();

        assert!(matches!(error, KernelError::InvalidArgument(_)));
    }
}
