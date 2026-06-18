// Concurrency / load benchmark for exo-dag-db against a LIVE Postgres.
//
// Exercises the receipt-append path — which runs each write under
// `SET TRANSACTION ISOLATION LEVEL SERIALIZABLE` plus a per-subject advisory
// lock (see src/receipt.rs) — under N concurrent tasks, and reports a custom
// throughput + tail-latency summary (p50/p99/max + ops/sec) that criterion's
// per-iteration timing does not surface. Also samples process RSS before/after
// the load so a memory-footprint figure is recorded.
//
// This proves the contended write/read paths under real concurrency, not just
// the correctness asserted by the existing concurrency *contract* tests.
//
// Skips cleanly when no Postgres is reachable.
#![cfg(feature = "postgres")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use criterion::{Criterion, criterion_group, criterion_main};
use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{ReceiptEventType, SubjectKind};
use exo_dag_db_postgres::receipt::{
    ReceiptAppendRequest, ReceiptAppendResult, ReceiptStoreError, append_receipt,
    reconstruct_receipt_chain,
};
use serde_json::json;
use sqlx::PgPool;
use tokio::{runtime::Runtime, sync::Barrier};

#[path = "common/mod.rs"]
mod common;

/// Number of concurrent tasks for the parallel-writers/readers load.
const CONCURRENCY: usize = 16;
/// Operations issued per concurrent task.
const OPS_PER_TASK: usize = 32;
/// Bounded retries on a SERIALIZABLE serialization failure (Postgres 40001).
///
/// The receipt path runs each write under SERIALIZABLE isolation, where SSI can
/// abort a transaction with 40001 ("might succeed if retried") even across
/// distinct subjects due to predicate-lock false conflicts. `append_receipt`
/// already retries once internally; under this 16-way load that is sometimes
/// not enough, so the bench retries a bounded number of times and counts the
/// retries as part of the contention cost rather than panicking.
const MAX_SERIALIZATION_RETRIES: usize = 32;

/// `true` when the error is a Postgres 40001 serialization failure.
fn is_serialization_failure(error: &ReceiptStoreError) -> bool {
    matches!(
        error,
        ReceiptStoreError::Postgres { source: sqlx::Error::Database(db) }
            if db.code().as_deref() == Some("40001")
    )
}

/// Append a receipt, retrying on serialization failure. Returns the result and
/// the number of retries consumed.
async fn append_with_retry(
    pool: &PgPool,
    request: &ReceiptAppendRequest,
) -> (ReceiptAppendResult, usize) {
    let mut retries = 0;
    loop {
        match append_receipt(pool, request).await {
            Ok(result) => return (result, retries),
            Err(error)
                if is_serialization_failure(&error) && retries < MAX_SERIALIZATION_RETRIES =>
            {
                retries += 1;
            }
            Err(error) => panic!("concurrent receipt append (retries={retries}): {error:?}"),
        }
    }
}

/// Subject id derived from (iteration, task) so every criterion iteration
/// writes a fresh receipt chain and never re-forks a previous iteration's head.
fn subject_id(iteration: u64, task: usize) -> Hash256 {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&iteration.to_le_bytes());
    bytes[8..16].copy_from_slice(&u64::try_from(task).unwrap_or(u64::MAX).to_le_bytes());
    Hash256::from_bytes(bytes)
}

fn writer_request(subject: Hash256, seq_byte: u8) -> ReceiptAppendRequest {
    let body_hash = Hash256::from_bytes([seq_byte; 32]);
    ReceiptAppendRequest {
        tenant_id: "tenant-bench".into(),
        namespace: "dag-db".into(),
        subject_kind: SubjectKind::Memory,
        subject_id: subject,
        expected_prev_receipt_hash: Hash256::ZERO,
        event_type: ReceiptEventType::IntakeCreated,
        actor_did: "did:exo:bench-agent".into(),
        event_hlc: Timestamp::new(10_000, u32::from(seq_byte)),
        event_body_hash: body_hash,
        receipt_body: json!({ "event": "intake_created", "bench": true }),
    }
}

/// Summarize a latency sample set: p50/p99/max and ops/sec over `wall`.
struct LoadSummary {
    op_count: usize,
    wall: Duration,
    p50: Duration,
    p99: Duration,
    max: Duration,
}

impl LoadSummary {
    fn from(mut samples: Vec<Duration>, wall: Duration) -> Self {
        samples.sort_unstable();
        let op_count = samples.len();
        // Integer percentile index: (len - 1) * pct / 100, no float casts.
        let pick = |pct: usize| -> Duration {
            if samples.is_empty() {
                return Duration::ZERO;
            }
            let idx = (samples.len() - 1).saturating_mul(pct) / 100;
            samples[idx.min(samples.len() - 1)]
        };
        Self {
            op_count,
            wall,
            p50: pick(50),
            p99: pick(99),
            max: samples.last().copied().unwrap_or(Duration::ZERO),
        }
    }

    /// Throughput in whole ops/sec, computed with integer nanosecond math so no
    /// float arithmetic is needed (the suite denies `clippy::float_arithmetic`).
    fn ops_per_sec(&self) -> u64 {
        let wall_nanos = self.wall.as_nanos();
        if wall_nanos == 0 {
            return 0;
        }
        let ops = u128::try_from(self.op_count).unwrap_or(u128::MAX);
        u64::try_from(ops.saturating_mul(1_000_000_000) / wall_nanos).unwrap_or(u64::MAX)
    }

