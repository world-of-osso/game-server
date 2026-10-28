//! Healing Stream Totem's native pulse selects one injured-priority target.
use super::combat;
use bevy::prelude::*;
use combat::random::{self as rand, seq::SliceRandom};
use combat::spell_cast::script_registry::{AreaTargetFilter, SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::{Health, Player};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AreaFilter(filter)];

fn filter(spell: u32, effect: u8) -> Option<AreaTargetFilter> {
    (spell == 52042 && effect == 0).then_some(select_target)
}

// TrinityCore spell_shaman.cpp:1427–1437 uses SelectRandomInjuredTargets(..., 1, true).
fn select_target(world: &World, _: &SpellInfo, _: Option<Entity>, units: &mut Vec<Entity>) {
    let priority = |unit: &Entity| {
        let full = world.get::<Health>(*unit).is_none_or(|h| h.current >= h.max);
        (full, world.get::<Player>(*unit).is_none())
    };
    if let Some(best) = units.iter().map(priority).min() {
        units.retain(|unit| priority(unit) == best);
        units.shuffle(&mut rand::thread_rng());
        units.truncate(1);
    }
}
