//! SimC76f8118 sc_mage.cpp:3547–3631, 5017–5028.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, RemoveReason, procs::AuraEvent, scripts::AuraScripts};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    queued_triggers::{self, DestinationTrigger},
    script_registry::SpellScriptHook,
    targets,
};
use combat::spell_info::{SpellInfo, SpellStore};

const TALENT: u32 = 1247777;
const BUFF: u32 = 1247778;
const RAY: u32 = 205021;
const COMET: u32 = 153595;
const PROJECTILE: u32 = 153596;
const TRAVEL_SOURCE: u32 = 228601;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::ActionAvailable(action_available),
    SpellScriptHook::SuccessfulCastRelationships(after_cast),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(TALENT).or_default().check_proc = Some(block_generated_proc);
}
fn block_generated_proc(_: &World, _: &AuraInstance, _: &AuraEvent) -> bool {
    false
}

fn action_available(world: &World, caster: Entity, spell: u32) -> bool {
    spell != RAY || !auras::has_aura(world, caster, BUFF)
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    match spell.id {
        RAY if world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(TALENT)) =>
        {
            spell_cast::cast_triggered(world, caster, BUFF, Some(caster), 1);
        }
        COMET if auras::has_aura(world, caster, BUFF) => {
            auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
                aura.spell_id == BUFF && aura.caster == caster
            });
            schedule_comets(world, caster, spell);
        }
        _ => {}
    }
}

fn schedule_comets(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    let Some(position) = targets::position(world, target) else {
        return;
    };
    let delay = world
        .resource::<SpellStore>()
        .get(TRAVEL_SOURCE)
        .and_then(|source| source.misc.as_ref())
        .expect("Comet Storm native travel source")
        .speed;
    let initial_ms = (delay * 1000.0) as u32;
    let ctx = EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    for ordinal in 0..7 {
        queued_triggers::schedule_destination_trigger(
            world,
            &ctx,
            DestinationTrigger {
                target,
                destination: [position.x, position.y, position.z],
                spell_id: PROJECTILE,
                delay_ms: initial_ms + ordinal * 200,
            },
        );
    }
}