    fn report(&self, label: &str) {
        eprintln!(
            "[exo-dag-db concurrency] {label}: ops={} wall={:?} ops/sec={} \
             p50={:?} p99={:?} max={:?}",
            self.op_count,
            self.wall,
            self.ops_per_sec(),
            self.p50,
            self.p99,
            self.max,
        );
    }
}

/// Drive N concurrent writers, each appending `OPS_PER_TASK` receipts to its
/// own subject. Distinct subjects still contend on the shared head table,
/// advisory-lock space, and SSI predicate locks. Returns the per-op latency
/// summary plus the total number of 40001 retries consumed under load.
async fn run_parallel_writers(pool: &PgPool, iteration: u64) -> (LoadSummary, usize) {
    let barrier = Arc::new(Barrier::new(CONCURRENCY));
    let mut handles = Vec::with_capacity(CONCURRENCY);
    for task in 0..CONCURRENCY {
        let pool = pool.clone();
        let barrier = barrier.clone();
        let subject = subject_id(iteration, task);
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            let mut latencies = Vec::with_capacity(OPS_PER_TASK);
            let mut task_retries = 0usize;
            let mut prev = Hash256::ZERO;
            for op in 0..OPS_PER_TASK {
                let seq_byte = u8::try_from(op + 1).expect("ops fit u8");
                let mut request = writer_request(subject, seq_byte);
                request.expected_prev_receipt_hash = prev;
                let started = Instant::now();
                let (result, retries) = append_with_retry(&pool, &request).await;
                latencies.push(started.elapsed());
                task_retries += retries;
                prev = result.receipt_hash;
            }
            (latencies, task_retries)
        }));
    }
    let wall_start = Instant::now();
    let mut samples = Vec::with_capacity(CONCURRENCY * OPS_PER_TASK);
    let mut total_retries = 0usize;
    for handle in handles {
        let (latencies, retries) = handle.await.expect("writer task joins");
        samples.extend(latencies);
        total_retries += retries;
    }
    (
        LoadSummary::from(samples, wall_start.elapsed()),
        total_retries,
    )
}

/// Drive N concurrent readers reconstructing receipt chains for the subjects
/// seeded under `iteration`. Returns the per-op latency summary.
async fn run_parallel_readers(pool: &PgPool, iteration: u64) -> LoadSummary {
    let barrier = Arc::new(Barrier::new(CONCURRENCY));
    let mut handles = Vec::with_capacity(CONCURRENCY);
    for task in 0..CONCURRENCY {
        let pool = pool.clone();
        let barrier = barrier.clone();
        let subject = subject_id(iteration, task);
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            let mut latencies = Vec::with_capacity(OPS_PER_TASK);
            for _ in 0..OPS_PER_TASK {
                let started = Instant::now();
                reconstruct_receipt_chain(
                    &pool,
                    "tenant-bench",
                    "dag-db",
                    SubjectKind::Memory,
                    subject,
                )
                .await
                .expect("concurrent receipt chain read");
                latencies.push(started.elapsed());
            }
            latencies
        }));
    }
    let wall_start = Instant::now();
    let mut samples = Vec::with_capacity(CONCURRENCY * OPS_PER_TASK);
    for handle in handles {
        samples.extend(handle.await.expect("reader task joins"));
    }
    LoadSummary::from(samples, wall_start.elapsed())
}

/// Current process resident-set size in KiB, if obtainable on this platform.
fn rss_kib() -> Option<u64> {
    // macOS / Linux: `ps -o rss= -p <pid>` reports RSS in KiB.
    let pid = std::process::id();
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
}

fn bench_concurrency(c: &mut Criterion) {
    let rt = Runtime::new().expect("build bench tokio runtime");
    // A pool sized for the concurrent load (a few spare over CONCURRENCY).
    let Some(store) = rt.block_on(common::BenchStore::try_new(
        "concurrency",
        u32::try_from(CONCURRENCY + 4).expect("pool size fits u32"),
    )) else {
        common::skip("concurrency");
        return;
    };

    let rss_before = rss_kib();

    // Writers: each iteration uses a unique iteration id so it writes a fresh
    // set of chains (never re-forking a prior head). Capture write-under-
    // contention latency plus the 40001 serialization-retry count.
    let mut group = c.benchmark_group("concurrency");
    group.sample_size(10);
    let writer_iteration = AtomicU64::new(0);
    group.bench_function("parallel_writers", |b| {
        b.iter(|| {
            let iteration = writer_iteration.fetch_add(1, Ordering::Relaxed);
            let (summary, retries) = rt.block_on(run_parallel_writers(&store.pool, iteration));
            eprintln!("[exo-dag-db concurrency] parallel_writers serialization_retries={retries}");
            summary.report("parallel_writers");
        });
    });

    // Readers: seed a fixed iteration once, then read its chains concurrently.
    const READER_ITERATION: u64 = 1_000_000;
    rt.block_on(async {
        let _ = run_parallel_writers(&store.pool, READER_ITERATION).await;
    });
    group.bench_function("parallel_readers", |b| {
        b.iter(|| {
            let summary = rt.block_on(run_parallel_readers(&store.pool, READER_ITERATION));
            summary.report("parallel_readers");
        });
    });
    group.finish();

    let rss_after = rss_kib();
    if let (Some(before), Some(after)) = (rss_before, rss_after) {
        eprintln!(
            "[exo-dag-db concurrency] rss_kib before={before} after={after} delta={}",
            i64::try_from(after).unwrap_or(i64::MAX) - i64::try_from(before).unwrap_or(0)
        );
    }

    rt.block_on(store.cleanup());
}

criterion_group!(benches, bench_concurrency);
criterion_main!(benches);
