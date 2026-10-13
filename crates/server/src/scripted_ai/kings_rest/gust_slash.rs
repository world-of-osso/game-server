//! TC a352b1fa `at_kings_rest_gust_slash` (AreaTrigger 17933, create properties
//! 13339) from Gust Slash 269931: a cylinder orbiting its creation point
//! (AreaTrigger.cpp InitOrbit/CalculateLocalOrbitPositionAndRotation) whose entering
//! players cast 269932 on themselves-as-caster (`unit->CastSpell(nullptr, .., true)`).
//! Native area-trigger visuals remain unconverted.
use std::collections::HashSet;
use std::f32::consts::PI;

use bevy::prelude::*;
use shared::components::{Health, Player, Position, UnitFlags};

use crate::maps::InMap;
use crate::terrain_height::{bevy_to_wow, wow_to_bevy};

pub(crate) const CREATE_PROPERTIES: i32 = 13339;
const DAMAGE: u32 = 269932;

/// Create properties 13339's actual cylinder and orbit rows.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub(crate) struct GustSlashShape {
    radius: f32,
    height: f32,
    location_z_offset: f32,
    /// `InitOrbit`'s TimeToTarget: whole seconds of one circle, in ms.
    time_to_target_ms: u32,
    orbit_radius: f32,
    blend_from_radius: f32,
    initial_angle: f32,
    orbit_z_offset: f32,
    counter_clockwise: bool,
    can_loop: bool,
}

/// Admit the actual TDB rows; curves, extra blend time and target shape changes
/// are not modelled, so rows using them are refused.
pub(crate) fn load_shape(connection: &rusqlite::Connection) -> rusqlite::Result<GustSlashShape> {
    let (radius, height, location_z_offset, speed) = connection.query_row(
        "SELECT ShapeData0, ShapeData2, ShapeData4, Speed FROM tdb_areatrigger_create_properties \
         WHERE Id=13339 AND IsCustom=0 AND AreaTriggerId=17933 \
         AND ScriptName='at_kings_rest_gust_slash' AND Shape=4 AND SpeedIsTime=0 \
         AND MoveCurveId=0 AND ScaleCurveId=0 AND MorphCurveId=0 AND FacingCurveId=0 \
         AND ShapeData0=ShapeData1 AND ShapeData2=ShapeData3 AND ShapeData4=ShapeData5",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get::<_, f32>(3)?)),
    )?;
    let shape = connection.query_row(
        "SELECT CircleRadius, BlendFromRadius, InitialAngle, ZOffset, CounterClockwise, CanLoop \
         FROM tdb_areatrigger_create_properties_orbit \
         WHERE AreaTriggerCreatePropertiesId=13339 AND IsCustom=0 AND ExtraTimeForBlending=0",
        [],
        |row| {
            let orbit_radius: f32 = row.get(0)?;
            Ok(GustSlashShape {
                radius,
                height,
                location_z_offset,
                time_to_target_ms: time_to_target_ms(orbit_radius, speed),
                orbit_radius,
                blend_from_radius: row.get(1)?,
                initial_angle: row.get(2)?,
                orbit_z_offset: row.get(3)?,
                counter_clockwise: row.get(4)?,
                can_loop: row.get(5)?,
            })
        },
    )?;
    if shape.time_to_target_ms == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(shape)
}

/// `InitOrbit`: the circle's duration truncates to whole seconds before scaling.
fn time_to_target_ms(orbit_radius: f32, speed: f32) -> u32 {
    let speed = if speed <= 0.0 { 1.0 } else { speed };
    (orbit_radius * 2.0 * PI / speed) as u32 * 1000
}

/// One live Gust Slash area trigger.
#[derive(Component, Debug)]
pub(crate) struct GustSlashTrigger {
    caster: Entity,
    /// WoW creation point and facing (`orbit.Center = pos`).
    pub(crate) center: [f32; 4],
    created_at: f64,
    expires_at: Option<f64>,
    inside: HashSet<Entity>,
}

/// `Spell::EffectCreateAreaTrigger` for create properties 13339.
pub(crate) fn create(
    world: &mut World,
    caster: Entity,
    dest: Vec3,
    orientation: f32,
    duration_ms: i32,
) {
    let Some(map) = world.get::<InMap>(caster).copied() else {
        return;
    };
    let now = crate::auras::aura_now(world);
    let (x, y, z) = bevy_to_wow(dest.x, dest.y, dest.z);
    let trigger = GustSlashTrigger {
        caster,
        center: [x, y, z, orientation],
        created_at: now,
        expires_at: (duration_ms >= 0).then_some(now + f64::from(duration_ms) / 1000.0),
        inside: HashSet::new(),
    };
    // Create computes the initial position; targets are first gathered on Update.
    let Some(&shape) = world.get_resource::<GustSlashShape>() else {
        error!("Gust Slash area13339 shape/orbit not loaded; area trigger not created");
        return;
    };
    let position = world_position(&shape, &trigger, 0);
    world.spawn((trigger, position, map));
}

