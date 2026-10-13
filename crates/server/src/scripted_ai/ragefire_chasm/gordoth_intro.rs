//! TC a352b1fa at_lava_guard_gordoth_intro7899 + ScriptMgr.cpp2705–2715.
//! Existing Player entities model ordinary players, not TC game-master mode;
//! account privileges are deliberately not inferred to be IsGameMaster.
use bevy::prelude::*;
use shared::components::{MovementControl, Npc, Player, Position};

use crate::maps::{InMap, store::AreaTriggerShape};
use crate::scripted_ai::{CreatureAiState, Hook, dispatch, instance_script};

const MAP: u32 = 389;
const TRIGGER: u32 = 7899;
const BOSS: u32 = 61528;

#[derive(Resource)]
pub(crate) struct IntroTrigger {
    center: Vec3,
    shape: AreaTriggerShape,
}

pub(crate) fn load_trigger(db: &rusqlite::Connection) -> rusqlite::Result<IntroTrigger> {
    db.query_row(
        "SELECT a.Pos_0,a.Pos_1,a.Pos_2,a.ShapeType,a.Radius,a.Box_length,a.Box_width,a.Box_height,a.Box_yaw \
         FROM area_trigger a JOIN tdb_areatrigger_scripts s ON s.entry=a.ID \
         WHERE a.ID=7899 AND a.ContinentID=389 AND s.ScriptName='at_lava_guard_gordoth_intro'",
        [],
        |row| {
            let shape = AreaTriggerShape::from_db2(
                row.get(3)?, row.get(4)?,
                [row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?],
            ).ok_or(rusqlite::Error::InvalidQuery)?;
            Ok(IntroTrigger { center: Vec3::new(row.get(0)?, row.get(1)?, row.get(2)?), shape })
        },
    )
}

#[derive(Component, Clone, Copy)]
pub(crate) struct IntroPresence {
    map: InMap,
    inside: bool,
}

type Players<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static InMap,
        &'static Position,
        Option<&'static MovementControl>,
        Option<&'static IntroPresence>,
    ),
    (With<Player>, Changed<Position>),
>;

/// Authoritative entry edges, like the existing quest/teleport trigger detectors.
/// First position on a map only establishes presence, not a walked-in packet.
pub(crate) fn detect_entries(
    players: Players,
    trigger: Option<Res<IntroTrigger>>,
    mut commands: Commands,
) {
    let Some(trigger) = trigger else { return };
    for (player, &map, position, control, presence) in &players {
        if map.map_id != MAP || map.instance_id == 0 {
            commands.entity(player).remove::<IntroPresence>();
            continue;
        }
        let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
        let inside = trigger.shape.contains(trigger.center, Vec3::new(x, y, z));
        let entered = inside && presence.is_some_and(|last| last.map == map && !last.inside);
        commands
            .entity(player)
            .insert(IntroPresence { map, inside });
        // MiscHandler.cpp ignores an in-flight player's trigger packet before scripts.
        if entered && !control.is_some_and(|control| control.controlled) {
            commands.queue(move |world: &mut World| handle_once(world, map));
        }
    }
}

fn handle_once(world: &mut World, map: InMap) {
    if instance_script::get_boss_state(world, map, super::GORDOTH_BOSS_ID).is_none() {
        return;
    }
    let scripts = world.resource::<instance_script::InstanceScripts>();
    if scripts.0[&map].handled_area_triggers.contains(&TRIGGER) {
        return;
    }
    let boss = world.query_filtered::<(Entity, &InMap, &Npc),
        (With<CreatureAiState>, Without<crate::combat::DeathState>)>()
        .iter(world)
        .find_map(|(entity, copy, npc)| (*copy == map && npc.template_id == BOSS).then_some(entity));
    let Some(boss) = boss else { return };
    dispatch(world, boss, Hook::DoAction(super::GORDOTH_ACTION_JUMP));
    world
        .resource_mut::<instance_script::InstanceScripts>()
        .0
        .get_mut(&map)
        .expect("Gordoth instance script")
        .handled_area_triggers
        .insert(TRIGGER);
}
