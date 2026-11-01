//! SimC76f8118 sc_druid.cpp:33–34, 6890–7006, 8505–8510, 9055–9060.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, RemoveReason, procs::AuraEvent, scripts::AuraScripts};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;

const TALENT: u32 = 393760;
const BUFF: u32 = 393763;
const SOLAR: u32 = 48517;
const LUNAR: u32 = 48518;
const MASTERY: u32 = 393014;
const STARFIRE: u32 = 194153;
const WRATH: u32 = 190984;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::BeforeDamageSnapshot(snapshot_damage),
    SpellScriptHook::SuccessfulCastRelationships(after_cast),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(TALENT).or_default().check_proc = Some(block_generated_proc);
}

fn block_generated_proc(_: &World, _: &AuraInstance, _: &AuraEvent) -> bool {
    false
}

fn in_eclipse(world: &World, caster: Entity) -> bool {
    auras::has_aura(world, caster, SOLAR) || auras::has_aura(world, caster, LUNAR)
}

fn snapshot_damage(world: &World, caster: Entity, spell: &mut SpellInfo) {
    let (opposite, mastery_index) = match spell.id {
        STARFIRE => (SOLAR, 2),
        WRATH => (LUNAR, 0),
        _ => return,
    };
    if !auras::has_aura(world, caster, BUFF) || !in_eclipse(world, caster) {
        return;
    }
    let umbral = script_effect_value(world, caster, BUFF, 0);
    let opposite_eclipse = script_effect_value(world, caster, opposite, 0);
    let mastery = script_effect_value(world, caster, MASTERY, mastery_index);
    let multiplier =
        (1.0 + umbral / 100.0) * (1.0 + opposite_eclipse / 100.0) * (1.0 + mastery / 100.0);
    spell.cast_damage_multiplier = Some(spell.cast_damage_multiplier.unwrap_or(1.0) * multiplier);
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !matches!(spell.id, STARFIRE | WRATH)
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(TALENT))
    {
        return;
    }
    if in_eclipse(world, caster) {
        auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
            aura.spell_id == BUFF && aura.caster == caster
        });
    }
    if combat::random::random::<f32>() < 0.2 {
        spell_cast::cast_triggered(world, caster, BUFF, Some(caster), 1);
    }
}
