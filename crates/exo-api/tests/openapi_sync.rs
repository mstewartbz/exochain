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

//! CI-asserted sync between the checked-in DAG DB machine contract
//! (`docs/dagdb/api/openapi.json`) and the live `exo-api` DTOs / fixtures.
//!
//! The published OpenAPI 3.1 spec is the artifact a non-Rust integrator
//! codegens from. This test makes it impossible for that spec to silently drift
//! from the Rust wire shapes:
//!
//! 1. Every request/response/error fixture in
//!    `crates/exo-dag-db-api/fixtures/json/all_dto_fixtures.json` validates against
//!    its component schema in the spec. Because each fixture is independently
//!    round-trip-asserted against its Rust DTO (see `dagdb::tests` in
//!    `src/dagdb.rs`), validating fixtures against the spec transitively binds
//!    the spec's field set to the DTO's field set — a spec field that the DTO
//!    drops, or a DTO field the spec forgets to allow (under
//!    `additionalProperties: false`), fails this test.
//! 2. The documented `schema_version` const for every response schema equals the
//!    Rust `DAGDB_*_RESPONSE_SCHEMA_VERSION` constant, and equals the value in
//!    the corresponding response fixture. This is the three-way binding
//!    (constant <-> spec <-> fixture) that anchors the version contract.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use exo_api::dagdb;
use jsonschema::{Draft, JSONSchema};
use serde_json::Value;

const OPENAPI_JSON: &str = include_str!("../../../docs/dagdb/api/openapi.json");
const FIXTURES_JSON: &str =
    include_str!("../../exo-dag-db-api/fixtures/json/all_dto_fixtures.json");

fn spec() -> Value {
    serde_json::from_str(OPENAPI_JSON).expect("parse docs/dagdb/api/openapi.json")
}

fn fixtures() -> Value {
    serde_json::from_str(FIXTURES_JSON).expect("parse all_dto_fixtures.json")
}

/// Compile a single component schema into a validator that can resolve the
/// `#/components/schemas/...` `$ref`s used throughout the spec by carrying the
/// full `components` block at the validation-document root.
fn compile_component(spec: &Value, schema_name: &str) -> JSONSchema {
    let components = spec.get("components").expect("spec has components").clone();
    assert!(
        components
            .get("schemas")
            .and_then(|schemas| schemas.get(schema_name))
            .is_some(),
        "spec is missing component schema {schema_name}"
    );
    let root = serde_json::json!({
        "$ref": format!("#/components/schemas/{schema_name}"),
        "components": components,
    });
    JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&root)
        .unwrap_or_else(|err| panic!("compile component {schema_name}: {err}"))
}

fn assert_fixture_validates(
    spec: &Value,
    fixtures: &Value,
    section: &str,
    name: &str,
    schema: &str,
) {
    let instance = fixtures
        .get(section)
        .and_then(|section| section.get(name))
        .unwrap_or_else(|| panic!("missing fixture {section}.{name}"));
    let validator = compile_component(spec, schema);
    if let Err(errors) = validator.validate(instance) {
        let messages: Vec<String> = errors
            .map(|error| format!("{} at {}", error, error.instance_path))
            .collect();
        panic!(
            "fixture {section}.{name} does not validate against spec schema {schema}:\n{}",
            messages.join("\n")
        );
    }
}

