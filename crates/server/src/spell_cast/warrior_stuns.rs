//! TC a352b1fa spell_warrior.cpp1792–1856, OnEffectHitTarget(EFFECT_0,
//! SCHOOL_DAMAGE) and Storm Bolts' conditional destination-area target filter.
use bevy::prelude::*;

use super::effects::{EFFECT_SCHOOL_DAMAGE, EffectCtx};
use crate::spell_info::{SpellEffect, SpellInfo};

const SHOCKWAVE: u32 = 46968;
const SHOCKWAVE_STUN: u32 = 132168;
const STORM_BOLT: u32 = 107570;
const STORM_BOLT_STUN: u32 = 132169;
const STORM_BOLTS: u32 = 436162;
const TARGET_UNIT_DEST_AREA_ENEMY: u32 = 16;

/// These source hooks run before the default effect handler, not once per
/// whole spell: Shockwave also selects its caster through a separate effect.
pub(super) fn cast_hit_stun(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
) {
    if effect.index != 0 || effect.effect != EFFECT_SCHOOL_DAMAGE {
        return;
    }
    match ctx.spell.id {
        SHOCKWAVE => {
            // Only IGNORE_CAST_IN_PROGRESS | DONT_REPORT_CAST_ERROR, not FULL_MASK.
            // Ordinary child costs, aura checks and cooldowns remain in force.
            if let Err(failure) = super::creature::creature_cast_ignoring_in_progress(
                world,
                ctx.caster,
                SHOCKWAVE_STUN,
                Some(target),
            ) {
                debug!(?failure, "Shockwave's native stun child refused");
            }
        }
        STORM_BOLT => super::cast_triggered(
            world,
            ctx.caster,
            STORM_BOLT_STUN,
            Some(target),
            ctx.depth + 1,
        ),
        _ => {}
    }
}

/// Without Storm Bolts, the source area-selector hook retains only the
/// explicit target. With the caster-owned talent, native area selection stays.
pub(super) fn filter_storm_bolt_targets(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    explicit: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    let source_selector = spell.id == STORM_BOLT
        && effect.index == 0
        && effect
            .implicit_targets
            .contains(&TARGET_UNIT_DEST_AREA_ENEMY);
    if source_selector && !crate::auras::has_aura(world, caster, STORM_BOLTS) {
        units.clear();
        units.extend(explicit);
    }
}
