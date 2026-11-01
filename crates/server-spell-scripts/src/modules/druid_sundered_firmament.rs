//! SimC76f8118 sc_druid.cpp:3210–3280, 11134–11143, 11849–11857.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, script_registry::SpellScriptHook, targets};

const TALENT: u32 = 394094;
const PARTING_SKIES: u32 = 395110;
const FIELD: u32 = 394106;
const ENERGIZE: u32 = 394108;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(PARTING_SKIES).or_default().on_remove = Some(on_parting_end);
}

pub(super) fn on_eclipse(world: &mut World, caster: Entity) {
    if !world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(TALENT))
    {
        return;
    }
    if auras::has_aura(world, caster, PARTING_SKIES) {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == PARTING_SKIES && aura.caster == caster
        });
    } else {
        spell_cast::cast_triggered(world, caster, PARTING_SKIES, Some(caster), 1);
    }
}

fn on_parting_end(world: &mut World, caster: Entity, aura: &AuraInstance, reason: RemoveReason) {
    if aura.caster != caster || !matches!(reason, RemoveReason::Consumed | RemoveReason::Expired) {
        return;
    }
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(caster)
        .and_then(|target| target.0)
    else {
        return;
    };
    let Some(position) = targets::position(world, target) else {
        return;
    };
    let mut field =
        spell_cast::load_triggered_spell(world, FIELD, 1).expect("Sundered Firmament field");
    field.cast_destination = Some(position.to_array());
    spell_cast::launch_triggered_spell_with_primary(world, caster, &field, None, Some(target), 1);
    let duration = field
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .expect("Sundered Firmament duration")
        .duration_ms as u32;
    if auras::has_aura(world, caster, ENERGIZE) {
        auras::extend_owned_duration(world, caster, caster, &[ENERGIZE], duration);
    } else {
        spell_cast::cast_triggered(world, caster, ENERGIZE, Some(caster), 1);
    }
}
