//! SimC76f8118 sc_druid.cpp13124–13144: Berserk stores excess combo points.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::{self, powers, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{ActiveSpec, PowerType};

const BERSERK: u32 = 106951;
const INCARNATION: u32 = 102543;
const OVERFLOW: u32 = 405189;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterComboOverflow(store_overflow),
    SpellScriptHook::AfterForegroundSpender(consume_overflow),
    SpellScriptHook::LateAuraRegistration(register),
];

fn register(scripts: &mut AuraScripts) {
    for spell in [BERSERK, INCARNATION] {
        scripts.0.entry(spell).or_default().on_remove = Some(remove_overflow);
    }
}

fn active(world: &World, unit: Entity) -> bool {
    world
        .get::<ActiveSpec>(unit)
        .is_some_and(|spec| spec.0 == 103)
        && (auras::has_aura(world, unit, BERSERK) || auras::has_aura(world, unit, INCARNATION))
}

pub(super) fn extra_max(world: &World, unit: Entity) -> f32 {
    if !active(world, unit) {
        return 0.0;
    }
    world
        .resource::<SpellStore>()
        .get(OVERFLOW)
        .expect("Overflowing Power spell")
        .aura_options
        .as_ref()
        .expect("Overflowing Power stacks")
        .cumulative_aura as f32
}

fn capacity(world: &World, unit: Entity) -> f32 {
    let max = extra_max(world, unit) as u32;
    let stacks = world
        .get::<ActiveAuras>(unit)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == OVERFLOW && aura.caster == unit)
        })
        .map_or(0, |aura| u32::from(aura.stacks));
    max.saturating_sub(stacks) as f32
}

fn store_overflow(world: &mut World, unit: Entity, overflow: f32) {
    let count = overflow.min(capacity(world, unit)) as u32;
    for _ in 0..count {
        spell_cast::cast_triggered(world, unit, OVERFLOW, Some(unit), 1);
    }
}

fn consume_overflow(world: &mut World, unit: Entity, spell: &SpellInfo, _: Option<Entity>) {
    if !active(world, unit)
        || !spell
            .power_costs
            .iter()
            .any(|cost| cost.power_type == PowerType::ComboPoints as i32)
    {
        return;
    }
    let stacks = world
        .get::<ActiveAuras>(unit)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == OVERFLOW && aura.caster == unit)
        })
        .map_or(0, |aura| aura.stacks);
    if stacks == 0 {
        return;
    }
    auras::remove_auras(world, unit, RemoveReason::Consumed, |aura| {
        aura.spell_id == OVERFLOW && aura.caster == unit
    });
    powers::change_power(world, unit, PowerType::ComboPoints, f32::from(stacks));
}

fn remove_overflow(world: &mut World, unit: Entity, _: &AuraInstance, _: RemoveReason) {
    auras::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == OVERFLOW && aura.caster == unit
    });
}
