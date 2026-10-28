//! SimC76f8118 sc_evoker.cpp7197–7242/7345–7435: Dragonrage launches Pyre at selected enemies.
use bevy::prelude::*;

use super::combat;
use combat::random::seq::SliceRandom;
use combat::spell_cast::script_registry::{AreaTargetFilter, HitHook, SpellScriptHook};
use combat::spell_info::SpellInfo;

const DRAGONRAGE_PYRE: u32 = 375088;
const PYRE_IMPACT: u32 = 393568;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EffectHit(effect_hit),
    SpellScriptHook::AreaFilter(area_filter),
];

fn effect_hit(spell: u32, effect: u8) -> Option<HitHook> {
    (spell == DRAGONRAGE_PYRE && effect == 0).then_some(launch_pyre)
}

fn launch_pyre(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    combat::spell_cast::cast_triggered(world, caster, PYRE_IMPACT, Some(target), depth + 1);
}

fn area_filter(spell: u32, effect: u8) -> Option<AreaTargetFilter> {
    (spell == DRAGONRAGE_PYRE && effect == 0).then_some(limit_targets)
}

fn limit_targets(
    _world: &World,
    spell: &SpellInfo,
    _target: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    let count = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Dragonrage Pyre target count")
        .base_points
        .max(0.0) as usize;
    if units.len() > count {
        units.shuffle(&mut combat::random::thread_rng());
        units.truncate(count);
    }
}
