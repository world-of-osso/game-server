//! Native main-pet Wild Hunt snapshots before Focus payment.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{CostPool, powers, script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;
use shared::components::PowerType;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::PowerCost(modify_cost),
    SpellScriptHook::CaptureConsumedDamageModifier(snapshot_damage),
    SpellScriptHook::ExpectedDamageMultiplier(expected_damage),
];

const WILD_HUNT: u32 = 62762;
const BASIC_ATTACKS: &[u32] = &[17253, 16827, 49966];

fn high_focus_basic(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    if !BASIC_ATTACKS.contains(&spell.id) { return false }
    let Some(pet) = world.get::<combat::hunter_pet::HunterPet>(caster) else { return false };
    if combat::hunter_pet::summoned_pet(world, pet.owner) != Some(caster) { return false }
    let threshold = script_effect_value(world, caster, WILD_HUNT, 2);
    powers::power_value(world, caster, PowerType::Focus)
        .is_some_and(|(focus, _)| focus > threshold)
}

fn modify_cost(world: &World, caster: Entity, spell: &SpellInfo, pool: CostPool, cost: f32) -> f32 {
    // SimC76f8118 sc_hunter.cpp3046–3048/3057–3059/3074–3079:
    // only the ordinary main-pet basic reads this live pre-payment condition.
    if pool != CostPool::Power(PowerType::Focus) || !high_focus_basic(world, caster, spell) {
        return cost;
    }
    cost * (1.0 + script_effect_value(world, caster, WILD_HUNT, 1) / 100.0)
}

fn snapshot_damage(world: &World, caster: Entity, spell: &mut SpellInfo, foreground: bool) -> Option<u32> {
    // SimC76f8118 sc_hunter.cpp3062–3071: snapshot the same condition
    // before the doubled payment drops Focus below the threshold. Background
    // basic_attack_base children do not use basic_attack_main's Wild Hunt.
    if foreground && high_focus_basic(world, caster, spell) {
        let multiplier = 1.0 + script_effect_value(world, caster, WILD_HUNT, 0) / 100.0;
        spell.cast_damage_multiplier = Some(spell.cast_damage_multiplier.unwrap_or(1.0) * multiplier);
    }
    None
}

fn expected_damage(world: &World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    if high_focus_basic(world, caster, spell) {
        1.0 + script_effect_value(world, caster, WILD_HUNT, 0) / 100.0
    } else {
        1.0
    }
}
