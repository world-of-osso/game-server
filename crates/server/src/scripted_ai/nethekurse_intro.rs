//! TC a352b1fa boss_nethekurse.cpp: sewer4347 and chamber-door182539 ingress.
use bevy::prelude::*;
use shared::components::{MovementControl, Npc, Player, Position};

use super::{CreatureAiState, Hook};
use crate::maps::{InMap, store::AreaTriggerShape};

pub(crate) const DOOR_SCRIPT: &str = "go_grand_warlock_chamber_door";

#[derive(Resource, Clone)]
pub(crate) struct SewerTrigger {
    center: Vec3,
    shape: AreaTriggerShape,
}

pub(crate) fn load_trigger(db: &rusqlite::Connection) -> rusqlite::Result<SewerTrigger> {
    db.query_row(
        "SELECT a.Pos_0,a.Pos_1,a.Pos_2,a.ShapeType,a.Radius,a.Box_length,a.Box_width,a.Box_height,a.Box_yaw \
         FROM area_trigger a JOIN tdb_areatrigger_scripts s ON s.entry=a.ID \
         WHERE a.ID=4347 AND a.ContinentID=540 AND s.ScriptName='at_shattered_halls_the_sewer'",
        [],
        |row| {
            let shape = AreaTriggerShape::from_db2(
                row.get(3)?, row.get(4)?, [row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?],
            ).ok_or(rusqlite::Error::InvalidQuery)?;
            Ok(SewerTrigger { center: Vec3::new(row.get(0)?,row.get(1)?,row.get(2)?), shape })
        },
    )
}

/// Both source handlers use instance->GetCreature(DATA_NETHEKURSE)->DoAction(0).
pub(crate) fn start_intro(world: &mut World, map: InMap) {
    let boss = world
        .query_filtered::<(Entity, &InMap, &Npc), With<CreatureAiState>>()
        .iter(world)
        .find_map(|(actor, copy, npc)| (*copy == map && npc.template_id == 16807).then_some(actor));
    if let Some(boss) = boss {
        super::dispatch(world, boss, Hook::DoAction(0));
    }
}

pub(crate) fn door_gossip_hello(world: &mut World, object: Entity) {
    if let Some(&map) = world.get::<InMap>(object) {
        start_intro(world, map);
    }
}

#[derive(Component)]
pub(crate) struct SewerPresence {
    map: InMap,
    inside: bool,
}

type Entrants<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static InMap,
        &'static Position,
        Option<&'static MovementControl>,
        Option<&'static SewerPresence>,
    ),
    (With<Player>, Changed<Position>),
>;

/// Ordinary entry edges; unlike Ragefire this source is NOT OnlyOnceAreaTrigger.
pub(crate) fn detect_entries(
    players: Entrants,
    trigger: Option<Res<SewerTrigger>>,
    mut commands: Commands,
) {
    let Some(trigger) = trigger else { return };
    for (player, &map, position, control, presence) in &players {
        if map.map_id != 540 || map.instance_id == 0 {
            commands.entity(player).remove::<SewerPresence>();
            continue;
        }
        let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
        let inside = trigger.shape.contains(trigger.center, Vec3::new(x, y, z));
        let before = presence
            .filter(|last| last.map == map)
            .map(|last| last.inside);
        commands
            .entity(player)
            .insert(SewerPresence { map, inside });
        if inside && before == Some(false) && !control.is_some_and(|control| control.controlled) {
            commands.queue(move |world: &mut World| start_intro(world, map));
        }
    }
}
