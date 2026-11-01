//! Fatal Flourish earns Energy from eligible off-hand and Outlaw hits.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::melee::Hand;
use combat::random::Rng;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

const TALENT: u32 = 35551;
const ENERGIZE: u32 = 35546;
const PISTOL_SHOT: u32 = 185763;
const MAIN_GAUCHE: u32 = 86392;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(|spell| spell == TALENT),
    SpellScriptHook::AfterWhiteHandHit(after_white_hit),
    SpellScriptHook::ResolvedDamage(after_damage),
];

fn after_white_hit(world: &mut World, caster: Entity, hand: Hand, _: bool) {
    if hand != Hand::Off {
        return;
    }
    let speed = world
        .get::<combat::stats::CharacterStats>(caster)
        .and_then(|stats| stats.off_hand.as_ref())
        .expect("off-hand swing requires an off-hand weapon")
        .speed;
    roll_fatal_flourish(world, caster, speed / 2.6);
}

fn after_damage(world: &mut World, ctx: &EffectCtx, _: Entity, _: bool, damage: f32) {
    if damage > 0.0 && matches!(ctx.spell.id, PISTOL_SHOT | MAIN_GAUCHE) {
        roll_fatal_flourish(world, ctx.caster, 1.0);
    }
}

fn roll_fatal_flourish(world: &mut World, caster: Entity, multiplier: f32) {
    if !auras::has_aura(world, caster, TALENT) {
        return;
    }
    // SimC76f8118 sc_rogue.cpp2158–2160, 5076–5077, 5109–5110,
    // 7976–7989: off-hand white hits, Pistol Shot and Main Gauche roll
    // native effect1; weapon swings scale by weapon speed / 2.6 seconds.
    let chance = spell_cast::scripts::script_effect_value(world, caster, TALENT, 0) * multiplier;
    if combat::random::thread_rng().gen_range(0.0..100.0) < chance {
        spell_cast::cast_triggered(world, caster, ENERGIZE, Some(caster), 1);
    }
}
