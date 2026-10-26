//! Main Gauche's native producers; payload coefficients remain catalog-owned.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    self, AuraInstance,
    procs::AuraEvent,
    scripts::{AuraScript, AuraScripts},
};
use combat::random::Rng;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

const MASTERY: u32 = 76806;
const MAIN_GAUCHE: u32 = 86392;
const BLADE_FLURRY: u32 = 13877;
// SimC76f8118 sc_rogue.cpp3615/3805/3936/4044/4726/5959/6000/6714/6816:
// Ambush, Dispatch, Flurry activation, Rush, Spree MH, Sinister root/extra,
// Unseen Blade and Coup's three damage children; never Pistol or BtE.
const SPECIALS: &[u32] = &[
    8676, 2098, 429951, 271881, 57841, 193315, 197834, 441144, 462140, 462239, 462240,
];

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::ResolvedDamage(after_special_hit),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        MASTERY,
        AuraScript {
            check_proc: Some(qualifies_white),
            on_proc: Some(launch_white_proc),
            ..Default::default()
        },
    );
}

fn qualifies_white(_: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    // sc_rogue.cpp3468–3469: only main-hand white swings; the native
    // proc system retains its authored chance and source/recipient lifetime.
    event.actor == aura.caster
        && event.spell_id.is_none()
        && event.damage > 0.0
        && event.actor_mask[0] & auras::procs::PROC_MAIN_HAND_WEAPON_SWING != 0
}

fn launch_white_proc(world: &mut World, caster: Entity, _: &AuraInstance, target: Option<Entity>) {
    if let Some(target) = target {
        spell_cast::cast_triggered(world, caster, MAIN_GAUCHE, Some(target), 1);
    }
}

fn after_special_hit(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool, damage: f32) {
    if damage <= 0.0
        || !SPECIALS.contains(&ctx.spell.id)
        || !auras::has_aura(world, ctx.caster, MASTERY)
    {
        return;
    }
    let chance = read_special_chance(world, ctx.caster);
    // sc_rogue.cpp7948–7973: successful qualifying impacts, including
    // secondary Sinister/Spree hands, independently roll the native mastery.
    if combat::random::thread_rng().gen_range(0.0..100.0) < chance {
        spell_cast::cast_triggered(world, ctx.caster, MAIN_GAUCHE, Some(target), ctx.depth + 1);
    }
}

fn read_special_chance(world: &World, caster: Entity) -> f32 {
    let spell = world
        .resource::<SpellStore>()
        .get(MASTERY)
        .expect("native Main Gauche mastery");
    let chance = spell
        .aura_options
        .as_ref()
        .expect("native mastery proc chance")
        .proc_chance as f32;
    let bonus = if auras::has_aura(world, caster, BLADE_FLURRY) {
        script_effect_value(world, caster, BLADE_FLURRY, 4)
    } else {
        0.0
    };
    chance + bonus
}
