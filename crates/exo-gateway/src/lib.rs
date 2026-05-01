//! EXOCHAIN constitutional trust fabric — HTTP gateway server with default-deny pattern.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

pub mod auth;
pub mod db;
pub mod error;
pub mod graphql;
pub mod handlers;
pub mod middleware;
pub mod rest;
pub mod routes;
pub mod server;
