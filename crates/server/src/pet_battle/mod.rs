//! Deterministic pet-battle rules, isolated from ECS, transport and persistence.
mod catalog;
mod stats;
pub use catalog::*;
pub use stats::*;
#[cfg(test)]
mod tests;
