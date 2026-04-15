//! EXOCHAIN append-only DAG with BFT consensus and Merkle structures.
//!
//! This crate provides:
//! - An append-only directed acyclic graph (`dag`)
//! - Validated persistent-store append with Byzantine clock defense (`append`)
//! - Checkpoint finality aggregation (`checkpoint`)
//! - BFT consensus over the DAG (`consensus`)
//! - Sparse Merkle Tree for authenticated key-value storage (`smt`)
//! - Merkle Mountain Range for append-only accumulation (`mmr`)
//! - Storage abstraction (`store`)

pub mod append;
pub mod checkpoint;
pub mod consensus;
pub mod dag;
pub mod error;
pub mod mmr;
#[cfg(feature = "postgres")]
pub mod pg_store;
pub mod smt;
pub mod store;