/// `CalculateLocalOrbitPositionAndRotation` (local WoW offset from the center).
fn local_orbit_offset(shape: &GustSlashShape, elapsed_ms: u32) -> Vec3 {
    let mut movement_time = elapsed_ms;
    let time_to_target = shape.time_to_target_ms;
    let mut first_loop = true;
    if shape.can_loop && movement_time > time_to_target {
        movement_time %= time_to_target;
        first_loop = false;
    }
    let mut progress = movement_time as f32 / time_to_target as f32;
    if !shape.can_loop {
        progress = progress.min(1.0);
    }
    let mut radius = shape.orbit_radius;
    if first_loop && (shape.blend_from_radius - radius).abs() > f32::EPSILON {
        let blend_curve = ((shape.blend_from_radius - radius) / radius).clamp(1.0, 4.0);
        let blend_progress = (4.0 * progress / blend_curve).abs().clamp(0.0, 1.0);
        radius = shape.blend_from_radius + (radius - shape.blend_from_radius) * blend_progress;
    }
    let mut angle = 2.0 * PI * progress;
    if !shape.counter_clockwise {
        angle = -angle;
    }
    angle += shape.initial_angle;
    Vec3::new(
        radius * angle.cos(),
        radius * angle.sin(),
        shape.orbit_z_offset,
    )
}

/// `GetMovementOrigin().GetPositionWithOffset(local)` as a Bevy position.
fn world_position(shape: &GustSlashShape, trigger: &GustSlashTrigger, elapsed_ms: u32) -> Position {
    let local = local_orbit_offset(shape, elapsed_ms);
    let [cx, cy, cz, facing] = trigger.center;
    let (sin, cos) = facing.sin_cos();
    let x = cx + local.x * cos - local.y * sin;
    let y = cy + local.x * sin + local.y * cos;
    let (x, y, z) = wow_to_bevy(x, y, cz + local.z);
    Position { x, y, z }
}

/// `SearchUnitInCylinder`: G3D `Cylinder::contains` is distance to the axis
/// segment, so the ends are rounded (a capsule).
fn contains(shape: &GustSlashShape, trigger_at: &Position, unit_at: &Position) -> bool {
    let (ax, ay, az) = bevy_to_wow(trigger_at.x, trigger_at.y, trigger_at.z);
    let (ux, uy, uz) = bevy_to_wow(unit_at.x, unit_at.y, unit_at.z);
    let bottom = az + shape.location_z_offset;
    let along = (uz - bottom).clamp(0.0, shape.height);
    let closest = Vec3::new(ax, ay, bottom + along);
    closest.distance_squared(Vec3::new(ux, uy, uz)) <= shape.radius * shape.radius
}

/// `AreaTrigger::Update`: move, expire, then the target list's enter hooks.
pub(crate) fn update(world: &mut World) {
    let triggers: Vec<Entity> = world
        .query_filtered::<Entity, With<GustSlashTrigger>>()
        .iter(world)
        .collect();
    if triggers.is_empty() {
        return;
    }
    let now = crate::auras::aura_now(world);
    let shape = *world.resource::<GustSlashShape>();
    for entity in triggers {
        let trigger = world.get::<GustSlashTrigger>(entity).expect("queried");
        // Unit::RemoveFromWorld → RemoveAllAreaTriggers; death keeps them.
        let caster_gone = world.get_entity(trigger.caster).is_err();
        let expired = trigger.expires_at.is_some_and(|expiry| now >= expiry);
        let elapsed_ms = ((now - trigger.created_at) * 1000.0) as u32;
        let position = world_position(&shape, trigger, elapsed_ms);
        if caster_gone || expired {
            world.entity_mut(entity).despawn();
            continue;
        }
        world.entity_mut(entity).insert(position);
        let map = *world.get::<InMap>(entity).expect("trigger map");
        let inside = players_inside(world, &shape, &position, map);
        let mut trigger = world.get_mut::<GustSlashTrigger>(entity).expect("queried");
        let entering: Vec<Entity> = inside.difference(&trigger.inside).copied().collect();
        trigger.inside = inside;
        for player in entering {
            // OnUnitEnter: `unit->CastSpell(nullptr, SPELL_GUST_SLASH_DAMAGE, true)`.
            if let Err(failure) =
                crate::spell_cast::creature_cast(world, player, DAMAGE, None, true)
            {
                debug!("Gust Slash {DAMAGE} from entering {player}: {failure:?}");
            }
        }
    }
}

/// `UpdateTargetList` restricted to players, the only units the script acts on:
/// dead/ghost players (no AllowWhile* action-set flags) and uninteractible units
/// are excluded; AreaTrigger 17933 has no conditions.
fn players_inside(
    world: &mut World,
    shape: &GustSlashShape,
    at: &Position,
    map: InMap,
) -> HashSet<Entity> {
    world
        .query_filtered::<(Entity, &Position, &InMap, &Health, Option<&UnitFlags>), With<Player>>()
        .iter(world)
        .filter(|(_, position, player_map, health, flags)| {
            **player_map == map
                && health.current > 0.0
                && flags.is_none_or(|flags| flags.0 & UnitFlags::NOT_SELECTABLE == 0)
                && contains(shape, at, position)
        })
        .map(|(player, ..)| player)
        .collect()
}
