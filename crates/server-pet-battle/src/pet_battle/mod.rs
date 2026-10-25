//! Deterministic pet-battle rules, isolated from ECS, transport and persistence.
mod catalog;
mod effects;
mod engine;
mod model;
mod rules;
mod stats;
pub use catalog::*;
pub use model::*;
pub use rules::{supported_property, type_multiplier, validate_ability};
pub use stats::*;
#[cfg(test)]
mod tests;
