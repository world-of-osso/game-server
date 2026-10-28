//! SimC76f8118 sc_monk.cpp7232–7243: a dodge consumes Elusive Brawler.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, RemoveReason, procs::AuraEvent, scripts::AuraScripts};
use combat::class_progression::TraitLoadout;
use combat::random::Rng;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};

const ELUSIVE_BRAWLER: u32 = 195630;
const ONE_WITH_THE_WIND: u32 = 454484;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::AvoidanceProcHitMask(avoidance_mask),
];

fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(ELUSIVE_BRAWLER).or_default();
    script.check_proc = Some(is_dodge);
    script.on_proc = Some(consume_on_dodge);
}

fn avoidance_mask(spell: u32) -> Option<u32> {
    (spell == ELUSIVE_BRAWLER).then_some(auras::procs::PROC_HIT_DODGE)
}

fn is_dodge(_: &World, _: &AuraInstance, event: &AuraEvent) -> bool {
    event.hit_mask & auras::procs::PROC_HIT_DODGE != 0
}

fn consume_on_dodge(world: &mut World, unit: Entity, _: &AuraInstance, _: Option<Entity>) {
    let selected = world.get::<TraitLoadout>(unit).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == ONE_WITH_THE_WIND)
    });
    let preserve = if selected {
        script_effect_value(world, unit, ONE_WITH_THE_WIND, 0) / 100.0
    } else {
        0.0
    };
    if !combat::random::thread_rng().gen_bool(f64::from(preserve.clamp(0.0, 1.0))) {
        auras::remove_auras(world, unit, RemoveReason::Consumed, |aura| {
            aura.spell_id == ELUSIVE_BRAWLER && aura.caster == unit
        });
    }
}
