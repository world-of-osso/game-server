//! Canonical graph types plus consumer-owned runtime capture.
pub use server::test_support::graph::*;

#[path = "corridor_inputs.rs"]
pub(super) mod corridor_inputs;
pub(super) mod extraction;
mod source;
mod topology;
