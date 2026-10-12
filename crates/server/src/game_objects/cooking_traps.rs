//! Source-limited linked cooking trap31442, not generic trap admission.
//! TC a352b1fa GameObject::Create/Update: unowned linked GO, environmental
//! player trigger, diameter/2 + combat reach, repeat charges0, and parent cleanup.
use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::{Connection, Row};
use shared::components::{Health, Player, Position, UnitFactionTemplate, UnitScale};
use shared::protocol::GameObjectInfo;

use crate::interest::NetworkVisibility;
use crate::maps::InMap;
use crate::spell_cast::{cast_triggered, now};

pub(super) const BASIC_CAMPFIRE_TRAP: u32 = 31442;
const COZY_FIRE: u32 = 7353;
const PLAYER_COMBAT_REACH: f32 = 1.5;
const DEFAULT_TRAP_COOLDOWN_SECONDS: f64 = 4.0;

#[derive(Clone, Debug)]
pub(super) struct TrapTemplate {
    info: GameObjectInfo,
    faction: u32,
    radius: f32,
    cooldown_seconds: f64,
}

/// Load the complete audited zero-charge environmental trap, including its addon.
/// A changed/absent row remains unavailable instead of dropping its gameplay.
pub(super) fn load_template(conn: &Connection) -> rusqlite::Result<Option<TrapTemplate>> {
    let fields = (0..=34)
        .map(|i| format!("t.Data{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT t.type,t.displayId,t.name,t.size,t.AIName,t.ScriptName,a.faction,a.flags,{fields}
        FROM tdb_gameobject_template t JOIN tdb_gameobject_template_addon a ON a.entry=t.entry
        WHERE t.entry={BASIC_CAMPFIRE_TRAP}"
    );
    let mut query = conn.prepare(&sql)?;
    let mut rows = query.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    if !matches_source_template(row)? {
        return Ok(None);
    }
    Ok(Some(TrapTemplate {
        info: GameObjectInfo {
            entry: BASIC_CAMPFIRE_TRAP,
            go_type: 6,
            display_id: 0,
            name: row.get(2)?,
            scale: row.get::<_, f64>(3)? as f32,
        },
        faction: row.get(6)?,
        radius: row.get::<_, u32>(10)? as f32 / 2.0,
        cooldown_seconds: DEFAULT_TRAP_COOLDOWN_SECONDS,
    }))
}

fn matches_source_template(row: &Row) -> rusqlite::Result<bool> {
    let data = (0..=34)
        .map(|i| row.get::<_, u32>(8 + i))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut expected = vec![0; 35];
    expected[2] = 10;
    expected[3] = COZY_FIRE;
    let invisible_trap = row.get::<_, u8>(0)? == 6 && row.get::<_, u32>(1)? == 0;
    let unscripted = row.get::<_, String>(4)?.is_empty() && row.get::<_, String>(5)?.is_empty();
    let friendly_addon = row.get::<_, u32>(6)? == 35 && row.get::<_, u32>(7)? == 0;
    Ok(invisible_trap && unscripted && friendly_addon && data == expected)
}

pub(super) fn spell_supported(world: &World) -> bool {
    crate::toy_catalog::unsupported_types(world, COZY_FIRE).is_empty()
}

#[derive(Component, Clone)]
struct LinkedCookingTrap {
    parent: Entity,
    template: TrapTemplate,
    ready_at: f64,
}

pub(super) fn spawn(world: &mut World, parent: Entity, template: TrapTemplate) {
    let position = *world
        .get::<Position>(parent)
        .expect("spawned focus position");
    let map = *world.get::<InMap>(parent).expect("spawned focus map");
    let ready_at = now(world);
    world.spawn((
        position,
        map,
        template.info.clone(),
        UnitFactionTemplate(template.faction),
        LinkedCookingTrap {
            parent,
            template,
            ready_at,
        },
        Replicate::to_clients(NetworkTarget::All),
        NetworkVisibility::default(),
    ));
}

/// Expire the parent before updating traps so an expired fire cannot refresh a buff.
pub(super) fn update(world: &mut World) {
    let time = now(world);
    let traps: Vec<_> = world
        .query::<(Entity, &LinkedCookingTrap)>()
        .iter(world)
        .map(|(entity, trap)| (entity, trap.clone()))
        .collect();
    for (entity, trap) in traps {
        if world.get_entity(trap.parent).is_err() {
            world.despawn(entity);
            continue;
        }
        if time < trap.ready_at {
            continue;
        }
        let Some(target) = find_trigger_player(world, entity, trap.template.radius) else {
            continue;
        };
        cast_triggered(world, entity, COZY_FIRE, Some(target), 0);
        world
            .get_mut::<LinkedCookingTrap>(entity)
            .expect("live trap")
            .ready_at = time + trap.template.cooldown_seconds;
    }
}

fn find_trigger_player(world: &mut World, trap: Entity, radius: f32) -> Option<Entity> {
    let position = *world.get::<Position>(trap)?;
    let map = *world.get::<InMap>(trap)?;
    let origin = Vec3::new(position.x, position.y, position.z);
    world
        .query_filtered::<(Entity, &Position, &InMap, &Health, &UnitScale), With<Player>>()
        .iter(world)
        .find(|(_, position, player_map, health, scale)| {
            let location = Vec3::new(position.x, position.y, position.z);
            let reach = PLAYER_COMBAT_REACH * scale.0;
            **player_map == map
                && health.current > 0.0
                && origin.distance_squared(location) < (radius + reach).powi(2)
        })
        .map(|(entity, ..)| entity)
}
