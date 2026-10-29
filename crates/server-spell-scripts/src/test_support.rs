//! Native harness with the same downstream registry as the server.
pub use crate::combat::spell_cast::test_harness::*;
use crate::combat::spell_info::SpellInfo;
use bevy::prelude::{Entity, Resource};
use std::collections::HashMap;

/// Optional replay-only receipts: pre-mitigation fiend damage and raw/effective transfers by owner.
#[derive(Resource, Default)]
pub struct FiendAtonementReceipts(pub HashMap<Entity, (f64, f64, f64)>);
#[cfg(all(feature = "test-support", not(test)))]
pub use crate::modules::{ghoul_energy, set_ghoul_energy};

pub fn harness(extra: Vec<SpellInfo>, ids: &[u32], class: u8) -> Option<Harness> {
    harness_with(extra, ids, class, false)
}

pub fn harness_with_native_enemy_dodge_reduction(
    extra: Vec<SpellInfo>,
    ids: &[u32],
    class: u8,
) -> Option<Harness> {
    crate::combat::spell_cast::test_harness::harness_with_native_enemy_dodge_reduction_and_plugins(
        extra,
        ids,
        class,
        |app| {
            app.add_plugins(crate::SpellScriptsPlugin);
        },
    )
}

pub fn harness_with(
    extra: Vec<SpellInfo>,
    ids: &[u32],
    class: u8,
    computed_stats: bool,
) -> Option<Harness> {
    crate::combat::spell_cast::test_harness::harness_with_plugins(
        extra,
        ids,
        class,
        computed_stats,
        |app| {
            app.add_plugins(crate::SpellScriptsPlugin);
        },
    )
}
