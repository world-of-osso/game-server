//! Native70448 spell1219480s7/s10: six entry Flame Shocks, 50% Lava Burst.
//! SimC76f8118 sc_shaman.cpp8639–8668 orders Flame Shock before Lava Burst.
use bevy::prelude::*;
use shared::components::CombatStatus;

use super::combat;
use combat::auras::{SpellModOp, spell_mods};
use combat::spell_cast::script_registry::{HitHook, SpellScriptHook};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, Sight, targets};
use combat::spell_info::SpellStore;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::Hit(hit_hook)];

fn hit_hook(spell: u32) -> Option<HitHook> {
    (spell == ASCENDANCE).then_some(cast_entry)
}

pub(super) const ASCENDANCE: u32 = 1219480;
const FLAME_SHOCK: u32 = 188389;
const LAVA_BURST_DAMAGE: u32 = 285452;

/// ELEMENTAL_ASCENDANCE_ENTRY_USES_FLAME_SHOCK_ELIGIBILITY is project policy,
/// not a Retail radius: in-combat hostiles, actual Shock validation, nearest-first.
fn select_flame_shock_eligible_enemies(
    world: &mut World,
    caster: Entity,
    cap: usize,
) -> Vec<Entity> {
    let shock = world
        .resource::<SpellStore>()
        .get(FLAME_SHOCK)
        .expect("native Ascendance requires Flame Shock metadata")
        .clone();
    let origin = targets::position(world, caster).expect("Ascendance caster retains position");
    let (_, maximum) = targets::spell_range(&shock, true);
    let maximum = spell_mods::modify(world, caster, &shock, SpellModOp::Range, maximum);
    let mut candidates =
        targets::area_units(world, caster, origin, maximum, targets::Relation::Enemy, 0);
    candidates.retain(|&unit| {
        world
            .get::<CombatStatus>(unit)
            .is_some_and(|status| status.0)
            && spell_cast::validate_target(world, caster, &shock, Some(unit), Sight::Start).is_ok()
            && spell_cast::aura_spell_restrictions::check_target(world, caster, Some(unit), &shock)
                .is_ok()
    });
    candidates.truncate(cap);
    candidates
}

fn cast_entry_lava_burst(
    world: &mut World,
    caster: Entity,
    target: Entity,
    depth: u8,
    multiplier: f32,
) {
    let Some(mut damage) = spell_cast::load_triggered_spell(world, LAVA_BURST_DAMAGE, depth) else {
        return;
    };
    // Only this native child cast is scaled; ordinary Lava Burst remains unchanged.
    damage.cast_damage_multiplier = Some(multiplier);
    spell_cast::launch_triggered_spell(world, caster, &damage, Some(target), depth);
}

pub(super) fn cast_entry(world: &mut World, caster: Entity, _self_target: Entity, depth: u8) {
    let cap = script_effect_value(world, caster, ASCENDANCE, 6) as usize;
    let multiplier = script_effect_value(world, caster, ASCENDANCE, 9) / 100.0;
    let units = select_flame_shock_eligible_enemies(world, caster, cap);
    for &unit in &units {
        spell_cast::cast_triggered(world, caster, FLAME_SHOCK, Some(unit), depth + 1);
    }
    for unit in units {
        cast_entry_lava_burst(world, caster, unit, depth + 1, multiplier);
    }
}
