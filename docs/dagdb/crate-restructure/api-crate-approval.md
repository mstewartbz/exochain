# DAG DB API Crate Approval

Schema: `dagdb_api_crate_approval_v1`

## Status

`exo-dag-db-api` is the approved owner for DAG DB wire DTOs and DTO fixtures in
the crate restructure.

| surface | owner or compatibility path |
| --- | --- |
| DTO structs and schema constants | `crates/exo-dag-db-api` |
| DTO JSON fixtures | `crates/exo-dag-db-api/fixtures/json` |
| REST/API compatibility module | `exo-api::dagdb` re-exports `exo-dag-db-api` |
| Rust SDK compatibility module | `exochain-sdk::dagdb` re-exports `exo-dag-db-api` DTOs |
| OpenAPI artifact | `docs/dagdb/api/openapi.json` |

## Contract

- `exo-api::dagdb` remains a compatibility re-export, not the DTO owner.
- `exochain-sdk::dagdb` should not redefine DAG DB DTOs.
- OpenAPI synchronization is verified against fixtures from
  `crates/exo-dag-db-api/fixtures/json/all_dto_fixtures.json`.
- Route names under `/api/v1/dag-db/**` and MCP tool names remain compatibility
  surfaces and are not changed by this crate split.

## Verification

Use these checks when changing DTOs or API compatibility:

```bash
cargo test -p exo-dag-db-api
cargo test -p exo-api --test openapi_sync
cargo test -p exochain-sdk --features http-client
```

This approval is repository/test scope only. It does not approve production
runtime rollout or live operator evidence.
