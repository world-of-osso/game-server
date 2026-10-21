//! TC a352b1fa Orator area scripts and AreaTrigger.cpp:776–792,900–937,982–1003.
//! These scripts affect Players only, not hostile NPCs: they are not generic
//! enemy-radius hazards. Shapes come from the installed TDB1210 properties.
use std::collections::HashSet;

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::components::{Health, Player, Position, UnitFlags};

use crate::auras::{ActiveAuras, AuraInstance, RemoveReason};
use crate::maps::InMap;

const CHAINS: u32 = 434691;
const CHAINS_PERIODIC: u32 = 440310;
const LINGERING_DAMAGE: u32 = 434926;
const LINGERING_GROW_SECONDS: f64 = 6.0;
const LINGERING_SCALE_TARGET: f32 = 18.0;

#[derive(Clone, Copy)]
struct Disk {
    inner: f32,
    outer: f32,
    height: f32,
    z_offset: f32,
}

#[derive(Resource, Clone)]
pub(crate) struct Catalog {
    disk: Result<Disk, String>,
    sphere: Result<f32, String>,
}

fn validate_template(connection: &Connection, template: u32) -> Result<(), String> {
    connection
        .query_row(
            "SELECT 1 FROM tdb_areatrigger_template WHERE Id=?1 AND IsCustom=0 \
         AND Flags=0 AND ActionSetId=0 AND ActionSetFlags=0 AND NOT EXISTS \
         (SELECT 1 FROM tdb_areatrigger_template_actions WHERE AreaTriggerId=?1 AND IsCustom=0)",
            [template],
            |_| Ok(()),
        )
        .map_err(|error| {
            format!("Orator area template{template}: unsupported source actions/flags: {error}")
        })
}

pub(crate) fn load_catalog(connection: &Connection) -> Catalog {
    let disk = validate_template(connection, 34738).and_then(|()| {
        connection.query_row(
            "SELECT ShapeData0,ShapeData2,ShapeData4,ShapeData6 FROM tdb_areatrigger_create_properties \
             WHERE Id=31997 AND IsCustom=0 AND AreaTriggerId=34738 AND IsAreatriggerCustom=0 \
             AND Flags=4 AND Shape=5 AND MoveCurveId=0 AND ScaleCurveId=0 AND MorphCurveId=0 \
             AND FacingCurveId=0 AND ShapeData0=ShapeData1 AND ShapeData2=ShapeData3 \
             AND ShapeData4=ShapeData5 AND ShapeData6=ShapeData7 \
             AND ScriptName='at_orator_krix_vizk_chains_of_oppression'",
            [], |row| Ok(Disk { inner: row.get(0)?, outer: row.get(1)?, height: row.get(2)?, z_offset: row.get(3)? }),
        ).map_err(|error| format!("Orator31997 fixed source disk: {error}"))
    });
    let sphere = validate_template(connection, 34764).and_then(|()| {
        connection
            .query_row(
                "SELECT ShapeData0 FROM tdb_areatrigger_create_properties \
             WHERE Id=32026 AND IsCustom=0 AND AreaTriggerId=34764 AND IsAreatriggerCustom=0 \
             AND Flags=2 AND Shape=0 AND ShapeData0=ShapeData1 AND MoveCurveId=0 \
             AND ScaleCurveId=0 AND MorphCurveId=0 AND FacingCurveId=0 \
             AND ScriptName='at_orator_krix_vizk_lingering_influence'",
                [],
                |row| row.get(0),
            )
            .map_err(|error| format!("Orator32026 source sphere: {error}"))
    });
    Catalog { disk, sphere }
}

pub(in crate::scripted_ai) fn startup_load_catalog(
    mut commands: Commands,
    injected: Option<Res<Catalog>>,
) {
    if injected.is_some() {
        return;
    }
    let connection = Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open Orator source area database read-only");
    commands.insert_resource(load_catalog(&connection));
}

#[derive(Clone, Copy)]
enum Shape {
    Chains {
        target: Entity,
        aura_instance: u32,
        disk: Disk,
    },
    Lingering {
        radius: f32,
        created_at: f64,
    },
}

#[derive(Component, Clone)]
struct OratorArea {
    caster: Entity,
    shape: Shape,
    expires_at: Option<f64>,
    inside: HashSet<Entity>,
}

impl OratorArea {
    fn affected_spell(&self) -> u32 {
        match self.shape {
            Shape::Chains { .. } => CHAINS_PERIODIC,
            Shape::Lingering { .. } => LINGERING_DAMAGE,
        }
    }
}

pub(crate) fn create_chains(
    world: &mut World,
    target: Entity,
    aura: &AuraInstance,
) -> Result<(), String> {
    let disk = world.resource::<Catalog>().disk.clone()?;
    let map = *world
        .get::<InMap>(target)
        .ok_or("Chains bearer has no map")?;
    let position = *world
        .get::<Position>(target)
        .ok_or("Chains bearer has no position")?;
    world.spawn((
        OratorArea {
            caster: aura.caster,
            shape: Shape::Chains {
                target,
                aura_instance: aura.instance_id,
                disk,
            },
            expires_at: aura.expires_at,
            inside: HashSet::new(),
        },
        position,
        map,
    ));
    Ok(())
}

