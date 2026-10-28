//! Runtime values for the two Aspect of Harmony pools.
use bevy::prelude::Component;

#[derive(Component, Default)]
pub struct HarmonyPools {
    pub accumulator: f32,
    pub spender: f32,
}
