//! Mainline 12.1.0.69933 named children and explicit tooltip fields.
use super::script_effect_value;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::effects::{CastEffectHandler, EffectCtx};
use crate::spell_cast::{self, PlannedEffect, targets};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::components::Health;
use std::collections::HashSet;

pub(super) const HEALING_RAIN: u32 = 73920;
const SIGIL_OF_FLAME: u32 = 204596;
const SIGIL_FLAME_DAMAGE: u32 = 204598;
pub(super) const RAIN_OF_FIRE: u32 = 5740;
const RAIN_OF_FIRE_DAMAGE: u32 = 42223;
pub(super) const CONSECRATION: u32 = 26573;
const CONSECRATION_DAMAGE: u32 = 81297;
pub(super) const EARTHQUAKE: u32 = 61882;
pub(super) const EMERALD_BLOSSOM: u32 = 355913;
pub(super) const WINTER_DAMAGE: u32 = 196771;
pub(super) const SCOURGE_STRIKE: u32 = 55090;
pub(super) const VOID_STEP: u32 = 1234796;
pub(super) const RADIANCE: u32 = 194509;
const RAIN_HEAL: u32 = 73921;
const QUAKE_DAMAGE: u32 = 77478;
const QUAKE_KNOCKDOWN: u32 = 77505;
const BLOSSOM_HEAL: u32 = 355916;
const WINTER_SLOW: u32 = 211793;
const VIRULENT_PLAGUE: u32 = 191587;
const ATONEMENT: u32 = 194384;

