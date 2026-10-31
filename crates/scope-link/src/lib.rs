//! The internal scope link between the finder console and the scope node.
//!
//! Provides the `scope_node.v1` protocol, bearer-token interceptors for both
//! ends, heartbeat clock mapping, and a reusable in-memory tonic transport.
//! The console schedules heartbeats; clock mapping itself takes only supplied
//! timestamps and never reads a system clock.

// Generated code: tonic's service trait trips this lint on Rust 1.99+.
#[allow(clippy::double_must_use)]
pub mod proto {
    tonic::include_proto!("scope_node.v1");
}

pub mod auth;
pub mod clock;
pub mod memory;

pub use auth::{tokens_match, BearerToken, TokenInterceptor};
pub use clock::{ClockMap, Exchange};
