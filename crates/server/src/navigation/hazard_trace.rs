//! Opt-in JSON records on the existing debug subscriber; never changes navigation.
use bevy::prelude::*;
use serde_json::{Value, json};
use std::sync::OnceLock;
use std::time::Instant;

const TARGET: &str = "game_server::follower_hazard_trace";
static START: OnceLock<Instant> = OnceLock::new();

/// Payload construction and clock reads occur only when this debug target is enabled.
pub(crate) fn emit(
    event: &str,
    simulation_s: Option<f64>,
    actor: Entity,
    payload: impl FnOnce() -> Value,
) {
    if !bevy::log::tracing::enabled!(target: TARGET, bevy::log::tracing::Level::DEBUG) {
        return;
    }
    let monotonic_ns = START.get_or_init(Instant::now).elapsed().as_nanos();
    let record = json!({
        "schema": 1,
        "event": event,
        "simulation_s": simulation_s,
        "monotonic_ns": monotonic_ns,
        "actor": actor.to_bits(),
        "data": payload(),
    });
    debug!(target: TARGET, "FOLLOWER_HAZARD_TRACE {record}");
}
