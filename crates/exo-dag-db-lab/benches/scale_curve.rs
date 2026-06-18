// Latency-vs-corpus-size benchmark for exo-dag-db against a LIVE Postgres.
//
// Seeds the store at and beyond production scale (the named tiers in
// common::SCALE_TIERS — ~4,326 production-corpus items and a ~10,000 headroom
// tier) and measures the context-packet build (retrieval) latency at each tier,
// so the latency curve is REAL rather than the 1,200-item synthetic ceiling in
// src/benchmark.rs. Reported with criterion's full distribution per tier.
//
// Skips cleanly when no Postgres is reachable.
#![cfg(feature = "postgres")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use exo_dag_db_postgres::postgres::{
    kg_import::persist_kg_import_report, kg_retrieval::retrieve_kg_context_packet,
};
use tokio::runtime::Runtime;

#[path = "common/mod.rs"]
mod common;

fn bench_retrieval_scale_curve(c: &mut Criterion) {
    let rt = Runtime::new().expect("build bench tokio runtime");

    let mut group = c.benchmark_group("retrieval_scale_curve");
    // The seed step at 10k items is multi-second; keep the sample count modest
    // so the suite stays runnable, while still capturing a tail.
    group.sample_size(15);

    for &tier in &common::SCALE_TIERS {
        let label = format!("tier_{tier}");
        let Some(store) = rt.block_on(common::BenchStore::try_new(&label, 1)) else {
            common::skip(&format!("retrieval_scale_curve/{label}"));
            return;
        };
        rt.block_on(async {
            let report = common::scale_import_report_json(tier);
            persist_kg_import_report(&store.pool, &report)
                .await
                .expect("seed scale-curve store at tier");
        });

        // Token budget + ref cap large enough to exercise selection over the
        // whole seeded corpus, so the cost scales with corpus size.
        let request = common::retrieval_request(50_000, Some(512));
        group.bench_with_input(
            BenchmarkId::new("context_packet_build", tier),
            &tier,
            |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        retrieve_kg_context_packet(&store.pool, &request)
                            .await
                            .expect("retrieve scale-curve context packet");
                    });
                });
            },
        );

        rt.block_on(store.cleanup());
    }

    group.finish();
}

criterion_group!(benches, bench_retrieval_scale_curve);
criterion_main!(benches);
