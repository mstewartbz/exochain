use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ed25519_dalek::SigningKey;
use exo_core::{
    compute_event_id, compute_signature, EventEnvelope, EventPayload, HybridLogicalClock,
    LedgerEvent,
};
use exo_dag::append::append_event;
use exo_dag::store::MemoryStore;
use rand::rngs::OsRng;

fn bench_append(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = MemoryStore::new();

    // Pre-compute a valid event to avoid measuring crypto generation time in the append loop
    // (We want to benchmark the append logic: validation + store insert)
    // However, append requires unique events (hash check), so we might need a mix.
    // For MVP bench, we'll benchmark the full flow including ID computation as it's part of throughput.

    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    c.bench_function("append_event_throughput", |b| {
        b.to_async(&rt).iter(|| async {
            let envelope = EventEnvelope {
                parents: vec![],
                logical_time: HybridLogicalClock {
                    physical_ms: 1000,
                    logical: 0,
                },
                author: "did:exo:bench".to_string(),
                key_version: 1,
                payload: EventPayload::Opaque(vec![1, 2, 3]),
            };

            let event_id = compute_event_id(&envelope).unwrap();
            let sig = compute_signature(&signing_key, &event_id);

            let event = LedgerEvent {
                envelope,
                event_id,
                signature: sig,
            };

            black_box(append_event(&store, event).await.unwrap());
        })
    });
}

criterion_group!(benches, bench_append);
criterion_main!(benches);
