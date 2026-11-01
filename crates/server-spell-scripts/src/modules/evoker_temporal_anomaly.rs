//! Temporal Anomaly 373861 shields the caster and allies with 373862.
//! SimC76f8118 has no Temporal Anomaly, and 70448 client DB2 lacks its
//! AreaTriggerCreateProperties 25294/34997, so the vortex's path is not modeled:
//! its caster dummy (s2, five allies) shields the nearest living raid allies
//! within 373862's authored 100-yard range. Retail WCL hbJmL1xDC6qkrdcR fight 1
//! shields five allies in 59 of 95 casts, nearly all within 250 ms of the cast.
//! Absorption past s2 targets is reduced in raids; that reduction is not modeled.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::{HitHook, SpellScriptHook};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::targets::{self, Relation};
use combat::spell_info::SpellStore;

const ANOMALY: u32 = 373861;
const SHIELD: u32 = 373862;
const TARGET_COUNT_EFFECT: u8 = 1;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EffectHit(effect_hit)];

fn effect_hit(spell: u32, index: u8) -> Option<HitHook> {
    (spell == ANOMALY && index == TARGET_COUNT_EFFECT).then_some(shield_allies)
}

fn shield_allies(world: &mut World, caster: Entity, _: Entity, depth: u8) {
    let Some(center) = targets::position(world, caster) else {
        return;
    };
    let count = script_effect_value(world, caster, ANOMALY, TARGET_COUNT_EFFECT);
    let range = world
        .resource::<SpellStore>()
        .get(SHIELD)
        .map(|shield| targets::spell_range(shield, false).1)
        .expect("Temporal Anomaly requires its native shield 373862");
    for ally in targets::area_units(world, caster, center, range, Relation::Raid, count as u32) {
        combat::spell_cast::cast_triggered(world, caster, SHIELD, Some(ally), depth + 1);
    }
}
