//! Pinned SimC76f8118 decisions absent from TC scripts/native trigger edges.
//! Native equipment, role expressions, proc masks/RPPM and child effects remain authoritative.
use crate::auras::{AuraInstance, procs::AuraEvent};
use crate::spell_cast::{self, TargetPlan};
use crate::spell_info::SpellInfo;
use bevy::prelude::*;

pub(crate) const GUILLOTINE: u32 = 1291728;
const GUILLOTINE_DAMAGE: u32 = 1306604;
const PERFECTED_GUILLOTINE: u32 = 1306624;
const PERFECTED_SET: u32 = 1291726;
const PROTECTION_FOUR_PIECE: u32 = 1296648;
const BLOODY_REBUKE: u32 = 1300690;
const FREE_REVENGE: u32 = 5302;

/// SimC dbc_proc_callback.cpp186–190: exclude the callback's own action, not unrelated procs.
pub(crate) fn check_guillotine_proc(
    _world: &World,
    aura: &AuraInstance,
    event: &AuraEvent,
) -> bool {
    event.actor == aura.caster
        && !matches!(
            event.spell_id,
            Some(GUILLOTINE_DAMAGE | PERFECTED_GUILLOTINE)
        )
}

/// GUILLOTINE_EQUIPPED_DRIVER: unique_gear_midnight.cpp6334–6378.
pub(crate) fn cast_guillotine(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    event: &AuraEvent,
) {
    let role = match crate::item_proc_data::role_multiplier(world, unit, GUILLOTINE) {
        Ok(role) => role,
        Err(error) => {
            warn!("Guillotine refused: {error}");
            return;
        }
    };
    let perfected = match crate::item_proc_data::has_equipped_bonus(world, unit, PERFECTED_SET) {
        Ok(perfected) => perfected,
        Err(error) => {
            warn!("Guillotine refused: {error}");
            return;
        }
    };
    let mut amount = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native equipped Guillotine requires item-scaled effect0")
        .amount
        * role;
    let child = if perfected {
        amount *= super::script_effect_value(world, unit, PERFECTED_SET, 1) / 100.0;
        PERFECTED_GUILLOTINE
    } else {
        GUILLOTINE_DAMAGE
    };
    spell_cast::cast_triggered_with_overrides(
        world,
        unit,
        child,
        event.target,
        1,
        spell_cast::TriggeredSpellOverrides {
            effect0_amount: Some(amount),
            ..Default::default()
        },
    );
}

/// GUILLOTINE_TARGET_HEALTH_SNAPSHOT: unique_gear_midnight.cpp6357–6365;
/// action.cpp1860–1915 calculates each recipient's amount before travel.
/// Reuse the existing per-target scalar snapshot, preserving native chain multipliers.
pub(in crate::spell_cast) fn snapshot_guillotine_health(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &mut TargetPlan,
) {
    if !matches!(spell.id, GUILLOTINE_DAMAGE | PERFECTED_GUILLOTINE) {
        return;
    }
    let percent = super::script_effect_value(world, caster, GUILLOTINE, 1);
    for planned in plan
        .iter_mut()
        .filter(|planned| spell.effects[planned.index].index == 0)
    {
        planned.chain_multipliers = planned
            .units
            .iter()
            .map(|&target| {
                let health = world
                    .get::<shared::components::Health>(target)
                    .expect("admitted Guillotine recipient requires Health");
                assert!(
                    health.max > 0.0,
                    "admitted Guillotine recipient requires positive max health"
                );
                let missing = (100.0 * (1.0 - health.current / health.max)).clamp(0.0, 100.0);
                (
                    target,
                    planned.chain_multiplier(target) * (1.0 + percent / 100.0 * missing),
                )
            })
            .collect();
    }
}

/// BLOODY_REBUKE_FREE_REVENGE: sc_warrior.cpp6065–6066, before native5302 consumption.
pub(super) fn revenge_bloody_rebuke(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if crate::auras::has_aura(world, caster, FREE_REVENGE) {
        cast_bloody_rebuke(world, caster, target, depth);
    }
}

/// BLOODY_REBUKE_RAVAGER_TICK: sc_warrior.cpp5843–5844.
pub(super) fn ravager_bloody_rebuke(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    cast_bloody_rebuke(world, caster, target, depth);
}

fn cast_bloody_rebuke(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    match crate::item_proc_data::has_equipped_bonus(world, caster, PROTECTION_FOUR_PIECE) {
        Ok(true) => {
            spell_cast::cast_triggered(world, caster, BLOODY_REBUKE, Some(target), depth + 1)
        }
        Ok(false) => {}
        Err(error) => warn!("Bloody Rebuke refused: {error}"),
    }
}
