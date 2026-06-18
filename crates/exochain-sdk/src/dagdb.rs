//! SDK re-exports for ExoChain DAG DB DTOs.
//!
//! This module intentionally re-exports the `exo-dag-db-api` DTOs rather than
//! defining a parallel SDK shape. JSON fixture tests guard against drift.

pub use exo_dag_db_api::{
    CanonicalizationDecision, CanonicalizationDecisionKind, CatalogEntryResponse, ConsentPurpose,
    ContextPacketMemoryRef, CouncilDecisionStatus, CouncilReviewStatus, CredentialStatus,
    DagDbCatalogLookupRequest, DagDbCatalogLookupResponse, DagDbContextPacketRequest,
    DagDbContextPacketResponse, DagDbCouncilDecisionRequest, DagDbCouncilDecisionResponse,
    DagDbErrorEnvelope, DagDbExportRequest, DagDbExportResponse, DagDbImportRequest,
    DagDbImportResponse, DagDbIntakeRequest, DagDbIntakeResponse, DagDbReceiptLookupRequest,
    DagDbReceiptLookupResponse, DagDbRouteLookupRequest, DagDbRouteLookupResponse,
    DagDbRouteRequest, DagDbRouteResponse, DagDbTrustCheckRequest, DagDbTrustCheckResponse,
    DagDbValidateRequest, DagDbValidateResponse, DagDbWritebackRequest, DagDbWritebackResponse,
    DagFinalityStatus, DecisionSource, GraphEdgeRef, GraphView, GraphViewType, MemoryCandidate,
    MemoryCandidateKind, MemoryCandidateUse, MemoryEdgeKind, MemoryEdgeType, MemoryGraphStyle,
    MemoryNodeKind, MemoryNodeType, PlacementResult, RedactionCode, RiskClass,
    RouteInvalidationReceipt, RouteInvalidationStatus, RouteInvalidationTrigger, RouteStatus,
    SafeMetadata, SafeMetadataDecision, SimilarityResult, SimilarityType, SourceType, SubjectKind,
    ValidationDecision, ValidationStatus,
};

/// HTTP verb for an SDK-prepared DAG DB request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DagDbHttpMethod {
    /// HTTP GET request.
    Get,
    /// HTTP POST request.
    Post,
}

/// Request metadata prepared by the SDK without redefining DTO shapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagDbRequestSpec<T> {
    /// HTTP method to use for the request.
    pub method: DagDbHttpMethod,
    /// Path and query string relative to the gateway origin.
    pub path: String,
    /// JSON body for mutation routes; lookup routes carry `None`.
    pub body: Option<T>,
}

/// Lightweight helpers for constructing every DAG DB REST call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagDbClient {
    prefix: String,
}

impl Default for DagDbClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DagDbClient {
    /// Create a client helper bound to the canonical DAG DB REST prefix.
    pub fn new() -> Self {
        Self {
            prefix: "/api/v1/dag-db".to_owned(),
        }
    }

    /// Build a `POST /api/v1/dag-db/intake` request spec.
    pub fn intake(&self, request: DagDbIntakeRequest) -> DagDbRequestSpec<DagDbIntakeRequest> {
        self.post("/intake", request)
    }

    /// Build a `POST /api/v1/dag-db/route` request spec.
    pub fn route(&self, request: DagDbRouteRequest) -> DagDbRequestSpec<DagDbRouteRequest> {
        self.post("/route", request)
    }

    /// Build a `POST /api/v1/dag-db/context-packet` request spec.
    pub fn context_packet(
        &self,
        request: DagDbContextPacketRequest,
    ) -> DagDbRequestSpec<DagDbContextPacketRequest> {
        self.post("/context-packet", request)
    }

    /// Build a `POST /api/v1/dag-db/validate` request spec.
    pub fn validate(
        &self,
        request: DagDbValidateRequest,
    ) -> DagDbRequestSpec<DagDbValidateRequest> {
        self.post("/validate", request)
    }

    /// Build a `POST /api/v1/dag-db/writeback` request spec.
    pub fn writeback(
        &self,
        request: DagDbWritebackRequest,
    ) -> DagDbRequestSpec<DagDbWritebackRequest> {
        self.post("/writeback", request)
    }

    /// Build a `POST /api/v1/dag-db/import` request spec.
    pub fn dagdb_import(
        &self,
        request: DagDbImportRequest,
    ) -> DagDbRequestSpec<DagDbImportRequest> {
        self.post("/import", request)
    }

    /// Build a `POST /api/v1/dag-db/export` request spec.
    pub fn dagdb_export(
        &self,
        request: DagDbExportRequest,
    ) -> DagDbRequestSpec<DagDbExportRequest> {
        self.post("/export", request)
    }

    /// Build a `POST /api/v1/dag-db/trust-check` request spec.
    pub fn trust_check(
        &self,
        request: DagDbTrustCheckRequest,
    ) -> DagDbRequestSpec<DagDbTrustCheckRequest> {
        self.post("/trust-check", request)
    }

    /// Build a `POST /api/v1/dag-db/council/decision` request spec.
    pub fn council_decision(
        &self,
        request: DagDbCouncilDecisionRequest,
    ) -> DagDbRequestSpec<DagDbCouncilDecisionRequest> {
        self.post("/council/decision", request)
    }

    /// Build a `GET /api/v1/dag-db/receipts/{receipt_hash}` request spec.
    pub fn receipt_lookup(
        &self,
        request: DagDbReceiptLookupRequest,
    ) -> DagDbRequestSpec<DagDbReceiptLookupRequest> {
        DagDbRequestSpec {
            method: DagDbHttpMethod::Get,
            path: format!(
                "{}/receipts/{}?tenant_id={}&namespace={}{}",
                self.prefix,
                request.receipt_hash,
                request.tenant_id,
                request.namespace,
                optional_bool_query("include_body", request.include_body)
            ),
            body: None,
        }
    }