pub(crate) fn create_lingering(
    world: &mut World,
    caster: Entity,
    position: Vec3,
    duration_ms: i32,
) -> Result<(), String> {
    let radius = world.resource::<Catalog>().sphere.clone()?;
    let map = *world
        .get::<InMap>(caster)
        .ok_or("Lingering caster has no map")?;
    let now = crate::auras::aura_now(world);
    let area = OratorArea {
        caster,
        shape: Shape::Lingering {
            radius,
            created_at: now,
        },
        expires_at: (duration_ms >= 0).then_some(now + f64::from(duration_ms) / 1000.0),
        inside: HashSet::new(),
    };
    let entity = world
        .spawn((
            area,
            Position {
                x: position.x,
                y: position.y,
                z: position.z,
            },
            map,
        ))
        .id();
    update_one(world, entity);
    Ok(())
}

pub(crate) fn remove_chains(world: &mut World, target: Entity, aura: &AuraInstance) {
    let fields: Vec<_> = world
        .query::<(Entity, &OratorArea)>()
        .iter(world)
        .filter(|(_, field)| {
            matches!(field.shape, Shape::Chains { target: bearer, aura_instance, .. }
            if bearer == target && aura_instance == aura.instance_id)
        })
        .map(|(entity, field)| (entity, field.clone()))
        .collect();
    for (entity, field) in fields {
        retire(world, entity, &field);
    }
}

pub(in crate::scripted_ai) fn update(world: &mut World) {
    let fields: Vec<_> = world
        .query_filtered::<Entity, With<OratorArea>>()
        .iter(world)
        .collect();
    for field in fields {
        update_one(world, field);
    }
}

fn live_position(world: &World, entity: Entity, field: &OratorArea, now: f64) -> Option<Position> {
    let map = world.get::<InMap>(entity)?;
    if field.expires_at.is_some_and(|expiry| now >= expiry)
        || world.get::<InMap>(field.caster) != Some(map)
    {
        return None;
    }
    match field.shape {
        Shape::Chains {
            target,
            aura_instance,
            ..
        } => {
            let bound = world.get::<ActiveAuras>(target)?.get(aura_instance)?;
            if bound.spell_id != CHAINS || world.get::<InMap>(target) != Some(map) {
                return None;
            }
            world.get::<Position>(target).copied()
        }
        Shape::Lingering { .. } => world.get::<Position>(entity).copied(),
    }
}

fn contains(shape: Shape, offset: Vec3, reach: f32, now: f64) -> bool {
    match shape {
        Shape::Chains { disk, .. } => {
            let horizontal_squared = offset.x * offset.x + offset.z * offset.z;
            let in_height = offset.y >= disk.z_offset && offset.y <= disk.z_offset + disk.height;
            let outside_inner = horizontal_squared > disk.inner * disk.inner;
            let inside_outer = horizontal_squared <= disk.outer * disk.outer;
            in_height && outside_inner && inside_outer
        }
        Shape::Lingering { radius, created_at } => {
            let progress = ((now - created_at) / LINGERING_GROW_SECONDS).clamp(0.0, 1.0) as f32;
            let scale = 1.0 + (LINGERING_SCALE_TARGET - 1.0) * progress;
            let radius = radius * scale + reach;
            offset.length_squared() < radius * radius
        }
    }
}

fn collect_members(
    world: &mut World,
    entity: Entity,
    field: &OratorArea,
    center: Position,
    now: f64,
) -> HashSet<Entity> {
    let map = *world.get::<InMap>(entity).expect("Orator area copy");
    let players: Vec<_> = world
        .query_filtered::<(Entity, &Position, &InMap, &Health, Option<&UnitFlags>), With<Player>>()
        .iter(world)
        .filter(|(_, _, copy, health, flags)| {
            **copy == map && health.current > 0.0 && flags.is_none_or(|flags| flags.is_selectable())
        })
        .map(|(unit, position, _, _, _)| (unit, *position))
        .collect();
    let origin = Vec3::new(center.x, center.y, center.z);
    players
        .into_iter()
        .filter(|&(unit, position)| {
            let alive = world
                .get::<crate::death::PlayerDeathState>(unit)
                .is_none_or(|state| state.state == shared::death::DeathState::Alive);
            let offset = Vec3::new(position.x, position.y, position.z) - origin;
            let reach = super::super::model_reach::combat_reach(world, unit)
                .expect("native Player combat reach");
            alive && contains(field.shape, offset, reach, now)
        })
        .map(|(unit, _)| unit)
        .collect()
}

fn update_one(world: &mut World, entity: Entity) {
    let field = world
        .get::<OratorArea>(entity)
        .expect("queried Orator area")
        .clone();
    let now = crate::auras::aura_now(world);
    let Some(position) = live_position(world, entity, &field, now) else {
        retire(world, entity, &field);
        return;
    };
    world.entity_mut(entity).insert(position);
    let inside = collect_members(world, entity, &field, position, now);
    world.get_mut::<OratorArea>(entity).unwrap().inside = inside.clone();
    for &unit in inside.difference(&field.inside) {
        super::cast_source_child(world, field.caster, unit, field.affected_spell());
    }
    for &unit in field.inside.difference(&inside) {
        remove_member_aura(world, unit, field.affected_spell());
    }
}

fn remove_member_aura(world: &mut World, unit: Entity, spell: u32) {
    if world.get_entity(unit).is_err() {
        return;
    }
    crate::auras::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == spell
    });
}

fn retire(world: &mut World, entity: Entity, field: &OratorArea) {
    for &unit in &field.inside {
        remove_member_aura(world, unit, field.affected_spell());
    }
    world.despawn(entity);
}
