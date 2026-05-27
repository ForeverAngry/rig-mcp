//! Result-cache layer for large MCP tool outputs.
//!
//! Large tool results (long search hit lists, document corpora, scraped
//! pages) don't belong in the model window. This module provides a
//! deterministic, transport-neutral way to:
//!
//! 1. **Cache** an oversized JSON array under an opaque handle.
//! 2. **Envelope** what the model actually sees: the handle, the total
//!    item count, the page size, and a deterministic first page.
//! 3. **Page** through the remaining items on demand via follow-up
//!    tools that read from the cache.
//! 4. **Release** the handle when the caller is done so memory bounds
//!    stay deterministic.
//!
//! `rig-mcp` does not yet auto-wire this into a transport; callers can
//! invoke [`cache_if_large`] from their tool body to opt-in per tool, or
//! register the cache as a [`rig_compose::tool::Tool`] for page/release
//! companion calls. Transport-level integration (e.g. a
//! `CachedResultsTransport<T>` wrapper) is a separate downstream concern
//! and intentionally not landed here.
//!
//! # Example
//!
//! ```no_run
//! use rig_mcp::result_cache::{
//!     CachedResultEnvelope, MemoryResultCache, ResultCache, cache_if_large,
//! };
//! use serde_json::{Value, json};
//! use std::sync::Arc;
//!
//! let cache: Arc<dyn ResultCache> = Arc::new(MemoryResultCache::new());
//! let big = json!([{"id": 1}, {"id": 2}, {"id": 3}]);
//! let envelope = cache_if_large(big, cache.as_ref(), 16, 1);
//! // The first page is a tiny preview; the rest are paged through the cache.
//! let env: CachedResultEnvelope = serde_json::from_value(envelope).unwrap();
//! assert_eq!(env.total_items, 3);
//! ```

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Public types ─────────────────────────────────────────────────────────────

/// Opaque, unique handle for a cached result. Stable for the lifetime of
/// the [`ResultCache`] entry; invalidated by [`ResultCache::release`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CachedResultHandle(pub String);

/// JSON envelope returned to the model in place of an oversized array.
///
/// The model sees `first_page` directly and can request later pages by
/// calling a host-supplied page tool with `handle` and an offset. The
/// envelope is deliberately small and self-describing so the model can
/// reason about how much data is hidden behind the handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CachedResultEnvelope {
    /// Opaque handle for the full cached result.
    pub handle: CachedResultHandle,
    /// Total number of items the cache holds for this handle.
    pub total_items: usize,
    /// Page size the cache will use when serving subsequent pages.
    pub page_size: usize,
    /// First `page_size.min(total_items)` items, inlined so the model
    /// doesn't always have to do a follow-up call.
    pub first_page: Vec<Value>,
}

/// Transport-neutral cache for paged tool results.
///
/// Implementations MUST be safe to share across tasks (`Send + Sync`).
/// Pagination uses `(offset, limit)` semantics: `page(handle, 0, n)`
/// returns up to the first `n` items. Implementations may serve
/// fewer items than requested if the offset is near the end.
pub trait ResultCache: Send + Sync {
    /// Store `items` and return the handle the caller should publish.
    fn store(&self, items: Vec<Value>) -> CachedResultHandle;
    /// Return up to `limit` items starting at `offset`. Returns `None`
    /// if the handle has been released or never existed; returns
    /// `Some(empty)` if the offset is past the end.
    fn page(&self, handle: &CachedResultHandle, offset: usize, limit: usize) -> Option<Vec<Value>>;
    /// Total item count for `handle`, or `None` if missing.
    fn len(&self, handle: &CachedResultHandle) -> Option<usize>;
    /// Release the handle. Returns `true` if it existed.
    fn release(&self, handle: &CachedResultHandle) -> bool;
}

/// Process-local, in-memory [`ResultCache`].
///
/// Backed by a `HashMap<String, Vec<Value>>` under a `std::sync::Mutex`.
/// Operations are short-lived and fully synchronous, so the mutex is
/// never held across an `.await` point.
#[derive(Debug, Default)]
pub struct MemoryResultCache {
    next_id: Mutex<u64>,
    inner: Mutex<HashMap<String, Vec<Value>>>,
}

impl MemoryResultCache {
    /// Construct an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of live handles. Useful for assertions and release audits.
    pub fn live_handles(&self) -> usize {
        self.inner.lock().map(|g| g.len()).unwrap_or(0)
    }

    fn fresh_handle(&self) -> CachedResultHandle {
        // Deterministic, monotonic IDs — easier to test than UUIDs and
        // perfectly adequate for an in-process cache. The wrapping
        // arithmetic is unreachable in practice but keeps clippy happy.
        let id = {
            let mut g = match self.next_id.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            let id = *g;
            *g = g.wrapping_add(1);
            id
        };
        CachedResultHandle(format!("mcp-cache-{id}"))
    }
}

impl ResultCache for MemoryResultCache {
    fn store(&self, items: Vec<Value>) -> CachedResultHandle {
        let handle = self.fresh_handle();
        if let Ok(mut g) = self.inner.lock() {
            g.insert(handle.0.clone(), items);
        }
        handle
    }