    /// Build a `GET /api/v1/dag-db/catalog/{catalog_id}` request spec.
    pub fn catalog_lookup(
        &self,
        request: DagDbCatalogLookupRequest,
    ) -> DagDbRequestSpec<DagDbCatalogLookupRequest> {
        DagDbRequestSpec {
            method: DagDbHttpMethod::Get,
            path: format!(
                "{}/catalog/{}?tenant_id={}&namespace={}{}{}",
                self.prefix,
                request.catalog_id,
                request.tenant_id,
                request.namespace,
                optional_bool_query("include_children", request.include_children),
                optional_bool_query("include_routes", request.include_routes)
            ),
            body: None,
        }
    }

    /// Build a `GET /api/v1/dag-db/routes/{route_id}` request spec.
    pub fn route_lookup(
        &self,
        request: DagDbRouteLookupRequest,
    ) -> DagDbRequestSpec<DagDbRouteLookupRequest> {
        DagDbRequestSpec {
            method: DagDbHttpMethod::Get,
            path: format!(
                "{}/routes/{}?tenant_id={}&namespace={}{}{}",
                self.prefix,
                request.route_id,
                request.tenant_id,
                request.namespace,
                optional_bool_query("include_memory_refs", request.include_memory_refs),
                optional_bool_query("include_validation", request.include_validation)
            ),
            body: None,
        }
    }

    fn post<T>(&self, suffix: &str, request: T) -> DagDbRequestSpec<T> {
        DagDbRequestSpec {
            method: DagDbHttpMethod::Post,
            path: format!("{}{}", self.prefix, suffix),
            body: Some(request),
        }
    }
}

fn optional_bool_query(name: &str, value: Option<bool>) -> String {
    value.map_or_else(String::new, |enabled| format!("&{name}={enabled}"))
}

#[cfg(feature = "http-client")]
pub use transport::{
    BearerToken, DagDbAuthConfig, DagDbClientError, DagDbHttpClient, DagDbServerError,
};

/// Real async HTTP transport for the DAG DB REST surface.
///
/// Compiled only under the non-default `http-client` feature so the spec-only
/// SDK (the [`DagDbClient`] request-spec builder above) stays free of the async
/// HTTP stack. With the feature enabled, [`DagDbHttpClient`] turns the same
/// specs into real `reqwest` calls: it injects the four gateway auth headers
/// (`Authorization`, `x-exo-tenant-id`, `x-exo-namespace`,
/// `x-exo-authority-scope`) with the per-route `{action}:{tenant}:{namespace}`
/// scope, sends the request, and maps the response into typed
/// [`DagDbClientError`] variants (transport/timeout, governed server error
/// carrying the [`DagDbErrorEnvelope`], schema-version mismatch, or decode
/// failure) without ever swallowing an error.
#[cfg(feature = "http-client")]
mod transport {
    use std::fmt;

