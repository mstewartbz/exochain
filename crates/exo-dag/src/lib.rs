//! EXOCHAIN append-only DAG with BFT consensus and Merkle structures.
//!
//! This crate provides:
//! - An append-only directed acyclic graph (`dag`)
//! - BFT consensus over the DAG (`consensus`)
//! - Sparse Merkle Tree for authenticated key-value storage (`smt`)
//! - Merkle Mountain Range for append-only accumulation (`mmr`)
//! - Storage abstraction (`store`)

pub mod consensus;
pub mod dag;
pub mod error;
pub mod mmr;
pub mod smt;
pub mod store;