/// `(spec schema name, fixture name, Rust schema_version constant)` for every
/// versioned consumer-facing response DTO.
fn response_contract() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "DagDbIntakeResponse",
            "intake",
            dagdb::DAGDB_INTAKE_RESPONSE_SCHEMA_VERSION,
        ),
        (
            "DagDbRouteResponse",
            "route",
            dagdb::DAGDB_ROUTE_RESPONSE_SCHEMA_VERSION,
        ),
        (
            "DagDbContextPacketResponse",
            "context_packet",
            dagdb::DAGDB_CONTEXT_PACKET_RESPONSE_SCHEMA_VERSION,
        ),
        (
            "DagDbValidateResponse",
            "validate",
            dagdb::DAGDB_VALIDATE_RESPONSE_SCHEMA_VERSION,
        ),
        (
            "DagDbWritebackResponse",
            "writeback",
            dagdb::DAGDB_WRITEBACK_RESPONSE_SCHEMA_VERSION,
        ),
        (
            "DagDbImportResponse",
            "import",
            dagdb::DAGDB_IMPORT_RESPONSE_SCHEMA_VERSION,
        ),
        (
            "DagDbExportResponse",
            "export",
            dagdb::DAGDB_EXPORT_RESPONSE_SCHEMA_VERSION,
        ),
        (
            "DagDbTrustCheckResponse",
            "trust_check",
            dagdb::DAGDB_TRUST_CHECK_RESPONSE_SCHEMA_VERSION,
        ),
        (
            "DagDbCouncilDecisionResponse",
            "council_decision",
            dagdb::DAGDB_COUNCIL_DECISION_RESPONSE_SCHEMA_VERSION,
        ),
        (
            "DagDbReceiptLookupResponse",
            "receipt_lookup",
            dagdb::DAGDB_RECEIPT_LOOKUP_RESPONSE_SCHEMA_VERSION,
        ),
        (
            "DagDbCatalogLookupResponse",
            "catalog_lookup",
            dagdb::DAGDB_CATALOG_LOOKUP_RESPONSE_SCHEMA_VERSION,
        ),
        (
            "DagDbRouteLookupResponse",
            "route_lookup",
            dagdb::DAGDB_ROUTE_LOOKUP_RESPONSE_SCHEMA_VERSION,
        ),
    ]
}

#[test]
fn openapi_doc_parses_and_declares_every_live_route() {
    let spec = spec();
    assert_eq!(spec.get("openapi").and_then(Value::as_str), Some("3.1.0"));
    let paths = spec.get("paths").expect("spec has paths");
    for route in [
        "/route",
        "/context-packet",
        "/writeback",
        "/import",
        "/export",
    ] {
        assert!(paths.get(route).is_some(), "spec is missing path {route}");
    }
}

#[test]
fn every_response_fixture_validates_against_spec_and_versions_agree() {
    let spec = spec();
    let fixtures = fixtures();
    for (schema, fixture_name, rust_const) in response_contract() {
        // (1) The fixture validates against the published response schema.
        assert_fixture_validates(&spec, &fixtures, "responses", fixture_name, schema);

        // (2a) The Rust constant equals the documented `const` in the spec.
        let spec_const = spec
            .pointer(&format!(
                "/components/schemas/{schema}/properties/schema_version/const"
            ))
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                panic!("spec schema {schema} is missing properties.schema_version.const")
            });
        assert_eq!(
            spec_const, rust_const,
            "spec schema_version const for {schema} ({spec_const}) does not match Rust constant ({rust_const})"
        );

        // (2b) The fixture carries the same `schema_version` value.
        let fixture_version = fixtures
            .pointer(&format!("/responses/{fixture_name}/schema_version"))
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("response fixture {fixture_name} is missing schema_version"));
        assert_eq!(
            fixture_version, rust_const,
            "response fixture {fixture_name} schema_version ({fixture_version}) does not match Rust constant ({rust_const})"
        );
    }
}

#[test]
fn every_request_and_error_fixture_validates_against_spec() {
    let spec = spec();
    let fixtures = fixtures();
    let requests: &[(&str, &str)] = &[
        ("intake", "DagDbIntakeRequest"),
        ("route", "DagDbRouteRequest"),
        ("context_packet", "DagDbContextPacketRequest"),
        ("validate", "DagDbValidateRequest"),
        ("writeback", "DagDbWritebackRequest"),
        ("trust_check", "DagDbTrustCheckRequest"),
        ("council_decision", "DagDbCouncilDecisionRequest"),
        ("receipt_lookup", "DagDbReceiptLookupRequest"),
        ("catalog_lookup", "DagDbCatalogLookupRequest"),
        ("route_lookup", "DagDbRouteLookupRequest"),
    ];
    for (fixture_name, schema) in requests {
        assert_fixture_validates(&spec, &fixtures, "requests", fixture_name, schema);
    }
    assert_fixture_validates(
        &spec,
        &fixtures,
        "errors",
        "tenant_scope_mismatch",
        "DagDbErrorEnvelope",
    );
}
