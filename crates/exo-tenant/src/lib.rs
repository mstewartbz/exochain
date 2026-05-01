//! EXOCHAIN constitutional trust fabric — multi-tenant isolation, cold storage, sharding.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

pub mod cold;
pub mod cold_storage;
pub mod error;
pub mod shard;
pub mod sharding;
pub mod store;
pub mod tenant;