pub(in crate::spell_cast) fn cast_effect(spell_id: u32, index: u8) -> Option<CastEffectHandler> {
    match (spell_id, index) {
        (HEALING_RAIN, 0)
        | (EARTHQUAKE, 2)
        | (EMERALD_BLOSSOM, 0)
        | (RAIN_OF_FIRE, 1)
        | (SIGIL_OF_FLAME, 0) => Some(spawn_ground_pulses),
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

/// TC areatrigger_sha_earthquake::AddStunnedTarget: one set per actual field.
#[derive(Component, Default)]
struct EarthquakeStunnedTargets(HashSet<Entity>);

fn spawn_ground_pulses(
    world: &mut World,
    ctx: &EffectCtx,
    _effect: &crate::spell_info::SpellEffect,
) {
    let center = ctx.dest.expect("validated named ground spell destination");
    let pulses = read_ground_pulses(world, ctx);
    if matches!(ctx.spell.id, HEALING_RAIN | CONSECRATION) {
        replace_owned_field(world, ctx.caster, ctx.spell.id);
    }
    let map = *world
        .get::<crate::maps::InMap>(ctx.caster)
        .expect("ground spell caster map");
    let mut field = world.spawn((
        pulses,
        shared::components::Position {
            x: center.position.x,
            y: center.position.y,
            z: center.position.z,
        },
        map,
    ));
    if ctx.spell.id == EARTHQUAKE {
        field.insert(EarthquakeStunnedTargets::default());
    }
}

fn read_ground_pulses(world: &World, ctx: &EffectCtx) -> GroundPulses {
    let now = crate::auras::aura_now(world);
    let duration = f64::from(spell_cast::duration_ms(world, ctx.caster, ctx.spell)) / 1000.0;
    let (child, radius, cap, relation, period) = match ctx.spell.id {
        //5740 names42223m1*8; duration8000, period1000 from effect2.
        RAIN_OF_FIRE => (
            RAIN_OF_FIRE_DAMAGE,
            targets::effect_radius(&ctx.spell.effects[0]),
            0,
            targets::Relation::Enemy,
            f64::from(ctx.spell.effects[2].aura_period_ms) / 1000.0,
        ),
        CONSECRATION => (
            CONSECRATION_DAMAGE,
            targets::radius(
                &world
                    .resource::<SpellStore>()
                    .get(CONSECRATION_DAMAGE)
                    .expect("Consecration requires81297")
                    .effects[0],
                1,
            ),
            0,
            targets::Relation::Enemy,
            f64::from(ctx.spell.effects[0].aura_period_ms) / 1000.0,
        ),
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
        SIGIL_OF_FLAME => (
            SIGIL_FLAME_DAMAGE,
            targets::effect_radius(
                &world
                    .resource::<SpellStore>()
                    .get(SIGIL_FLAME_DAMAGE)
                    .expect("Sigil requires204598")
                    .effects[0],
            ),
            0,
            targets::Relation::Enemy,
            0.0,
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

///26573's local description variable names81297; period=t1, limit=s2=1.
///The field remains at the original caster position, not the moving caster.
pub(super) fn spawn_consecration(
    world: &mut World,
    caster: Entity,
    spell: &crate::spell_info::SpellInfo,
) {
    let position = targets::position(world, caster).expect("Consecration caster position");
    let ctx = EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: Some(targets::Dest {
            position,
            orientation: 0.0,
        }),
        chain_multiplier: 1.0,
    };
    spawn_ground_pulses(world, &ctx, &spell.effects[0]);
}

///53595's condition is positional and owner-specific, not just an aura flag.
pub(super) fn inside_owned_consecration(world: &mut World, caster: Entity) -> bool {
    let Some(position) = targets::position(world, caster) else {
        return false;
    };
    let now = crate::auras::aura_now(world);
    world
        .query::<(
            &GroundPulses,
            &shared::components::Position,
            &crate::maps::InMap,
        )>()
        .iter(world)
        .any(|(field, anchor, map)| {
            field.caster == caster
                && field.root == CONSECRATION
                && field.expires_at > now
                && Some(map) == world.get::<crate::maps::InMap>(caster)
                && position.distance(Vec3::new(anchor.x, anchor.y, anchor.z)) <= field.radius
        })
}

/// Native field radius for policy callers; no substitute radius for other spells.
pub(crate) fn ground_pulse_radius(world: &World, caster: Entity, spell_id: u32) -> Option<f32> {
    if !matches!(
        spell_id,
        CONSECRATION | EARTHQUAKE | HEALING_RAIN | RAIN_OF_FIRE | SIGIL_OF_FLAME | EMERALD_BLOSSOM
    ) {
        return None;
    }
    let spell = world.resource::<SpellStore>().get(spell_id)?;
    let ctx = EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    Some(read_ground_pulses(world, &ctx).radius)
}

/// Owned fields keep their actual centers when their caster moves.
pub(crate) fn owned_ground_fields(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
) -> Vec<(Vec3, f32)> {
    let now = crate::auras::aura_now(world);
    let map = world.get::<crate::maps::InMap>(caster).copied();
    world
        .query::<(
            &GroundPulses,
            &shared::components::Position,
            &crate::maps::InMap,
        )>()
        .iter(world)
        .filter(|(field, _, field_map)| {
            field.caster == caster
                && field.root == spell_id
                && field.expires_at > now
                && Some(**field_map) == map
        })
        .map(|(field, center, _)| (Vec3::new(center.x, center.y, center.z), field.radius))
        .collect()
}

/// Active harmful bound fields affecting this unit, not friendly healing fields.
pub(crate) fn hostile_ground_fields(world: &mut World, unit: Entity) -> Vec<(Entity, Vec3, f32)> {
    let now = crate::auras::aura_now(world);
    let map = world.get::<crate::maps::InMap>(unit).copied();
    world
        .query::<(
            Entity,
            &GroundPulses,
            &shared::components::Position,
            &crate::maps::InMap,
        )>()
        .iter(world)
        .filter(|(_, field, _, field_map)| {
            field.relation == targets::Relation::Enemy
                && field.expires_at > now
                && Some(**field_map) == map
                && targets::relation_matches(world, field.caster, unit, targets::Relation::Enemy)
        })
        .map(|(entity, field, position, _)| {
            (
                entity,
                Vec3::new(position.x, position.y, position.z),
                field.radius,
            )
        })
        .collect()
}

fn replace_owned_field(world: &mut World, caster: Entity, root: u32) {
    let previous: Vec<_> = world
        .query::<(Entity, &GroundPulses)>()
        .iter(world)
        .filter(|(_, p)| p.caster == caster && p.root == root)
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
    let mut child = world
        .resource::<SpellStore>()
        .get(pulses.child)
        .expect("named ground spell child")
        .clone();
    // TC casts77478 with OriginalCaster=at->GetGUID(); preserve it through flight.
    if pulses.root == EARTHQUAKE {
        child.cast_original_caster = Some(entity);
    }
    //26573's tooltip explicitly multiplies the81297 total by1.05.
    let chain_multipliers = if pulses.root == CONSECRATION {
        units.iter().map(|&unit| (unit, 1.05)).collect()
    } else {
        Vec::new()
    };
    let mut plan = vec![PlannedEffect {
        index: 0,
        units,
        dest: Some(targets::Dest {
            position: center,
            orientation: 0.0,
        }),
        chain_multipliers,
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }];
    //204598 has one targeted native bleed plus a latent duplicate with target0.
    //Keep the field's fixed destination; do not spawn an invented179 actor.
    if pulses.root == SIGIL_OF_FLAME {
        plan.push(PlannedEffect {
            index: 1,
            units: plan[0].units.clone(),
            dest: plan[0].dest,
            chain_multipliers: Vec::new(),
            blocked_units: Vec::new(),
            prevent_hit_script: false,
        });
    }
    // Named damage/heal effect only; Earthquake reads E1 chance in its OnHit script.
    spell_cast::hit::remove_defended_targets(world, pulses.caster, &child, &mut plan);
    spell_cast::missiles::launch(world, pulses.caster, &child, plan, pulses.depth);
}

/// TC a352b1fa spell_shaman.cpp1015-1027: roll native E1, then find the original
/// field among this caster's Earthquakes; stun a target only once per field.
pub(in crate::spell_cast) fn cast_earthquake_knockdown(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
) {
    let chance = ctx
        .spell
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("native Earthquake tick chance E1")
        .base_points;
    if rand::random::<f32>() * 100.0 >= chance {
        return;
    }
    let Some(field) = ctx.spell.cast_original_caster else {
        return;
    };
    if mark_earthquake_stunned_target(world, ctx.caster, field, target) {
        spell_cast::cast_triggered(
            world,
            ctx.caster,
            QUAKE_KNOCKDOWN,
            Some(target),
            ctx.depth + 1,
        );
    }
}

fn mark_earthquake_stunned_target(
    world: &mut World,
    caster: Entity,
    field: Entity,
    target: Entity,
) -> bool {
    let owned_earthquake = world
        .get::<GroundPulses>(field)
        .is_some_and(|pulses| pulses.root == EARTHQUAKE && pulses.caster == caster);
    if !owned_earthquake {
        return false;
    }
    world
        .get_mut::<EarthquakeStunnedTargets>(field)
        .expect("Earthquake field owns its stunned-target set")
        .0
        .insert(target)
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
