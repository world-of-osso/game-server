//! Blood Boiling Point's Heart Strike proc and delayed Blood Boil echo.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, RemoveReason};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::AfterForegroundSpecial(on_cast),
];

fn register(scripts: &mut combat::auras::scripts::AuraScripts) {
    scripts.0.entry(1265982).or_default().on_remove = Some(echo);
    scripts.0.entry(1265968).or_default().on_remove = Some(queue_echo);
}

fn on_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 250)
    {
        return;
    }
    if matches!(spell.id, 206930 | 433895) {
        roll_proc(world, caster);
    } else if spell.id == 50842 && auras::has_aura(world, caster, 1265968) {
        consume_boiling_point(world, caster);
    }
}

fn roll_proc(world: &mut World, caster: Entity) {
    if !world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(1265790))
    {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp11238–11276: 15% for the first
    // target and a second roll weighted by the remaining active targets.
    let extra = combat::combat_refs::active_opponents(world, caster)
        .count()
        .saturating_sub(1);
    let first = combat::random::random::<f32>() < 0.15;
    let additional = extra > 0 && combat::random::random::<f32>() < 0.15 * extra as f32;
    if first || additional {
        spell_cast::cast_triggered(world, caster, 1265968, Some(caster), 1);
    }
}

fn consume_boiling_point(world: &mut World, unit: Entity) {
    auras::remove_auras(world, unit, RemoveReason::Consumed, |aura| {
        aura.spell_id == 1265968 && aura.caster == unit
    });
}

#[derive(Component)]
struct ForcedEchoMultiplier(f32);

fn queue_echo(world: &mut World, unit: Entity, aura: &AuraInstance, reason: RemoveReason) {
    if unit != aura.caster || reason != RemoveReason::Consumed {
        return;
    }
    if auras::has_aura(world, unit, 1265982) {
        let amount = aura
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("Boiling Point damage modifier")
            .amount;
        world
            .entity_mut(unit)
            .insert(ForcedEchoMultiplier(1.0 + amount / 100.0));
    }
    auras::remove_auras(world, unit, RemoveReason::Consumed, |old| {
        old.spell_id == 1265982 && old.caster == unit
    });
    spell_cast::cast_triggered(world, unit, 1265982, Some(unit), 1);
}

fn echo(world: &mut World, unit: Entity, aura: &AuraInstance, reason: RemoveReason) {
    if unit != aura.caster || !matches!(reason, RemoveReason::Expired | RemoveReason::Consumed) {
        return;
    }
    let multiplier = world
        .entity_mut(unit)
        .take::<ForcedEchoMultiplier>()
        .map(|value| value.0);
    let Some(mut spell) = spell_cast::load_triggered_spell(world, 50842, 1) else {
        return;
    };
    spell.cast_damage_multiplier = multiplier;
    if !spell_cast::launch_triggered_spell(world, unit, &spell, None, 1) {
        return;
    }
    super::blood_rune_weapon::copy_weapon_actions(world, unit, &spell);
    // SimC76f8118 sc_death_knight.cpp9069–9079: an echo can consume
    // a new Boiling Point proc and schedule a subsequent echo.
    consume_boiling_point(world, unit);
}
