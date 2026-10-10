//! Mainline 12.1.0.69933 named children and explicit tooltip fields.
use super::script_effect_value;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::effects::{CastEffectHandler, EffectCtx};
use crate::spell_cast::{self, PlannedEffect, targets};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::components::Health;

pub(super) const HEALING_RAIN: u32 = 73920;
pub(super) const EARTHQUAKE: u32 = 61882;
pub(super) const EMERALD_BLOSSOM: u32 = 355913;
pub(super) const WINTER_DAMAGE: u32 = 196771;
pub(super) const SCOURGE_STRIKE: u32 = 55090;
pub(super) const VOID_STEP: u32 = 1234796;
pub(super) const RADIANCE: u32 = 194509;
const RAIN_HEAL: u32 = 73921;
const QUAKE_DAMAGE: u32 = 77478;
const BLOSSOM_HEAL: u32 = 355916;
const WINTER_SLOW: u32 = 211793;
const VIRULENT_PLAGUE: u32 = 191587;
const ATONEMENT: u32 = 194384;

pub(in crate::spell_cast) fn cast_effect(spell_id: u32, index: u8) -> Option<CastEffectHandler> {
    match (spell_id, index) {
        (HEALING_RAIN, 0) | (EARTHQUAKE, 2) | (EMERALD_BLOSSOM, 0) => Some(spawn_ground_pulses),
        (VOID_STEP, 0) => Some(teleport_void_step),
        _ => None,
    }
}

/// Unlike target-hit hooks these consume a destination once, including empty areas.
#[derive(Component, Clone)]
pub(super) struct GroundPulses {
    caster: Entity,
    root: u32,
    child: u32,
    radius: f32,
    cap: usize,
    relation: targets::Relation,
    expires_at: f64,
    next_tick: f64,
    period: f64,
    depth: u8,
}

fn spawn_ground_pulses(
    world: &mut World,
    ctx: &EffectCtx,
    _effect: &crate::spell_info::SpellEffect,
) {
    let center = ctx.dest.expect("validated named ground spell destination");
    let pulses = read_ground_pulses(world, ctx);
    if ctx.spell.id == HEALING_RAIN {
        replace_owned_rain(world, ctx.caster);
    }
    let map = *world
        .get::<crate::maps::InMap>(ctx.caster)
        .expect("ground spell caster map");
    world.spawn((
        pulses,
        shared::components::Position {
            x: center.position.x,
            y: center.position.y,
            z: center.position.z,
        },
        map,
    ));
}

fn read_ground_pulses(world: &World, ctx: &EffectCtx) -> GroundPulses {
    let now = crate::auras::aura_now(world);
    let duration = f64::from(spell_cast::duration_ms(world, ctx.caster, ctx.spell)) / 1000.0;
    let (child, radius, cap, relation, period) = match ctx.spell.id {
        HEALING_RAIN => (
            RAIN_HEAL,
            targets::effect_radius(&ctx.spell.effects[0]),
            script_effect_value(world, ctx.caster, HEALING_RAIN, 3) as usize,
            targets::Relation::Ally,
            f64::from(ctx.spell.effects[2].aura_period_ms) / 1000.0,
        ),
        EARTHQUAKE => (
            QUAKE_DAMAGE,
            targets::effect_radius(&ctx.spell.effects[0]),
            0,
            targets::Relation::Enemy,
            f64::from(ctx.spell.effects[1].aura_period_ms) / 1000.0,
        ),
        EMERALD_BLOSSOM => (
            BLOSSOM_HEAL,
            script_effect_value(world, ctx.caster, EMERALD_BLOSSOM, 0),
            script_effect_value(world, ctx.caster, BLOSSOM_HEAL, 1) as usize,
            targets::Relation::Ally,
            0.0,
        ),
        _ => unreachable!("only registered ground pulse spells"),
    };
    GroundPulses {
        caster: ctx.caster,
        root: ctx.spell.id,
        child,
        radius,
        cap,
        relation,
        expires_at: now + duration,
        next_tick: now + if period == 0.0 { duration } else { period },
        period,
        depth: ctx.depth + 1,
    }
}

