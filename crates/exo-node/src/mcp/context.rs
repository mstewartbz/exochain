// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! Runtime context for MCP tools — provides access to live node state.

use std::sync::{Arc, Mutex};

use crate::{reactor::SharedReactorState, store::SqliteDagStore};

/// Operator-supplied DAG DB gateway proxy configuration for MCP tools.
///
/// Present only when the `dagdb-gateway-proxy` feature is compiled. Empty
/// fields are rejected by the DAG DB tool dispatch before any HTTP request is
/// attempted.
#[cfg(feature = "dagdb-gateway-proxy")]
#[derive(Clone, Default)]
pub struct DagDbGatewayConfig {
    /// Gateway origin, for example `https://gateway.example.com`.
    pub base_url: Option<String>,
    /// Bearer token used by the SDK transport.
    pub bearer_token: Option<zeroize::Zeroizing<String>>,
    /// Tenant id authorized for this MCP proxy context.
    pub tenant_id: Option<String>,
    /// Namespace authorized for this MCP proxy context.
    pub namespace: Option<String>,
}

#[cfg(feature = "dagdb-gateway-proxy")]
impl DagDbGatewayConfig {
    #[must_use]
    pub fn new(
        base_url: impl Into<String>,
        bearer_token: impl Into<String>,
        tenant_id: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        Self {
            base_url: Some(base_url.into()),
            bearer_token: Some(zeroize::Zeroizing::new(bearer_token.into())),
            tenant_id: Some(tenant_id.into()),
            namespace: Some(namespace.into()),
        }
    }
}

/// Shared runtime context available to MCP tools.
///
/// Wraps the node's live state in a thread-safe, clonable handle that
/// tool implementations can query. All fields are optional so the MCP
/// server can also run in a pure-stdio mode without a full node.
#[derive(Clone, Default)]
pub struct NodeContext {
    /// Shared consensus reactor state (round, height, validators).
    pub reactor_state: Option<SharedReactorState>,
    /// Shared DAG store (event persistence, checkpoints).
    pub store: Option<Arc<Mutex<SqliteDagStore>>>,
    /// The node's own DID string.
    pub node_did: Option<String>,
    /// Opt-in DAG DB gateway proxy configuration.
    #[cfg(feature = "dagdb-gateway-proxy")]
    pub dagdb_gateway: Option<DagDbGatewayConfig>,
}

impl NodeContext {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Returns whether a live reactor state is attached.
    ///
    /// Reserved for future tools that want to short-circuit when running
    /// without a reactor.
    #[must_use]
    #[allow(dead_code)]
    pub fn has_reactor(&self) -> bool {
        self.reactor_state.is_some()
    }

    #[must_use]
    pub fn has_store(&self) -> bool {
        self.store.is_some()
    }
}
