//! Rampaging Berserker changes Rampage while Recklessness is active.
use bevy::prelude::*;
use shared::components::{ActiveSpec, PowerType};

use super::combat;
use combat::spell_cast::{
    CostPool, script_registry::SpellScriptHook, scripts::script_effect_value,
};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::PowerCost(rampage_cost),
    SpellScriptHook::TargetDamageMultiplier(rampage_damage),
];

const TALENT: u32 = 1269309;
const RECKLESSNESS: u32 = 1719;
const RAMPAGE: u32 = 184367;
const RAMPAGE_HITS: &[u32] = &[
    184707, 184709, 201364, 201363, 1265579, 1265580, 1265581, 1265582, 1299944,
];

fn active(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 72)
        && combat::auras::has_aura(world, caster, TALENT)
        && combat::auras::has_aura(world, caster, RECKLESSNESS)
}

fn rampage_cost(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    pool: CostPool,
    cost: f32,
) -> f32 {
    // SimC76f8118 sc_warrior.cpp5756–5762: effect1 is signed raw Rage.
    if spell.id == RAMPAGE && pool == CostPool::Power(PowerType::Rage) && active(world, caster) {
        cost + script_effect_value(world, caster, TALENT, 0)
    } else {
        cost
    }
}

fn rampage_damage(world: &World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    // SimC76f8118 sc_warrior.cpp5528–5531: all Rampage child impacts use effect2.
    if RAMPAGE_HITS.contains(&spell.id) && active(world, caster) {
        1.0 + script_effect_value(world, caster, TALENT, 1) / 100.0
    } else {
        1.0
    }
}
