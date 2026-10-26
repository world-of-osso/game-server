//! Native Tactician rolls once for a real foreground Rage payment.
use bevy::prelude::*;
use shared::components::{ActiveSpec, PowerType};

use super::combat;
use combat::spell_cast::CostPool;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterSpellPayment(roll_tactician),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
];

const TACTICIAN: u32 = 184783;
const BRUTE_FORCE: u32 = 1261046;
const OPPORTUNIST: u32 = 444774;
const OPPORTUNIST_BUFF: u32 = 456120;
const OVERPOWER: u32 = 7384;

fn owns_proc(spell: u32) -> bool {
    spell == TACTICIAN
}

fn roll_tactician(world: &mut World, caster: Entity, spell: &SpellInfo, costs: &[(CostPool, f32)]) {
    // SimC76f8118 sc_warrior.cpp1458–1471: positive paid foreground Rage,
    // native30% plus selected Brute Force's Slam/Heroic Strike bonus.
    if world.get::<ActiveSpec>(caster).is_none_or(|spec| spec.0 != 71)
        || combat::spell_cast::spent(costs, PowerType::Rage) <= 0.0
        || !combat::auras::has_aura(world, caster, TACTICIAN)
    {
        return;
    }
    let chance = read_tactician_chance(world, caster, spell.id);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    restore_overpower_charge(world, caster);
    if combat::auras::has_aura(world, caster, OPPORTUNIST)
        && !combat::spell_cast::try_cast_triggered(world, caster, OPPORTUNIST_BUFF, Some(caster), 1)
    {
        warn!(?caster, "Native Tactician Opportunist payload refused");
    }
}

fn read_tactician_chance(world: &World, caster: Entity, action: u32) -> f32 {
    let base = script_effect_value(world, caster, TACTICIAN, 0);
    let brute_force = matches!(action, 1464 | 1269383) && combat::auras::has_aura(world, caster, BRUTE_FORCE);
    let bonus = if brute_force {
        script_effect_value(world, caster, BRUTE_FORCE, 0)
    } else {
        0.0
    };
    base + bonus
}

fn restore_overpower_charge(world: &mut World, caster: Entity) {
    let action = combat::auras::overrides::resolve_spell(world, caster, OVERPOWER);
    let spell = world.resource::<SpellStore>().get(action).expect("native Overpower action");
    let category = spell.categories.as_ref().expect("native Overpower charge category").charge_category;
    combat::spell_cast::restore_charges(world, caster, category, 1);
}
