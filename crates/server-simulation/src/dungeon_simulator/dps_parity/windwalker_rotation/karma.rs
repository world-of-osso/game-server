//! SimC's modeled incoming-damage option belongs only to the player fixture.
use bevy::prelude::*;
use shared::components::{Health, Player};

use super::super::literal_rotation::execution::Prepared;
use crate::spell_cast::{self, SpellHistory, TriggeredSpellOverrides};
use crate::spell_info::SpellStore;
use crate::stats::CharacterStats;

const KARMA: u32 = 122470;
const REDIRECT: u32 = 124280;
// SimC76f8118 sc_monk.cpp2769–2790/2828–2847: default interval100s,
// default interval_stddev_opt0, and default modeled absorbed health fraction0.5.
const INTERVAL: f64 = 100.0;
const DEFAULT_HEALTH_FRACTION: f32 = 0.5;

pub(super) fn suppress_during_interval(world: &World, actor: Entity, prepared: &mut Prepared) {
    if world.get::<Player>(actor).is_none() {
        return;
    }
    let last = world
        .get::<SpellHistory>(actor)
        .and_then(|history| history.last_successful_cast_at(KARMA));
    if last.is_some_and(|at| spell_cast::now(world) - at < INTERVAL) {
        prepared.suppress_action("touch_of_karma");
    }
}

pub(super) fn apply_modeled_absorption(
    world: &mut World,
    actor: Entity,
    target: Entity,
) -> Result<(), String> {
    if world.get::<Player>(actor).is_none() {
        return Ok(());
    }
    let health = world
        .get::<Health>(actor)
        .ok_or("Karma player lacks health")?
        .max;
    let versatility = world
        .get::<CharacterStats>(actor)
        .ok_or("Karma player lacks stats")?
        .versatility_pct;
    let maximum = spell_cast::scripts::script_effect_value(world, actor, KARMA, 2) / 100.0;
    let redirect = spell_cast::scripts::script_effect_value(world, actor, KARMA, 3) / 100.0;
    let dot = world
        .resource::<SpellStore>()
        .get(REDIRECT)
        .ok_or("Karma redirect missing")?;
    let duration = dot
        .duration
        .as_ref()
        .ok_or("Karma redirect duration missing")?
        .duration_ms;
    let period = dot
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .ok_or("Karma redirect effect missing")?
        .aura_period_ms;
    if duration <= 0 || period == 0 || duration as u32 % period != 0 {
        return Err("Karma redirect has invalid native tick schedule".into());
    }
    let ticks = duration as f32 / period as f32;
    // SimC2834–2850/2735–2753: this is modeled absorbed damage, not
    // injected incoming damage; the residual DoT snapshots versatility only.
    let amount =
        health * DEFAULT_HEALTH_FRACTION.min(maximum) * redirect * (1.0 + versatility / 100.0)
            / ticks;
    if !spell_cast::cast_triggered_with_overrides(
        world,
        actor,
        REDIRECT,
        Some(target),
        1,
        TriggeredSpellOverrides {
            effect0_amount: Some(amount),
            ..Default::default()
        },
    ) {
        return Err("modeled Karma redirect cast refused".into());
    }
    Ok(())
}
