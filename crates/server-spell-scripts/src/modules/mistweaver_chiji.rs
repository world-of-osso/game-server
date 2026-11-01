//! Invoke Chi-Ji's 70448 kick-triggered Gusts and Celestial Harmony's Chi Cocoons.
use super::combat;
use bevy::prelude::*;
use combat::random::{self as rand, seq::SliceRandom};
use combat::spell_cast::summon::SummonedCreature;
use combat::spell_cast::{
    self,
    script_registry::{AreaTargetFilter, SpellScriptHook},
    scripts::{AfterCastHook, script_effect_value},
    targets::{self, Relation},
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{Health, UnitIsPet};

const INVOKE: u32 = 325197;
const YULON: u32 = 322118;
const PRESENCE: u32 = 343818;
const GUST: u32 = 343819;
const CELESTIAL_HARMONY: u32 = 343655;
/// 343655 names 406139 for its count and radius; WCL logs the applied cocoon as 406220.
const COCOON_DATA: u32 = 406139;
const COCOON: u32 = 406220;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::AreaFilter(area_filter),
];

// Rushing Wind Kick 467307 replaces Rising Sun Kick and carries its class flag (mask_1 128).
fn after_cast(spell: u32) -> Option<AfterCastHook> {
    match spell {
        100784 | 107428 | 467307 | 101546 => Some(trigger_gusts),
        INVOKE | YULON => Some(apply_cocoons),
        _ => None,
    }
}

fn trigger_gusts(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, INVOKE) {
        return;
    }
    let radius = targets::radius(
        &world
            .resource::<SpellStore>()
            .get(PRESENCE)
            .expect("Chi-Ji presence missing")
            .effects[0],
        0,
    );
    let limit = script_effect_value(world, caster, PRESENCE, 2).max(0.0) as usize;
    let allies = select_allies(world, caster, radius, limit);
    let count = script_effect_value(world, caster, INVOKE, 1).max(0.0) as usize;
    let multiplier = script_effect_value(world, caster, INVOKE, 5) / 100.0;
    for _ in 0..count {
        for &target in &allies {
            let overrides = spell_cast::TriggeredSpellOverrides {
                healing_multiplier: Some(multiplier),
                ..Default::default()
            };
            if !spell_cast::cast_triggered_with_overrides(
                world,
                caster,
                GUST,
                Some(target),
                1,
                overrides,
            ) {
                warn!(?caster, ?target, "Chi-Ji Gust failed");
            }
        }
    }
}

/// WCL 6Lg4hCVtj12JNbXc fight 41: five 406220 cocoons follow every Chi-Ji summon.
/// 406220's own area (40 yd around the caster) selects them; `area_filter` caps the count.
fn apply_cocoons(world: &mut World, caster: Entity, _: &SpellInfo) {
    if combat::auras::has_aura(world, caster, CELESTIAL_HARMONY)
        && !spell_cast::try_cast_triggered(world, caster, COCOON, Some(caster), 1)
    {
        warn!(?caster, "Celestial Harmony Chi Cocoon failed");
    }
}

fn area_filter(spell: u32, effect: u8) -> Option<AreaTargetFilter> {
    (spell == COCOON && effect == 0).then_some(limit_cocoons)
}

/// 406220 has no MaxTargets; Celestial Harmony names 406139 s3 targets.
fn limit_cocoons(world: &World, _: &SpellInfo, caster: Option<Entity>, units: &mut Vec<Entity>) {
    let Some(caster) = caster else {
        units.clear();
        return;
    };
    let limit = script_effect_value(world, caster, COCOON_DATA, 2).max(0.0) as usize;
    prioritize_party(world, units, limit);
}

/// WCL TQXmJ6YVr71Pdycw fight 8: Gusts reach every party member, favour injured
/// ones, and never the effigy; neither monk- nor enemy-distance order fits.
fn select_allies(world: &mut World, caster: Entity, radius: f32, limit: usize) -> Vec<Entity> {
    let Some(center) = targets::position(world, caster) else {
        return Vec::new();
    };
    let mut allies = targets::area_units(world, caster, center, radius, Relation::Raid, 0);
    prioritize_party(world, &mut allies, limit);
    allies
}

/// Keep `limit` party members, injured first in random order, excluding summons and pets.
fn prioritize_party(world: &World, units: &mut Vec<Entity>, limit: usize) {
    // Followers carry UnitSummonedBy too; only spell-created creatures and pets are excluded.
    units.retain(|&unit| {
        world.get::<SummonedCreature>(unit).is_none()
            && !world.get::<UnitIsPet>(unit).is_some_and(|pet| pet.0)
    });
    units.shuffle(&mut rand::thread_rng());
    units.sort_by_key(|&unit| {
        world
            .get::<Health>(unit)
            .is_none_or(|health| health.current >= health.max)
    });
    units.truncate(limit);
}
