//! SimC76f8118 sc_druid.cpp:3277–3279, 11070–11076, 14191–14192.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, procs::AuraEvent, scripts::AuraScripts};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;

const TALENT: u32 = 1261564;
const BUFF: u32 = 1263382;
const STARSURGE: u32 = 78674;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::BeforeDamageSnapshot(snapshot_damage),
    SpellScriptHook::SuccessfulCastRelationships(after_cast),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(BUFF).or_default().check_proc = Some(block_generic_proc);
}
fn block_generic_proc(_: &World, _: &AuraInstance, _: &AuraEvent) -> bool {
    false
}

pub(super) fn on_eclipse(world: &mut World, caster: Entity) {
    if !world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(TALENT))
    {
        return;
    }
    let maximum = world
        .resource::<combat::spell_info::SpellStore>()
        .get(BUFF)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("Ascendant Stars native stacks")
        .cumulative_aura as u8;
    spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        BUFF,
        Some(caster),
        1,
        spell_cast::TriggeredSpellOverrides {
            aura_stacks: Some(maximum),
            ..Default::default()
        },
    );
}

fn snapshot_damage(world: &World, caster: Entity, spell: &mut SpellInfo) {
    if spell.id != STARSURGE || !auras::has_aura(world, caster, BUFF) {
        return;
    }
    let amount = script_effect_value(world, caster, BUFF, 0);
    spell.cast_damage_multiplier =
        Some(spell.cast_damage_multiplier.unwrap_or(1.0) * (1.0 + amount / 100.0));
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id == STARSURGE {
        super::cast_relationships::decrement(world, caster, BUFF);
    }
}