    fn page(&self, handle: &CachedResultHandle, offset: usize, limit: usize) -> Option<Vec<Value>> {
        let g = self.inner.lock().ok()?;
        let items = g.get(&handle.0)?;
        let end = offset.saturating_add(limit).min(items.len());
        let start = offset.min(items.len());
        Some(
            items
                .get(start..end)
                .map(<[Value]>::to_vec)
                .unwrap_or_default(),
        )
    }

    fn len(&self, handle: &CachedResultHandle) -> Option<usize> {
        let g = self.inner.lock().ok()?;
        g.get(&handle.0).map(Vec::len)
    }

    fn release(&self, handle: &CachedResultHandle) -> bool {
        match self.inner.lock() {
            Ok(mut g) => g.remove(&handle.0).is_some(),
            Err(_) => false,
        }
    }
}

// ── Sizing helper ────────────────────────────────────────────────────────────

/// If `value` is a JSON array whose serialized form exceeds
/// `threshold_bytes`, store it in `cache` and return a JSON
/// [`CachedResultEnvelope`]. Otherwise return `value` unchanged.
///
/// `page_size` is recorded in the envelope and used to slice
/// `first_page`. Non-array values are always returned unchanged because
/// pagination only makes sense over a sequence.
pub fn cache_if_large(
    value: Value,
    cache: &dyn ResultCache,
    threshold_bytes: usize,
    page_size: usize,
) -> Value {
    let arr = match value {
        Value::Array(items) => items,
        other => return other,
    };
    // Estimate size deterministically via the canonical JSON rendering.
    let serialized_size = match serde_json::to_string(&Value::Array(arr.clone())) {
        Ok(s) => s.len(),
        Err(_) => return Value::Array(arr),
    };
    if serialized_size <= threshold_bytes {
        return Value::Array(arr);
    }
    let total_items = arr.len();
    let first_page_len = page_size.min(total_items);
    let first_page: Vec<Value> = arr
        .get(..first_page_len)
        .map(<[Value]>::to_vec)
        .unwrap_or_default();
    let handle = cache.store(arr);
    let envelope = CachedResultEnvelope {
        handle,
        total_items,
        page_size,
        first_page,
    };
    serde_json::to_value(envelope).unwrap_or(Value::Null)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn store_then_page_returns_slice() {
        let cache = MemoryResultCache::new();
        let h = cache.store(vec![json!(1), json!(2), json!(3), json!(4)]);
        assert_eq!(cache.len(&h), Some(4));
        assert_eq!(cache.page(&h, 0, 2), Some(vec![json!(1), json!(2)]));
        assert_eq!(cache.page(&h, 2, 2), Some(vec![json!(3), json!(4)]));
    }

    #[test]
    fn page_past_end_returns_empty_not_none() {
        let cache = MemoryResultCache::new();
        let h = cache.store(vec![json!(1)]);
        assert_eq!(cache.page(&h, 5, 10), Some(vec![]));
    }

    #[test]
    fn page_unknown_handle_returns_none() {
        let cache = MemoryResultCache::new();
        let phantom = CachedResultHandle("nope".to_string());
        assert!(cache.page(&phantom, 0, 1).is_none());
        assert!(cache.len(&phantom).is_none());
        assert!(!cache.release(&phantom));
    }

    #[test]
    fn release_frees_handle_and_subsequent_calls_return_none() {
        let cache = MemoryResultCache::new();
        let h = cache.store(vec![json!("a"), json!("b")]);
        assert_eq!(cache.live_handles(), 1);
        assert!(cache.release(&h));
        assert_eq!(cache.live_handles(), 0);
        assert!(cache.page(&h, 0, 1).is_none());
        assert!(cache.len(&h).is_none());
        // Double-release is a no-op.
        assert!(!cache.release(&h));
    }

    #[test]
    fn handles_are_unique_per_store_call() {
        let cache = MemoryResultCache::new();
        let h1 = cache.store(vec![json!(1)]);
        let h2 = cache.store(vec![json!(2)]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn cache_if_large_passes_through_when_under_threshold() {
        let cache = MemoryResultCache::new();
        let v = json!([1, 2, 3]);
        let out = cache_if_large(v.clone(), &cache, 1024, 10);
        assert_eq!(out, v);
        assert_eq!(cache.live_handles(), 0);
    }

    #[test]
    fn cache_if_large_passes_through_for_non_arrays() {
        let cache = MemoryResultCache::new();
        let v = json!({"k": "v"});
        let out = cache_if_large(v.clone(), &cache, 0, 10);
        assert_eq!(out, v);
        assert_eq!(cache.live_handles(), 0);
    }

    #[test]
    fn cache_if_large_envelopes_oversized_array() {
        let cache = MemoryResultCache::new();
        let items: Vec<Value> = (0..50).map(|i| json!({"id": i})).collect();
        let out = cache_if_large(Value::Array(items), &cache, 16, 5);
        let env: CachedResultEnvelope = serde_json::from_value(out).unwrap();
        assert_eq!(env.total_items, 50);
        assert_eq!(env.page_size, 5);
        assert_eq!(env.first_page.len(), 5);
        assert_eq!(env.first_page[0], json!({"id": 0}));
        assert_eq!(env.first_page[4], json!({"id": 4}));
        // The handle is live and the full vec is paged through the cache.
        assert_eq!(cache.len(&env.handle), Some(50));
        let page2 = cache.page(&env.handle, 5, 5).unwrap();
        assert_eq!(page2.len(), 5);
        assert_eq!(page2[0], json!({"id": 5}));
    }
}