fn replace_owned_rain(world: &mut World, caster: Entity) {
    let previous: Vec<_> = world
        .query::<(Entity, &GroundPulses)>()
        .iter(world)
        .filter(|(_, p)| p.caster == caster && p.root == HEALING_RAIN)
        .map(|(e, _)| e)
        .collect();
    for entity in previous {
        world.entity_mut(entity).despawn();
    }
}

pub(in crate::spell_cast) fn update_ground_pulses(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<GroundPulses>>()
        .iter(world)
        .collect();
    for entity in entities {
        update_ground_pulse(world, entity);
    }
}

fn update_ground_pulse(world: &mut World, entity: Entity) {
    let pulses = world
        .get::<GroundPulses>(entity)
        .expect("queried ground pulses")
        .clone();
    let now = crate::auras::aura_now(world);
    let same_map =
        world.get::<crate::maps::InMap>(pulses.caster) == world.get::<crate::maps::InMap>(entity);
    if !targets::is_alive(world, pulses.caster) || !same_map {
        world.entity_mut(entity).despawn();
        return;
    }
    // Include the final authored tick at expiry; never emit catch-up ticks after it.
    if now >= pulses.next_tick && pulses.next_tick <= pulses.expires_at {
        cast_ground_pulse(world, entity, &pulses);
        if pulses.period > 0.0 {
            world
                .get_mut::<GroundPulses>(entity)
                .expect("ground pulse still present")
                .next_tick += pulses.period;
        }
    }
    if now >= pulses.expires_at {
        world.entity_mut(entity).despawn();
    }
}

fn cast_ground_pulse(world: &mut World, entity: Entity, pulses: &GroundPulses) {
    let center = targets::position(world, entity).expect("ground pulse anchor");
    let mut units = targets::area_units(
        world,
        pulses.caster,
        center,
        pulses.radius,
        pulses.relation,
        0,
    );
    if pulses.cap > 0 {
        units.retain(|&unit| {
            world
                .get::<Health>(unit)
                .is_some_and(|health| health.current < health.max)
        });
        units.truncate(pulses.cap);
    }
    let child = world
        .resource::<SpellStore>()
        .get(pulses.child)
        .expect("named ground spell child")
        .clone();
    let mut plan = vec![PlannedEffect {
        index: 0,
        units,
        dest: Some(targets::Dest {
            position: center,
            orientation: 0.0,
        }),
        chain_multipliers: Vec::new(),
    }];
    // Named damage/heal effect only: Earthquake's knockdown77 remains unsupported.
    spell_cast::hit::remove_defended_targets(world, pulses.caster, &child, &mut plan);
    spell_cast::missiles::launch(world, pulses.caster, &child, plan, pulses.depth);
}

fn teleport_void_step(world: &mut World, ctx: &EffectCtx, effect: &crate::spell_info::SpellEffect) {
    spell_cast::effects::teleport_units(world, ctx, effect, ctx.caster);
}

///196770 names211793s1. Apply only to the enemy the native damage actually hit.
pub(super) fn slow_winter_hit(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    crate::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id: WINTER_SLOW,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
}

///55090 applies Virulent Plague if absent. Existing marks belong to the separate
///spread/eruption contract; they are not refreshed as a substitute for spreading.
pub(super) fn apply_missing_plague(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    let owned = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|a| a.spell_id == VIRULENT_PLAGUE && a.caster == caster)
    });
    if !owned {
        crate::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target,
                spell_id: VIRULENT_PLAGUE,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
    }
}

///194509s4 is60% of194384's normal duration, not a full-duration mark.
pub(super) fn apply_radiance_atonement(
    world: &mut World,
    caster: Entity,
    target: Entity,
    _depth: u8,
) {
    let mut mark = world
        .resource::<SpellStore>()
        .get(ATONEMENT)
        .expect("Radiance requires Atonement")
        .clone();
    let percent = script_effect_value(world, caster, RADIANCE, 3);
    let duration = mark
        .misc
        .as_mut()
        .and_then(|misc| misc.duration.as_mut())
        .expect("Atonement duration");
    duration.duration_ms = (duration.duration_ms as f32 * percent / 100.0).round() as i32;
    crate::auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id: ATONEMENT,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
        &mark,
    );
}
