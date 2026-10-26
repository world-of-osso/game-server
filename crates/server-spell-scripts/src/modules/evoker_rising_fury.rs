//! SIMC_DEVASTATION_RISING_FURY: pinned12.1.5 sc_evoker.cpp10571–10633.
//! Native1271783 owns stack cap, periodic cadence and per-stack haste.
use bevy::prelude::*;

use super::combat;
use combat::auras::scripts::AuraScripts;
use combat::auras::{ActiveAuras, AuraInstance, RemoveReason, has_aura};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::{AfterCastHook, script_effect_value};
use combat::spell_info::{SpellInfo, SpellStore};

const DRAGONRAGE: u32 = 375087;
const RISING_FURY: u32 = 1271783;
const FIRST_RANK: u32 = 1271687;
const THIRD_RANK: u32 = 1271788;
const UNBOUND_FLAME: u32 = 1292323;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::MidAuraRegistration(register),
    SpellScriptHook::AfterResourceValues(cast_hook),
];

fn register(scripts: &mut AuraScripts) {
    let dragonrage = scripts.0.entry(DRAGONRAGE).or_default();
    dragonrage.on_apply = Some(begin_fury);
    dragonrage.on_remove = Some(finish_fury);
    scripts.0.entry(RISING_FURY).or_default().on_periodic = Some(grow_fury);
}

fn begin_fury(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster != unit || !has_aura(world, unit, FIRST_RANK) {
        return;
    }
    // sc_evoker.cpp10615–10619: starting Dragonrage unfreezes and earns one stack.
    if !combat::spell_cast::try_cast_triggered(world, unit, RISING_FURY, Some(unit), 1) {
        warn!(?unit, "Native Rising Fury grant refused");
    }
}

fn grow_fury(world: &mut World, unit: Entity, aura: &AuraInstance, effect: u8) {
    // buff.cpp194–212: each actual native periodic tick bumps an unfrozen stack.
    if effect != 3 || aura.caster != unit || !has_aura(world, unit, DRAGONRAGE) {
        return;
    }
    combat::auras::set_self_cast_stacks(world, unit, RISING_FURY, aura.stacks.saturating_add(1));
}

fn finish_fury(world: &mut World, unit: Entity, aura: &AuraInstance, _reason: RemoveReason) {
    if aura.caster != unit {
        return;
    }
    let stacks = world.get::<ActiveAuras>(unit).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == RISING_FURY && aura.caster == unit)
            .map(|aura| aura.stacks)
    });
    let Some(stacks) = stacks else {
        return;
    };
    if !has_aura(world, unit, THIRD_RANK) {
        combat::auras::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
            aura.spell_id == RISING_FURY && aura.caster == unit
        });
        return;
    }
    // sc_evoker.cpp10623–10627: retain earned stacks for native rank3 time per stack.
    let milliseconds = script_effect_value(world, unit, THIRD_RANK, 0) * f32::from(stacks);
    combat::auras::set_owned_duration(world, unit, unit, RISING_FURY, milliseconds as u32);
    grant_unbound_flame(world, unit);
}

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    (spell == 1292321).then_some(spend_unbound_flame)
}

fn spend_unbound_flame(world: &mut World, unit: Entity, _spell: &SpellInfo) {
    // SimC76f8118 sc_evoker.cpp7470–7475: the successful native cast spends one use.
    let instance = world.get::<ActiveAuras>(unit).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == UNBOUND_FLAME && aura.caster == unit)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        combat::auras::spend_stack(world, unit, instance);
    }
}

fn grant_unbound_flame(world: &mut World, unit: Entity) {
    let spell = world
        .resource::<SpellStore>()
        .get(UNBOUND_FLAME)
        .expect("native Unbound Flame window");
    // buff.cpp2075–2079: a reverse buff initially grants its native maximum stacks.
    let stacks = combat::auras::fresh_max_stacks(world, unit, spell);
    let values = combat::spell_cast::TriggeredSpellOverrides {
        aura_stacks: Some(stacks),
        ..Default::default()
    };
    if !combat::spell_cast::cast_triggered_with_overrides(
        world,
        unit,
        UNBOUND_FLAME,
        Some(unit),
        1,
        values,
    ) {
        warn!(?unit, "Native Unbound Flame window refused");
    }
}
