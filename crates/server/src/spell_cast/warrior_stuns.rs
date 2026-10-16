//! Native Warrior effect scripts: Rumbling Earth cooldowns, per-hit stuns and
//! Storm Bolts' conditional destination-area target filter (TC a352b1fa).
use bevy::prelude::*;

use super::effects::{EFFECT_SCHOOL_DAMAGE, EffectCtx};
use crate::spell_info::{SpellEffect, SpellInfo};

const SHOCKWAVE: u32 = 46968;
const SHOCKWAVE_STUN: u32 = 132168;
const RUMBLING_EARTH: u32 = 275339;
const STORM_BOLT: u32 = 107570;
const STORM_BOLT_STUN: u32 = 132169;
const STORM_BOLTS: u32 = 436162;
const TARGET_UNIT_DEST_AREA_ENEMY: u32 = 16;

/// TC warrior1718–1750: the caster-targeted script effect runs once, after damage.
pub(super) fn is_rumbling_earth_effect(spell: &SpellInfo, effect: &SpellEffect) -> bool {
    spell.id == SHOCKWAVE
        && effect.index == 1
        && effect.effect == super::effects::EFFECT_SCRIPT_EFFECT
}

/// TC Spell.cpp2712 counts original successful effect0 targets, including kills.
/// The hit plan already removed misses and immune effect bits (hit.rs); freeze
/// its count before damage/stun dispatch instead of rechecking live target state.
pub(super) fn count_shockwave_targets(spell: &SpellInfo, plan: &super::TargetPlan) -> usize {
    if spell.id != SHOCKWAVE {
        return 0;
    }
    plan.iter()
        .find(|planned| spell.effects[planned.index].index == 0)
        .map_or(0, |planned| planned.units.len())
}

pub(super) fn reduce_shockwave_with_rumbling_earth(
    world: &mut World,
    caster: Entity,
    hit_count: usize,
) {
    let Some(aura) = world
        .get::<crate::auras::ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == RUMBLING_EARTH))
    else {
        return;
    };
    let fields = (
        aura.effects.iter().find(|effect| effect.index == 0),
        aura.effects.iter().find(|effect| effect.index == 1),
    );
    let (Some(minimum), Some(reduction)) = fields else {
        warn!("Rumbling Earth requires native aura effects0/1");
        return;
    };
    let minimum_targets = f64::from(minimum.amount.trunc());
    if (hit_count as f64) < minimum_targets {
        return;
    }
    let seconds = f64::from(reduction.amount);
    super::scripts::reduce_cooldown(world, caster, SHOCKWAVE, seconds);
}

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
