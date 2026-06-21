// End-to-end hot-path latency benchmark for exo-dag-db against a LIVE Postgres.
//
// Measures the real wall-clock cost of the three operations that dominate the
// agent-memory request path, each reported with criterion's full latency
// distribution (median/p50 plus the tail):
//   * kg_import     — persist a dry-run import report into the store;
//   * kg_retrieval  — select + assemble a context packet (the packet-build path);
//   * kg_writeback  — persist a validated writeback proposal.
//
// This is the measured source of truth that replaces the synthetic integer
// `LatencyBreakdown::from_inputs` formula in src/diagnostics.rs. Skips cleanly
// (no failure) when no Postgres is reachable, like the contract-test suite.
#![cfg(feature = "postgres")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use criterion::{Criterion, criterion_group, criterion_main};
use exo_dag_db_postgres::postgres::{
    kg_import::persist_kg_import_report, kg_retrieval::retrieve_kg_context_packet,
    kg_writeback::persist_kg_writeback_report,
};
use tokio::runtime::Runtime;

#[path = "common/mod.rs"]
mod common;

fn runtime() -> Runtime {
    Runtime::new().expect("build bench tokio runtime")
}

fn bench_kg_import(c: &mut Criterion) {
    let rt = runtime();
    let report = common::scale_import_report_json(common::SMOKE_TIER);

    let mut group = c.benchmark_group("kg_import");
    group.sample_size(20);
    // Each iteration imports into a fresh isolated schema so the measurement is
    // a cold first-import (no idempotency-replay short-circuit) and never
    // pollutes a shared store.
    group.bench_function("persist_import_report", |b| {
        b.iter(|| {
            rt.block_on(async {
                let Some(store) = common::BenchStore::try_new("import", 1).await else {
                    common::skip("kg_import");
                    return;
                };
                persist_kg_import_report(&store.pool, &report)
                    .await
                    .expect("persist bench import report");
                store.cleanup().await;
            });
        });
    });
    group.finish();
}

fn bench_kg_retrieval(c: &mut Criterion) {
    let rt = runtime();
    // Seed once; measure only the retrieval/packet-build path against it.
    let Some(store) = rt.block_on(common::BenchStore::try_new("retrieval", 1)) else {
        common::skip("kg_retrieval");
        return;
    };
    rt.block_on(async {
        let report = common::scale_import_report_json(common::SMOKE_TIER);
        persist_kg_import_report(&store.pool, &report)
            .await
            .expect("seed retrieval bench store");
    });
    let request = common::retrieval_request(2_000, Some(32));

    let mut group = c.benchmark_group("kg_retrieval");
    group.bench_function("context_packet_build", |b| {
        b.iter(|| {
            rt.block_on(async {
                retrieve_kg_context_packet(&store.pool, &request)
                    .await
                    .expect("retrieve bench context packet");
            });
        });
    });
    group.finish();

    rt.block_on(store.cleanup());
}

fn bench_kg_writeback(c: &mut Criterion) {
    let rt = runtime();
    // Build one preview to drive the writeback, then bench writeback into a
    // fresh schema per iteration (cold persist, no idempotency replay).
    let preview = {
        let Some(store) = rt.block_on(common::BenchStore::try_new("writeback_seed", 1)) else {
            common::skip("kg_writeback");
            return;
        };
        let preview = rt.block_on(async {
            let report = common::scale_import_report_json(common::SMOKE_TIER);
            persist_kg_import_report(&store.pool, &report)
                .await
                .expect("seed writeback bench store");
            retrieve_kg_context_packet(&store.pool, &common::retrieval_request(2_000, Some(2)))
                .await
                .expect("retrieve writeback bench preview")
        });
        rt.block_on(store.cleanup());
        preview
    };
    let report_json =
        serde_json::to_string(&common::writeback_report(&preview)).expect("serialize writeback");

    let mut group = c.benchmark_group("kg_writeback");
    group.sample_size(20);
    group.bench_function("persist_writeback_report", |b| {
        b.iter(|| {
            rt.block_on(async {
                let Some(store) = common::BenchStore::try_new("writeback", 1).await else {
                    common::skip("kg_writeback");
                    return;
                };
                // Seed the memory the writeback's existing-memory evidence
                // points at, then persist the writeback proposal (the measured
                // operation includes the import seed in this group; the
                // dedicated kg_import group isolates import on its own).
                let seed = common::scale_import_report_json(common::SMOKE_TIER);
                persist_kg_import_report(&store.pool, &seed)
                    .await
                    .expect("seed writeback iteration store");
                persist_kg_writeback_report(&store.pool, &report_json)
                    .await
                    .expect("persist bench writeback report");
                store.cleanup().await;
            });
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_kg_import,
    bench_kg_retrieval,
    bench_kg_writeback,
);
criterion_main!(benches);