    use reqwest::{
        Client, StatusCode,
        header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue},
    };
    use serde::{Serialize, de::DeserializeOwned};
    use zeroize::Zeroize;

    use super::{
        DagDbCatalogLookupRequest, DagDbCatalogLookupResponse, DagDbClient,
        DagDbContextPacketRequest, DagDbContextPacketResponse, DagDbCouncilDecisionRequest,
        DagDbCouncilDecisionResponse, DagDbErrorEnvelope, DagDbExportRequest, DagDbExportResponse,
        DagDbHttpMethod, DagDbImportRequest, DagDbImportResponse, DagDbIntakeRequest,
        DagDbIntakeResponse, DagDbReceiptLookupRequest, DagDbReceiptLookupResponse,
        DagDbRequestSpec, DagDbRouteLookupRequest, DagDbRouteLookupResponse, DagDbRouteRequest,
        DagDbRouteResponse, DagDbTrustCheckRequest, DagDbTrustCheckResponse, DagDbValidateRequest,
        DagDbValidateResponse, DagDbWritebackRequest, DagDbWritebackResponse,
    };

    /// Gateway header carrying the requesting tenant id.
    const TENANT_HEADER: &str = "x-exo-tenant-id";
    /// Gateway header carrying the requesting namespace.
    const NAMESPACE_HEADER: &str = "x-exo-namespace";
    /// Gateway header carrying the `{action}:{tenant}:{namespace}` authority scope.
    const AUTHORITY_SCOPE_HEADER: &str = "x-exo-authority-scope";

    /// Bearer token wrapper that never exposes its secret via [`fmt::Debug`].
    ///
    /// The inner string is zeroized on drop and redacted in both `Debug` and
    /// `Display`, so a token cannot leak through a `{:?}` of an auth config,
    /// client, or error. Read the raw value only at the point of header
    /// construction, via the bearer token's reveal accessor.
    #[derive(Clone, Zeroize)]
    #[zeroize(drop)]
    pub struct BearerToken(String);

    impl BearerToken {
        /// Wrap a raw bearer token string.
        pub fn new(token: impl Into<String>) -> Self {
            Self(token.into())
        }

        /// Borrow the raw token. Use only to build the `Authorization` header;
        /// never log the result.
        fn reveal(&self) -> &str {
            &self.0
        }
    }

    impl From<String> for BearerToken {
        fn from(token: String) -> Self {
            Self(token)
        }
    }

    impl From<&str> for BearerToken {
        fn from(token: &str) -> Self {
            Self(token.to_owned())
        }
    }

    impl fmt::Debug for BearerToken {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("BearerToken(<redacted>)")
        }
    }

    impl fmt::Display for BearerToken {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("<redacted>")
        }
    }

    /// Tenant- and namespace-scoped auth material the gateway requires on every
    /// DAG DB request. The bearer token is held redacted (see [`BearerToken`]);
    /// `tenant_id`/`namespace` populate the matching headers and the
    /// `{action}:{tenant}:{namespace}` authority scope.
    #[derive(Clone, Debug)]
    pub struct DagDbAuthConfig {
        /// Bearer token presented as `Authorization: Bearer <token>`.
        pub token: BearerToken, // pragma-allowlist-secret (field type, not a credential)
        /// Tenant id for the `x-exo-tenant-id` header and the scope grammar.
        pub tenant_id: String,
        /// Namespace for the `x-exo-namespace` header and the scope grammar.
        pub namespace: String,
    }

    impl DagDbAuthConfig {
        /// Build an auth config from a token, tenant id, and namespace.
        pub fn new(
            token: impl Into<BearerToken>,
            tenant_id: impl Into<String>,
            namespace: impl Into<String>,
        ) -> Self {
            Self {
                token: token.into(),
                tenant_id: tenant_id.into(),
                namespace: namespace.into(),
            }
        }

        /// Render the `{action}:{tenant}:{namespace}` authority scope for an action.
        fn authority_scope(&self, action: &str) -> String {
            format!("{action}:{}:{}", self.tenant_id, self.namespace)
        }
    }

    /// Governed error returned by the gateway for a non-2xx DAG DB response.
    ///
    /// Carries the parsed [`DagDbErrorEnvelope`] fields so a caller can branch
    /// on `error_code`, surface the operator message, follow the
    /// `receipt_hash`, or route a `requires_council_review` case to governance —
    /// without re-parsing the body.
    #[derive(Clone, Debug, thiserror::Error)]
    #[error("DAG DB gateway returned {status} ({error_code}): {message}")]
    pub struct DagDbServerError {
        /// HTTP status code the gateway returned.
        pub status: u16,
        /// Stable machine error code (e.g. `tenant_scope_mismatch`).
        pub error_code: String,
        /// Human-readable operator message.
        pub message: String,
        /// Receipt hash for the denied/failed action, when the gateway emits one.
        pub receipt_hash: Option<String>,
        /// Validation report id, when the failure is a validation rejection.
        pub validation_report_id: Option<String>,
        /// Whether the action is parked pending council review.
        pub requires_council_review: bool,
    }

    /// Every failure mode of a [`DagDbHttpClient`] call. No error is swallowed:
    /// a call returns `Ok(Resp)` only on a 2xx whose body deserializes and whose
    /// `schema_version` matches the expected per-DTO constant.
    #[derive(Debug, thiserror::Error)]
    pub enum DagDbClientError {
        /// The request never produced an HTTP response (DNS/connect/TLS) or the
        /// connection failed mid-flight.
        #[error("DAG DB transport error: {0}")]
        Transport(#[source] reqwest::Error),

        /// The request exceeded the configured timeout.
        #[error("DAG DB request timed out: {0}")]
        Timeout(#[source] reqwest::Error),

        /// The gateway returned a non-2xx status with a parsed error envelope.
        #[error(transparent)]
        Server(DagDbServerError),

        /// The gateway returned a non-2xx status whose body was not a valid
        /// [`DagDbErrorEnvelope`]. Carries the status and the raw body so the
        /// failure is still actionable.
        #[error("DAG DB gateway returned {status} with unparseable error body: {body}")]
        UnexpectedStatus {
            /// HTTP status code returned.
            status: u16,
            /// Raw response body (truncated by the gateway, surfaced verbatim).
            body: String,
        },

        /// A 2xx response body failed to deserialize into the expected DTO.
        #[error("DAG DB response decode error: {0}")]
        Decode(#[source] reqwest::Error),

        /// A 2xx response deserialized, but its `schema_version` did not match
        /// the constant this SDK was built against — surfaced, never ignored.
        #[error("DAG DB schema version mismatch: expected `{expected}`, got `{actual}`")]
        SchemaVersionMismatch {
            /// Schema version this SDK expects for the DTO.
            expected: &'static str,
            /// Schema version the gateway actually emitted.
            actual: String,
        },

        /// Auth material (token/tenant/namespace/scope) contained bytes that are
        /// not a legal HTTP header value, so the request was never sent. The
        /// token itself is never included in this error.
        #[error("DAG DB auth header `{header}` is not a valid HTTP header value")]
        InvalidAuthHeader {
            /// Name of the header that could not be constructed.
            header: &'static str,
        },
    }

    impl DagDbClientError {
        /// Classify a `reqwest::Error` into the timeout vs. transport variant.
        fn from_reqwest(err: reqwest::Error) -> Self {
            if err.is_timeout() {
                Self::Timeout(err)
            } else {
                Self::Transport(err)
            }
        }
    }

    /// Async HTTP client for the DAG DB REST surface.
    ///
    /// Holds the base URL, auth config, and a `reqwest::Client`. Each typed
    /// method reuses the [`DagDbClient`] spec builder, attaches the four auth
    /// headers (with the route's `{action}` baked into the authority scope),
    /// sends the request, and maps the response. The bearer token is never
    /// logged: `DagDbHttpClient` deliberately does not derive `Debug`.
    pub struct DagDbHttpClient {
        base_url: String,
        auth: DagDbAuthConfig,
        specs: DagDbClient,
        http: Client,
    }

    impl DagDbHttpClient {
        /// Build a client against `base_url` (gateway origin, e.g.
        /// `https://gateway.example.com`) using the supplied auth config and a
        /// default-configured `reqwest::Client`.
        ///
        /// # Errors
        /// Returns [`DagDbClientError::Transport`] if the underlying
        /// `reqwest::Client` cannot be constructed.
        pub fn new(
            base_url: impl Into<String>,
            auth: DagDbAuthConfig,
        ) -> Result<Self, DagDbClientError> {
            let http = Client::builder()
                .build()
                .map_err(DagDbClientError::from_reqwest)?;
            Ok(Self::with_client(base_url, auth, http))
        }

        /// Build a client with a caller-supplied `reqwest::Client` (for shared
        /// connection pools, custom timeouts, proxies, or TLS roots).
        pub fn with_client(
            base_url: impl Into<String>,
            auth: DagDbAuthConfig,
            http: Client,
        ) -> Self {
            Self {
                base_url: trim_trailing_slash(base_url.into()),
                auth,
                specs: DagDbClient::new(),
                http,
            }
        }

        /// `POST /api/v1/dag-db/intake`.
        pub async fn intake(
            &self,
            request: DagDbIntakeRequest,
        ) -> Result<DagDbIntakeResponse, DagDbClientError> {
            self.send(
                self.specs.intake(request),
                "dagdb:intake",
                exo_dag_db_api::DAGDB_INTAKE_RESPONSE_SCHEMA_VERSION,
                |r: &DagDbIntakeResponse| r.schema_version.as_str(),
            )
            .await
        }

        /// `POST /api/v1/dag-db/route`.
        pub async fn route(
            &self,
            request: DagDbRouteRequest,
        ) -> Result<DagDbRouteResponse, DagDbClientError> {
            self.send(
                self.specs.route(request),
                "dagdb:route",
                exo_dag_db_api::DAGDB_ROUTE_RESPONSE_SCHEMA_VERSION,
                |r: &DagDbRouteResponse| r.schema_version.as_str(),
            )
            .await
        }

        /// `POST /api/v1/dag-db/context-packet`.
        pub async fn context_packet(
            &self,
            request: DagDbContextPacketRequest,
        ) -> Result<DagDbContextPacketResponse, DagDbClientError> {
            self.send(
                self.specs.context_packet(request),
                "dagdb:context_packet",
                exo_dag_db_api::DAGDB_CONTEXT_PACKET_RESPONSE_SCHEMA_VERSION,
                |r: &DagDbContextPacketResponse| r.schema_version.as_str(),
            )
            .await
        }

        /// `POST /api/v1/dag-db/validate`.
        pub async fn validate(
            &self,
            request: DagDbValidateRequest,
        ) -> Result<DagDbValidateResponse, DagDbClientError> {
            self.send(
                self.specs.validate(request),
                "dagdb:validate",
                exo_dag_db_api::DAGDB_VALIDATE_RESPONSE_SCHEMA_VERSION,
                |r: &DagDbValidateResponse| r.schema_version.as_str(),
            )
            .await
        }

        /// `POST /api/v1/dag-db/writeback`.
        pub async fn writeback(
            &self,
            request: DagDbWritebackRequest,
        ) -> Result<DagDbWritebackResponse, DagDbClientError> {
            self.send(
                self.specs.writeback(request),
                "dagdb:writeback",
                exo_dag_db_api::DAGDB_WRITEBACK_RESPONSE_SCHEMA_VERSION,
                |r: &DagDbWritebackResponse| r.schema_version.as_str(),
            )
            .await
        }

        /// `POST /api/v1/dag-db/import`.
        pub async fn dagdb_import(
            &self,
            request: DagDbImportRequest,
        ) -> Result<DagDbImportResponse, DagDbClientError> {
            self.send(
                self.specs.dagdb_import(request),
                "dagdb:import",
                exo_dag_db_api::DAGDB_IMPORT_RESPONSE_SCHEMA_VERSION,
                |r: &DagDbImportResponse| r.schema_version.as_str(),
            )
            .await
        }

        /// `POST /api/v1/dag-db/export`.
        pub async fn dagdb_export(
            &self,
            request: DagDbExportRequest,
        ) -> Result<DagDbExportResponse, DagDbClientError> {
            self.send(
                self.specs.dagdb_export(request),
                "dagdb:export",
                exo_dag_db_api::DAGDB_EXPORT_RESPONSE_SCHEMA_VERSION,
                |r: &DagDbExportResponse| r.schema_version.as_str(),
            )
            .await
        }

        /// `POST /api/v1/dag-db/trust-check`.
        pub async fn trust_check(
            &self,
            request: DagDbTrustCheckRequest,
        ) -> Result<DagDbTrustCheckResponse, DagDbClientError> {
            self.send(
                self.specs.trust_check(request),
                "dagdb:trust_check",
                exo_dag_db_api::DAGDB_TRUST_CHECK_RESPONSE_SCHEMA_VERSION,
                |r: &DagDbTrustCheckResponse| r.schema_version.as_str(),
            )
            .await
        }

        /// `POST /api/v1/dag-db/council/decision`.
        pub async fn council_decision(
            &self,
            request: DagDbCouncilDecisionRequest,
        ) -> Result<DagDbCouncilDecisionResponse, DagDbClientError> {
            self.send(
                self.specs.council_decision(request),
                "dagdb:council_decision",
                exo_dag_db_api::DAGDB_COUNCIL_DECISION_RESPONSE_SCHEMA_VERSION,
                |r: &DagDbCouncilDecisionResponse| r.schema_version.as_str(),
            )
            .await
        }

        /// `GET /api/v1/dag-db/receipts/{receipt_hash}`.
        pub async fn receipt_lookup(
            &self,
            request: DagDbReceiptLookupRequest,
        ) -> Result<DagDbReceiptLookupResponse, DagDbClientError> {
            self.send(
                self.specs.receipt_lookup(request),
                "dagdb:receipt_lookup",
                exo_dag_db_api::DAGDB_RECEIPT_LOOKUP_RESPONSE_SCHEMA_VERSION,
                |r: &DagDbReceiptLookupResponse| r.schema_version.as_str(),
            )
            .await
        }

        /// `GET /api/v1/dag-db/catalog/{catalog_id}`.
        pub async fn catalog_lookup(
            &self,
            request: DagDbCatalogLookupRequest,
        ) -> Result<DagDbCatalogLookupResponse, DagDbClientError> {
            self.send(
                self.specs.catalog_lookup(request),
                "dagdb:catalog_lookup",
                exo_dag_db_api::DAGDB_CATALOG_LOOKUP_RESPONSE_SCHEMA_VERSION,
                |r: &DagDbCatalogLookupResponse| r.schema_version.as_str(),
            )
            .await
        }

        /// `GET /api/v1/dag-db/routes/{route_id}`.
        pub async fn route_lookup(
            &self,
            request: DagDbRouteLookupRequest,
        ) -> Result<DagDbRouteLookupResponse, DagDbClientError> {
            self.send(
                self.specs.route_lookup(request),
                "dagdb:route_lookup",
                exo_dag_db_api::DAGDB_ROUTE_LOOKUP_RESPONSE_SCHEMA_VERSION,
                |r: &DagDbRouteLookupResponse| r.schema_version.as_str(),
            )
            .await
        }

        /// Build, sign, send, and map one request spec.
        ///
        /// `action` is the gateway authority action (e.g. `dagdb:intake`) baked
        /// into the `{action}:{tenant}:{namespace}` scope header. `expected` is
        /// the `schema_version` constant this SDK was built against;
        /// `schema_of` extracts the version from the decoded response so a
        /// mismatch is surfaced as [`DagDbClientError::SchemaVersionMismatch`].
        async fn send<Body, Resp>(
            &self,
            spec: DagDbRequestSpec<Body>,
            action: &str,
            expected: &'static str,
            schema_of: impl Fn(&Resp) -> &str,
        ) -> Result<Resp, DagDbClientError>
        where
            Body: Serialize,
            Resp: DeserializeOwned,
        {
            let url = format!("{}{}", self.base_url, spec.path);
            let mut builder = match spec.method {
                DagDbHttpMethod::Get => self.http.get(url),
                DagDbHttpMethod::Post => self.http.post(url),
            };
            builder = builder.headers(self.auth_headers(action)?);
            if let Some(body) = spec.body.as_ref() {
                builder = builder.json(body);
            }

            let response = builder
                .send()
                .await
                .map_err(DagDbClientError::from_reqwest)?;
            let status = response.status();
            if status.is_success() {
                let decoded: Resp = response.json().await.map_err(DagDbClientError::Decode)?;
                let actual = schema_of(&decoded);
                if actual != expected {
                    return Err(DagDbClientError::SchemaVersionMismatch {
                        expected,
                        actual: actual.to_owned(),
                    });
                }
                Ok(decoded)
            } else {
                Err(self.map_error(status, response).await)
            }
        }

        /// Parse a non-2xx response into the governed error variant, falling
        /// back to [`DagDbClientError::UnexpectedStatus`] with the raw body when
        /// the envelope does not parse.
        async fn map_error(
            &self,
            status: StatusCode,
            response: reqwest::Response,
        ) -> DagDbClientError {
            let code = status.as_u16();
            let body = match response.text().await {
                Ok(body) => body,
                Err(err) => return DagDbClientError::from_reqwest(err),
            };
            match serde_json::from_str::<DagDbErrorEnvelope>(&body) {
                Ok(envelope) => DagDbClientError::Server(DagDbServerError {
                    status: code,
                    error_code: envelope.error_code,
                    message: envelope.message,
                    receipt_hash: envelope.receipt_hash,
                    validation_report_id: envelope.validation_report_id,
                    requires_council_review: envelope.requires_council_review,
                }),
                Err(_) => DagDbClientError::UnexpectedStatus { status: code, body },
            }
        }

        /// Assemble the four gateway auth headers for `action`.
        ///
        /// Callers wanting a per-request deadline should build the
        /// `reqwest::Client` with [`reqwest::ClientBuilder::timeout`] and pass
        /// it to [`DagDbHttpClient::with_client`]; an elapsed deadline maps to
        /// [`DagDbClientError::Timeout`].
        fn auth_headers(&self, action: &str) -> Result<HeaderMap, DagDbClientError> {
            let mut headers = HeaderMap::with_capacity(4);
            headers.insert(
                AUTHORIZATION,
                header_value(
                    &format!("Bearer {}", self.auth.token.reveal()),
                    "authorization",
                )?,
            );
            headers.insert(
                HeaderName::from_static(TENANT_HEADER),
                header_value(&self.auth.tenant_id, TENANT_HEADER)?,
            );
            headers.insert(
                HeaderName::from_static(NAMESPACE_HEADER),
                header_value(&self.auth.namespace, NAMESPACE_HEADER)?,
            );
            headers.insert(
                HeaderName::from_static(AUTHORITY_SCOPE_HEADER),
                header_value(&self.auth.authority_scope(action), AUTHORITY_SCOPE_HEADER)?,
            );
            Ok(headers)
        }
    }

    fn trim_trailing_slash(mut base: String) -> String {
        while base.ends_with('/') {
            base.pop();
        }
        base
    }

    fn header_value(value: &str, header: &'static str) -> Result<HeaderValue, DagDbClientError> {
        HeaderValue::from_str(value).map_err(|_| DagDbClientError::InvalidAuthHeader { header })
    }
}

#[cfg(test)]
mod tests {
    use serde::{Serialize, de::DeserializeOwned};

    use super::*;

    #[test]
    fn dagdb_json_fixtures() {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../exo-dag-db-api/fixtures/json/all_dto_fixtures.json"
        ))
        .expect("SDK parses complete DAG DB fixture set");

        assert_fixture::<DagDbIntakeRequest>(&fixtures, "requests", "intake");
        assert_fixture::<DagDbRouteRequest>(&fixtures, "requests", "route");
        assert_fixture::<DagDbContextPacketRequest>(&fixtures, "requests", "context_packet");
        assert_fixture::<DagDbValidateRequest>(&fixtures, "requests", "validate");
        assert_fixture::<DagDbWritebackRequest>(&fixtures, "requests", "writeback");
        assert_fixture::<DagDbTrustCheckRequest>(&fixtures, "requests", "trust_check");
        assert_fixture::<DagDbCouncilDecisionRequest>(&fixtures, "requests", "council_decision");
        assert_fixture::<DagDbReceiptLookupRequest>(&fixtures, "requests", "receipt_lookup");
        assert_fixture::<DagDbCatalogLookupRequest>(&fixtures, "requests", "catalog_lookup");
        assert_fixture::<DagDbRouteLookupRequest>(&fixtures, "requests", "route_lookup");

        assert_fixture::<DagDbIntakeResponse>(&fixtures, "responses", "intake");
        assert_fixture::<DagDbRouteResponse>(&fixtures, "responses", "route");
        assert_fixture::<DagDbContextPacketResponse>(&fixtures, "responses", "context_packet");
        assert_fixture::<DagDbValidateResponse>(&fixtures, "responses", "validate");
        assert_fixture::<DagDbWritebackResponse>(&fixtures, "responses", "writeback");
        assert_fixture::<DagDbImportResponse>(&fixtures, "responses", "import");
        assert_fixture::<DagDbExportResponse>(&fixtures, "responses", "export");
        assert_fixture::<DagDbTrustCheckResponse>(&fixtures, "responses", "trust_check");
        assert_fixture::<DagDbCouncilDecisionResponse>(&fixtures, "responses", "council_decision");
        assert_fixture::<DagDbReceiptLookupResponse>(&fixtures, "responses", "receipt_lookup");
        assert_fixture::<DagDbCatalogLookupResponse>(&fixtures, "responses", "catalog_lookup");
        assert_fixture::<DagDbRouteLookupResponse>(&fixtures, "responses", "route_lookup");

        assert_fixture::<DagDbErrorEnvelope>(&fixtures, "errors", "tenant_scope_mismatch");
    }

    #[test]
    fn dagdb_graph_json_fixtures() {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../exo-dag-db-api/fixtures/json/all_dto_fixtures.json"
        ))
        .expect("SDK parses complete DAG DB fixture set");

        assert_fixture::<MemoryCandidate>(&fixtures, "graph", "memory_candidate");
        assert_fixture::<SimilarityResult>(&fixtures, "graph", "similarity_result");
        assert_fixture::<CanonicalizationDecision>(&fixtures, "graph", "canonicalization_decision");
        assert_fixture::<GraphView>(&fixtures, "graph", "graph_view");
        assert_fixture::<RouteInvalidationReceipt>(
            &fixtures,
            "graph",
            "route_invalidation_receipt",
        );
        assert_fixture::<PlacementResult>(&fixtures, "graph", "placement_result");
    }

    #[test]
    fn dagdb_sdk_helpers_cover_every_route_without_shape_drift() {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../exo-dag-db-api/fixtures/json/all_dto_fixtures.json"
        ))
        .expect("SDK parses complete DAG DB fixture set");
        let client = DagDbClient::new();

        assert_post(
            client.intake(fixture(&fixtures, "requests", "intake")),
            "/api/v1/dag-db/intake",
        );
        assert_post(
            client.route(fixture(&fixtures, "requests", "route")),
            "/api/v1/dag-db/route",
        );
        assert_post(
            client.context_packet(fixture(&fixtures, "requests", "context_packet")),
            "/api/v1/dag-db/context-packet",
        );
        assert_post(
            client.validate(fixture(&fixtures, "requests", "validate")),
            "/api/v1/dag-db/validate",
        );
        assert_post(
            client.writeback(fixture(&fixtures, "requests", "writeback")),
            "/api/v1/dag-db/writeback",
        );
        assert_post(import_request(&client), "/api/v1/dag-db/import");
        assert_post(export_request(&client), "/api/v1/dag-db/export");
        assert_post(
            client.trust_check(fixture(&fixtures, "requests", "trust_check")),
            "/api/v1/dag-db/trust-check",
        );
        assert_post(
            client.council_decision(fixture(&fixtures, "requests", "council_decision")),
            "/api/v1/dag-db/council/decision",
        );

        let receipt = client.receipt_lookup(fixture(&fixtures, "requests", "receipt_lookup"));
        assert_eq!(receipt.method, DagDbHttpMethod::Get);
        assert!(receipt.path.starts_with("/api/v1/dag-db/receipts/"));
        assert!(receipt.path.contains("tenant_id=tenant-a"));
        assert!(receipt.body.is_none());

        let catalog = client.catalog_lookup(fixture(&fixtures, "requests", "catalog_lookup"));
        assert_eq!(catalog.method, DagDbHttpMethod::Get);
        assert!(catalog.path.starts_with("/api/v1/dag-db/catalog/"));
        assert!(catalog.path.contains("include_children=true"));
        assert!(catalog.body.is_none());

        let route = client.route_lookup(fixture(&fixtures, "requests", "route_lookup"));
        assert_eq!(route.method, DagDbHttpMethod::Get);
        assert!(route.path.starts_with("/api/v1/dag-db/routes/"));
        assert!(route.path.contains("include_memory_refs=true"));
        assert!(route.body.is_none());
    }

    fn assert_fixture<T>(fixtures: &serde_json::Value, section: &str, name: &str)
    where
        T: DeserializeOwned + Serialize,
    {
        let fixture = fixtures
            .get(section)
            .and_then(|section| section.get(name))
            .unwrap_or_else(|| panic!("missing fixture {section}.{name}"));
        let parsed: T = serde_json::from_value(fixture.clone())
            .unwrap_or_else(|err| panic!("parse fixture {section}.{name}: {err}"));
        let serialized = serde_json::to_value(parsed)
            .unwrap_or_else(|err| panic!("serialize fixture {section}.{name}: {err}"));
        assert_eq!(serialized, *fixture, "fixture {section}.{name} drifted");
    }

    fn fixture<T>(fixtures: &serde_json::Value, section: &str, name: &str) -> T
    where
        T: DeserializeOwned,
    {
        serde_json::from_value(
            fixtures
                .get(section)
                .and_then(|section| section.get(name))
                .unwrap_or_else(|| panic!("missing fixture {section}.{name}"))
                .clone(),
        )
        .unwrap_or_else(|err| panic!("parse fixture {section}.{name}: {err}"))
    }

    fn assert_post<T>(spec: DagDbRequestSpec<T>, path: &str) {
        assert_eq!(spec.method, DagDbHttpMethod::Post);
        assert_eq!(spec.path, path);
        assert!(spec.body.is_some());
    }

    fn import_request(client: &DagDbClient) -> DagDbRequestSpec<DagDbImportRequest> {
        client.dagdb_import(DagDbImportRequest {
            tenant_id: "tenant-a".to_owned(),
            namespace: "project_memory_v3".to_owned(),
            idempotency_key: "idem-import-1".to_owned(),
            db_set_version: "dag_db-project_memory_v3".to_owned(),
            source_hash: "1111111111111111111111111111111111111111111111111111111111111111"
                .to_owned(),
            requester_did: "did:exo:importer".to_owned(),
            import_report: serde_json::json!({
                "schema_version": "dagdb_kg_dry_run_import_report_v1",
                "source_candidates_schema_version": "dagdb_markdown_kg_import_candidates_v1",
                "graph_root": "KnowledgeGraphs/dag-db",
                "tenant_id": "tenant-a",
                "namespace": "project_memory_v3",
                "actor_did": "did:exo:kg-importer",
                "batch_id": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                "dry_run_only": true,
                "postgres_writes": false,
                "raw_markdown_included": false,
                "proposed_memory_records": [],
                "proposed_catalog_entries": [],
                "proposed_graph_nodes": [],
                "proposed_graph_edges": [],
                "proposed_required_edges": [],
                "proposed_placement_decisions": [],
                "proposed_receipt_intents": [],
                "proposed_validation_reports": [],
                "proposed_governance_reviews": [],
                "proposed_graph_view_refreshes": [],
                "proposed_route_invalidations": [],
                "proposed_subdag_boundaries": [],
                "rollback_plan": {},
                "placement_governance_summary": {},
                "review_items": [],
                "warnings": []
            }),
        })
    }

    fn export_request(client: &DagDbClient) -> DagDbRequestSpec<DagDbExportRequest> {
        client.dagdb_export(DagDbExportRequest {
            tenant_id: "tenant-a".to_owned(),
            namespace: "project_memory_v3".to_owned(),
            idempotency_key: "idem-export-1".to_owned(),
            db_set_version: "dag_db-project_memory_v3".to_owned(),
            requester_did: "did:exo:exporter".to_owned(),
            included_memory_ids: vec![
                "2222222222222222222222222222222222222222222222222222222222222222".to_owned(),
            ],
            included_graph_styles: Vec::new(),
            included_writeback_idempotency_keys: Vec::new(),
            source_commit_or_repo_ref: None,
            include_preview_context: false,
        })
    }
}

