//! Invoke Chi-Ji's 70448 kick-triggered Gusts.
use super::combat;
use bevy::prelude::*;
use combat::random::{self as rand, seq::SliceRandom};
use combat::spell_cast::{
    self,
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, script_effect_value},
    targets::{self, Relation},
};
use combat::spell_info::{SpellInfo, SpellStore};
use combat::spell_cast::summon::SummonedCreature;
use shared::components::{Health, UnitIsPet};

const INVOKE: u32 = 325197;
const PRESENCE: u32 = 343818;
const GUST: u32 = 343819;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::NativeAfterCast(after_cast)];

// Rushing Wind Kick 467307 replaces Rising Sun Kick and carries its class flag (mask_1 128).
fn after_cast(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 100784 | 107428 | 467307 | 101546).then_some(trigger_gusts)
}

fn trigger_gusts(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, INVOKE) {
        return;
    }
    let allies = select_allies(world, caster);
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

/// WCL TQXmJ6YVr71Pdycw fight 8: Gusts reach every party member, favour injured
/// ones, and never the effigy; neither monk- nor enemy-distance order fits.
fn select_allies(world: &mut World, caster: Entity) -> Vec<Entity> {
    let Some(center) = targets::position(world, caster) else {
        return Vec::new();
    };
    let presence = world
        .resource::<SpellStore>()
        .get(PRESENCE)
        .expect("Chi-Ji presence missing");
    let radius = targets::radius(&presence.effects[0], 0);
    let mut allies = targets::area_units(world, caster, center, radius, Relation::Raid, 0);
    // Followers carry UnitSummonedBy too; only spell-created creatures and pets are excluded.
    allies.retain(|&unit| {
        world.get::<SummonedCreature>(unit).is_none()
            && !world.get::<UnitIsPet>(unit).is_some_and(|pet| pet.0)
    });
    allies.shuffle(&mut rand::thread_rng());
    allies.sort_by_key(|&unit| {
        world
            .get::<Health>(unit)
            .is_none_or(|health| health.current >= health.max)
    });
    let limit = script_effect_value(world, caster, PRESENCE, 2).max(0.0) as usize;
    allies.truncate(limit);
    allies
}
