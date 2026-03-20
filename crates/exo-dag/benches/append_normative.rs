//! DAG append throughput benchmark.
//!
//! STATUS: Disabled — references types (`EventEnvelope`, `LedgerEvent`, etc.)
//! that were refactored in the exo-core API simplification. This benchmark
//! needs to be rewritten against the current `exo_dag::DagStore` API.
//!
//! To re-enable:
//! 1. Update imports to match current exo-core and exo-dag public APIs
//! 2. Add required dev-dependencies (criterion, tokio, ed25519-dalek, rand)
//! 3. Uncomment the [[bench]] section in Cargo.toml

fn main() {
    eprintln!("Benchmark disabled — needs rewrite against current API. See source for details.");
}
