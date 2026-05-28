//! Adapter-local registration snapshots for reconnect/replay flows.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::transport::{McpTool, McpTransport};
use rig_compose::registry::{KernelError, ToolRegistry};
use rig_compose::tool::{Tool, ToolSchema};

/// Reconnect policy attached to a discovered MCP registration snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RegistrationReplayPolicy {
    /// Re-register the last discovered tool schemas after reconnecting.
    ReRegisterDiscoveredTools,
    /// Do not replay automatically; the host must rediscover explicitly.
    ManualRediscovery,
}

/// Stable snapshot of the MCP tools discovered from one transport endpoint.
///
/// This type is intentionally adapter-local. It records the remote schemas a
/// host may replay after reconnecting, without pushing network or reconnect
/// state into `rig-compose::ToolRegistry`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationSnapshot {
    /// Transport endpoint that produced the snapshot.
    pub endpoint: String,
    /// Replay policy selected by the host or adapter.
    pub policy: RegistrationReplayPolicy,
    /// Deterministic tool schemas, sorted and deduplicated by name.
    pub tools: Vec<ToolSchema>,
}

impl RegistrationSnapshot {
    /// Build a deterministic snapshot from an endpoint and discovered schemas.
    pub fn new(endpoint: impl Into<String>, tools: Vec<ToolSchema>) -> Self {
        Self::with_policy(
            endpoint,
            tools,
            RegistrationReplayPolicy::ReRegisterDiscoveredTools,
        )
    }

    /// Build a deterministic snapshot with an explicit replay policy.
    pub fn with_policy(
        endpoint: impl Into<String>,
        tools: Vec<ToolSchema>,
        policy: RegistrationReplayPolicy,
    ) -> Self {
        let mut by_name = BTreeMap::new();
        for tool in tools {
            by_name.insert(tool.name.clone(), tool);
        }
        Self {
            endpoint: endpoint.into(),
            policy,
            tools: by_name.into_values().collect(),
        }
    }

    /// Snapshot visible descriptors from a local `rig-compose` registry.
    ///
    /// This is useful for loopback and host-owned reconnect flows that already
    /// have a registry view and want the same deterministic descriptor surface
    /// used by MCP discovery.
    pub fn from_registry(endpoint: impl Into<String>, registry: &ToolRegistry) -> Self {
        Self::new(endpoint, registry.descriptors())
    }

    /// Snapshot visible descriptors from a local registry with an explicit
    /// replay policy.
    pub fn from_registry_with_policy(
        endpoint: impl Into<String>,
        registry: &ToolRegistry,
        policy: RegistrationReplayPolicy,
    ) -> Self {
        Self::with_policy(endpoint, registry.descriptors(), policy)
    }

    /// Discover schemas from `transport` and snapshot them.
    pub async fn discover(transport: &dyn McpTransport) -> Result<Self, KernelError> {
        let tools = transport.list_tools().await?;
        Ok(Self::new(transport.endpoint(), tools))
    }

    /// Return tool names in deterministic replay order.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.iter().map(|tool| tool.name.clone()).collect()
    }

    /// Rebuild MCP tool adapters from this snapshot and a live transport.
    pub fn replay_tools(&self, transport: Arc<dyn McpTransport>) -> Vec<Arc<dyn Tool>> {
        self.tools
            .iter()
            .cloned()
            .map(|schema| {
                let tool: Arc<dyn Tool> = Arc::new(McpTool::from_schema(transport.clone(), schema));
                tool
            })
            .collect()
    }

    /// Register replayed MCP tools into `registry`.
    ///
    /// Replaying the same snapshot repeatedly is idempotent at registry level:
    /// `ToolRegistry::register` keys tools by name, so later replays replace
    /// the same remote tool name instead of growing duplicate entries.
    pub fn replay_into(&self, registry: &ToolRegistry, transport: Arc<dyn McpTransport>) -> usize {
        let tools = self.replay_tools(transport);
        let count = tools.len();
        for tool in tools {
            registry.register(tool);
        }
        count
    }
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
    use crate::transport::LoopbackTransport;
    use rig_compose::tool::LocalTool;
    use serde_json::json;

    fn schema(name: &str, description: &str) -> ToolSchema {
        ToolSchema {
            name: name.into(),
            description: description.into(),
            args_schema: json!({"type": "object"}),
            result_schema: json!({"type": "object"}),
        }
    }

    fn registry() -> ToolRegistry {
        let registry = ToolRegistry::new();
        registry.register(Arc::new(LocalTool::new(
            schema("math.add", "add"),
            |args| async move { Ok(args) },
        )));
        registry
    }

    #[test]
    fn snapshot_sorts_and_deduplicates_by_tool_name() {
        let snapshot = RegistrationSnapshot::new(
            "loopback://tools",
            vec![
                schema("z.last", "first version"),
                schema("a.first", "first"),
                schema("z.last", "second version"),
            ],
        );

        assert_eq!(snapshot.tool_names(), vec!["a.first", "z.last"]);
        assert_eq!(snapshot.tools[1].description, "second version");
    }

    #[test]
    fn snapshot_from_registry_uses_visible_descriptors() {
        let registry = registry();
        let scoped = registry.scoped(["math.add"]);
        let snapshot = RegistrationSnapshot::from_registry("loopback://registry", &scoped);

        assert_eq!(snapshot.endpoint, "loopback://registry");
        assert_eq!(snapshot.tool_names(), vec!["math.add"]);
    }

    #[test]
    fn snapshot_from_registry_honours_scope() {
        let registry = registry();
        let scoped = registry.scoped(["missing.tool"]);
        let snapshot = RegistrationSnapshot::from_registry("loopback://empty", &scoped);

        assert!(snapshot.tools.is_empty());
    }

    #[tokio::test]
    async fn discover_records_endpoint_and_sorted_tools() {
        let transport = LoopbackTransport::new("loopback://discover", registry());
        let snapshot = RegistrationSnapshot::discover(&transport).await.unwrap();

        assert_eq!(snapshot.endpoint, "loopback://discover");
        assert_eq!(
            snapshot.policy,
            RegistrationReplayPolicy::ReRegisterDiscoveredTools
        );
        assert_eq!(snapshot.tool_names(), vec!["math.add"]);
    }

    #[tokio::test]
    async fn replaying_snapshot_twice_keeps_registry_deduplicated() {
        let server = registry();
        let transport: Arc<dyn McpTransport> =
            Arc::new(LoopbackTransport::new("loopback://replay", server));
        let snapshot = RegistrationSnapshot::discover(transport.as_ref())
            .await
            .unwrap();
        let client = ToolRegistry::new();

        assert_eq!(snapshot.replay_into(&client, transport.clone()), 1);
        assert_eq!(snapshot.replay_into(&client, transport.clone()), 1);
        assert_eq!(client.len(), 1);

        let result = client
            .invoke("math.add", json!({"a": 1, "b": 2}))
            .await
            .unwrap();
        assert_eq!(result, json!({"a": 1, "b": 2}));
    }
}