#[cfg(all(test, feature = "http-client"))]
mod transport_tests {
    use std::time::Duration;

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        sync::oneshot,
    };

    use super::{
        DagDbIntakeRequest, DagDbReceiptLookupRequest,
        transport::{DagDbAuthConfig, DagDbClientError, DagDbHttpClient},
    };

    /// The raw HTTP request a [`TestServer`] captured from the SDK.
    struct CapturedRequest {
        request_line: String,
        headers: Vec<(String, String)>,
        body: String,
    }

    impl CapturedRequest {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.as_str())
        }
    }

    /// Minimal in-process HTTP/1.1 server: accepts one connection, captures the
    /// request, writes a canned response, then hands the captured request back.
    /// No external mock-server crate is a workspace dependency, so this stands
    /// in for a live gateway using only `tokio`.
    struct TestServer {
        base_url: String,
        captured: oneshot::Receiver<CapturedRequest>,
    }

    impl TestServer {
        /// Spawn a server that replies with `status`/`body` and captures the request.
        async fn spawn(status_line: &'static str, body: impl Into<String>) -> Self {
            let body = body.into();
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind test server");
            let addr = listener.local_addr().expect("server addr");
            let base_url = format!("http://{addr}");
            let (tx, captured) = oneshot::channel();
            tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.expect("accept connection");
                let request = read_request(&mut stream).await;
                let response = format!(
                    "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .await
                    .expect("write response");
                stream.flush().await.expect("flush response");
                let _ = tx.send(request);
            });
            Self { base_url, captured }
        }

        /// Spawn a server that accepts a connection but never replies, so the
        /// client's timeout fires.
        async fn spawn_silent() -> String {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind silent server");
            let addr = listener.local_addr().expect("silent addr");
            tokio::spawn(async move {
                let _conn = listener.accept().await;
                // Hold the connection open without responding.
                tokio::time::sleep(Duration::from_secs(30)).await;
            });
            format!("http://{addr}")
        }

        async fn captured(self) -> CapturedRequest {
            self.captured.await.expect("server captured a request")
        }
    }

    /// Read one HTTP/1.1 request: request line, headers, and Content-Length body.
    async fn read_request(stream: &mut TcpStream) -> CapturedRequest {
        let mut buf = Vec::new();
        let header_end = loop {
            let mut chunk = [0_u8; 1024];
            let n = stream.read(&mut chunk).await.expect("read request bytes");
            assert!(n > 0, "connection closed before headers were complete");
            buf.extend_from_slice(&chunk[..n]);
            if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
                break pos;
            }
        };
        let head = String::from_utf8(buf[..header_end].to_vec()).expect("utf8 request head");
        let mut lines = head.split("\r\n");
        let request_line = lines.next().unwrap_or_default().to_owned();
        let mut headers = Vec::new();
        let mut content_length = 0_usize;
        for line in lines {
            if let Some((key, value)) = line.split_once(": ") {
                if key.eq_ignore_ascii_case("content-length") {
                    content_length = value.trim().parse().unwrap_or(0);
                }
                headers.push((key.to_owned(), value.to_owned()));
            }
        }
        let mut body_bytes = buf[header_end + 4..].to_vec();
        while body_bytes.len() < content_length {
            let mut chunk = [0_u8; 1024];
            let n = stream.read(&mut chunk).await.expect("read body bytes");
            if n == 0 {
                break;
            }
            body_bytes.extend_from_slice(&chunk[..n]);
        }
        let body = String::from_utf8(body_bytes).expect("utf8 request body");
        CapturedRequest {
            request_line,
            headers,
            body,
        }
    }

    fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    fn auth() -> DagDbAuthConfig {
        DagDbAuthConfig::new("super-secret-token-value", "tenant-a", "primary")
    }

    fn fixtures() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../exo-dag-db-api/fixtures/json/all_dto_fixtures.json"
        ))
        .expect("parse fixture set")
    }

    fn fixture_request<T: serde::de::DeserializeOwned>(name: &str) -> T {
        serde_json::from_value(fixtures()["requests"][name].clone()).expect("parse request fixture")
    }

    fn intake_request() -> DagDbIntakeRequest {
        fixture_request("intake")
    }

    fn receipt_lookup_request() -> DagDbReceiptLookupRequest {
        fixture_request("receipt_lookup")
    }

    fn fixture_response(section: &str, name: &str) -> String {
        fixtures()
            .get(section)
            .and_then(|s| s.get(name))
            .expect("fixture exists")
            .to_string()
    }

    // (a) A POST builds the correct path + all four auth headers with the
    // correct `{action}:{tenant}:{namespace}` scope.
    #[tokio::test]
    async fn post_attaches_path_and_four_auth_headers_with_scope() {
        let body = fixture_response("responses", "intake");
        let server = TestServer::spawn("200 OK", body).await;
        let client = DagDbHttpClient::new(&server.base_url, auth()).expect("client");

        let _ = client.intake(intake_request()).await;
        let request = server.captured().await;

        assert!(
            request
                .request_line
                .starts_with("POST /api/v1/dag-db/intake "),
            "request line was {:?}",
            request.request_line
        );
        assert_eq!(
            request.header("authorization"),
            Some("Bearer super-secret-token-value")
        );
        assert_eq!(request.header("x-exo-tenant-id"), Some("tenant-a"));
        assert_eq!(request.header("x-exo-namespace"), Some("primary"));
        assert_eq!(
            request.header("x-exo-authority-scope"),
            Some("dagdb:intake:tenant-a:primary")
        );
        assert!(
            request
                .body
                .contains("\"idempotency_key\":\"idem-intake-1\""),
            "body was {}",
            request.body
        );
    }

    // (a) A GET builds the correct path + the four headers with the route's
    // own action in the scope.
    #[tokio::test]
    async fn get_attaches_path_and_scope_for_receipt_lookup() {
        let body = fixture_response("responses", "receipt_lookup");
        let server = TestServer::spawn("200 OK", body).await;
        let client = DagDbHttpClient::new(&server.base_url, auth()).expect("client");

        let _ = client.receipt_lookup(receipt_lookup_request()).await;
        let request = server.captured().await;

        assert!(
            request
                .request_line
                .starts_with("GET /api/v1/dag-db/receipts/"),
            "request line was {:?}",
            request.request_line
        );
        assert!(request.request_line.contains("tenant_id=tenant-a"));
        assert_eq!(
            request.header("x-exo-authority-scope"),
            Some("dagdb:receipt_lookup:tenant-a:primary")
        );
        assert!(request.body.is_empty(), "GET body should be empty");
    }

    // (b) A 2xx body deserializes into the right versioned DTO.
    #[tokio::test]
    async fn success_body_deserializes_into_versioned_dto() {
        let body = fixture_response("responses", "intake");
        let server = TestServer::spawn("200 OK", body).await;
        let client = DagDbHttpClient::new(&server.base_url, auth()).expect("client");

        let response = client
            .intake(intake_request())
            .await
            .expect("2xx maps to DTO");
        assert_eq!(response.schema_version, "dagdb_intake_response_v1");
        assert_eq!(response.tenant_id, "tenant-a");
        assert_eq!(response.idempotency_key, "idem-intake-1");
    }

    // (c) A non-2xx body maps to the typed server error with error_code.
    #[tokio::test]
    async fn error_body_maps_to_typed_server_error() {
        let body = fixture_response("errors", "tenant_scope_mismatch");
        let server = TestServer::spawn("403 Forbidden", body).await;
        let client = DagDbHttpClient::new(&server.base_url, auth()).expect("client");

        let err = client
            .intake(intake_request())
            .await
            .expect_err("non-2xx is an error");
        match err {
            DagDbClientError::Server(server_err) => {
                assert_eq!(server_err.status, 403);
                assert_eq!(server_err.error_code, "tenant_scope_mismatch");
                assert!(!server_err.requires_council_review);
            }
            other => panic!("expected Server error, got {other:?}"),
        }
    }

    // (c') A non-2xx body that is NOT a valid envelope maps to UnexpectedStatus
    // carrying the raw body, never swallowed.
    #[tokio::test]
    async fn non_envelope_error_body_maps_to_unexpected_status() {
        let server = TestServer::spawn("502 Bad Gateway", "upstream exploded").await;
        let client = DagDbHttpClient::new(&server.base_url, auth()).expect("client");

        let err = client
            .intake(intake_request())
            .await
            .expect_err("non-2xx is an error");
        match err {
            DagDbClientError::UnexpectedStatus { status, body } => {
                assert_eq!(status, 502);
                assert_eq!(body, "upstream exploded");
            }
            other => panic!("expected UnexpectedStatus, got {other:?}"),
        }
    }

    // (d) A transport/timeout error maps to the Timeout variant.
    #[tokio::test]
    async fn timeout_maps_to_timeout_variant() {
        let base_url = TestServer::spawn_silent().await;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(150))
            .build()
            .expect("client with timeout");
        let client = DagDbHttpClient::with_client(&base_url, auth(), http);

        let err = client
            .intake(intake_request())
            .await
            .expect_err("timeout is an error");
        assert!(
            matches!(err, DagDbClientError::Timeout(_)),
            "expected Timeout, got {err:?}"
        );
    }

    // (d') A connect failure (nothing listening) maps to the Transport variant.
    #[tokio::test]
    async fn connect_failure_maps_to_transport_variant() {
        // Reserve then drop a port so nothing is listening on it.
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        drop(listener);
        let client = DagDbHttpClient::new(format!("http://{addr}"), auth()).expect("client");

        let err = client
            .intake(intake_request())
            .await
            .expect_err("connect failure is an error");
        assert!(
            matches!(err, DagDbClientError::Transport(_)),
            "expected Transport, got {err:?}"
        );
    }

    // (d'') A 2xx body whose schema_version mismatches the SDK constant is
    // surfaced, not silently accepted.
    #[tokio::test]
    async fn schema_version_mismatch_is_surfaced() {
        let mut intake = fixtures()["responses"]["intake"].clone();
        intake["schema_version"] =
            serde_json::Value::String("dagdb_intake_response_v999".to_owned());
        let body = intake.to_string();
        let server = TestServer::spawn("200 OK", body).await;
        let client = DagDbHttpClient::new(&server.base_url, auth()).expect("client");

        let err = client
            .intake(intake_request())
            .await
            .expect_err("mismatch is an error");
        match err {
            DagDbClientError::SchemaVersionMismatch { expected, actual } => {
                assert_eq!(expected, "dagdb_intake_response_v1");
                assert_eq!(actual, "dagdb_intake_response_v999");
            }
            other => panic!("expected SchemaVersionMismatch, got {other:?}"),
        }
    }

    // (e) The bearer token is never exposed through Debug.
    #[tokio::test]
    async fn bearer_token_is_redacted_in_debug() {
        let config = auth();
        let rendered = format!("{config:?}");
        assert!(
            !rendered.contains("super-secret-token-value"),
            "auth config Debug leaked the token: {rendered}"
        );
        assert!(
            rendered.contains("redacted"),
            "expected redaction marker: {rendered}"
        );
    }
}
