//! TC a352b1fa Ragefire OnlyOnceAreaTriggerScripts + ScriptMgr.cpp2705–2715:
//! at_lava_guard_gordoth_intro7899 and at_adarogg_intro7904 each DoAction their boss.
//! Existing Player entities model ordinary players, not TC game-master mode;
//! account privileges are deliberately not inferred to be IsGameMaster.
use bevy::prelude::*;
use shared::components::{MovementControl, Npc, Player, Position};

use crate::maps::{InMap, store::AreaTriggerShape};
use crate::scripted_ai::{CreatureAiState, Hook, dispatch, instance_script};

const MAP: u32 = 389;

/// One authored trigger: DB2 id, TDB script, boss entry, `BOSS_*` data id, action.
struct Authored {
    trigger: u32,
    script: &'static str,
    boss: u32,
    boss_id: u32,
    action: i32,
}

const AUTHORED: [Authored; 2] = [
    Authored {
        trigger: 7899,
        script: "at_lava_guard_gordoth_intro",
        boss: 61528,
        boss_id: super::GORDOTH_BOSS_ID,
        action: super::GORDOTH_ACTION_JUMP,
    },
    Authored {
        trigger: 7904,
        script: "at_adarogg_intro",
        boss: 61408,
        boss_id: super::ADAROGG_BOSS_ID,
        action: super::ADAROGG_ACTION_KILL_HOUNDMASTERS,
    },
];

#[cfg_attr(test, derive(Clone))]
struct IntroTrigger {
    authored: &'static Authored,
    center: Vec3,
    shape: AreaTriggerShape,
}

#[derive(Resource)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct IntroTriggers(Vec<IntroTrigger>);

/// Every authored trigger's actual DB2 shape, admitted only through its TDB script row.
pub(crate) fn load_triggers(db: &rusqlite::Connection) -> rusqlite::Result<IntroTriggers> {
    AUTHORED
        .iter()
        .map(|authored| load_trigger(db, authored))
        .collect::<rusqlite::Result<_>>()
        .map(IntroTriggers)
}

fn load_trigger(
    db: &rusqlite::Connection,
    authored: &'static Authored,
) -> rusqlite::Result<IntroTrigger> {
    db.query_row(
        "SELECT a.Pos_0,a.Pos_1,a.Pos_2,a.ShapeType,a.Radius,a.Box_length,a.Box_width,a.Box_height,a.Box_yaw \
         FROM area_trigger a JOIN tdb_areatrigger_scripts s ON s.entry=a.ID \
         WHERE a.ID=?1 AND a.ContinentID=?2 AND s.ScriptName=?3",
        rusqlite::params![authored.trigger, MAP, authored.script],
        |row| {
            let shape = AreaTriggerShape::from_db2(
                row.get(3)?, row.get(4)?,
                [row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?],
            ).ok_or(rusqlite::Error::InvalidQuery)?;
            let center = Vec3::new(row.get(0)?, row.get(1)?, row.get(2)?);
            Ok(IntroTrigger { authored, center, shape })
        },
    )
}

/// Last observed containment per trigger (bit = `AUTHORED` index).
#[derive(Component, Clone, Copy)]
pub(crate) struct IntroPresence {
    map: InMap,
    inside: u8,
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
    triggers: Option<Res<IntroTriggers>>,
    mut commands: Commands,
) {
    let Some(triggers) = triggers else { return };
    for (player, &map, position, control, presence) in &players {
        if map.map_id != MAP || map.instance_id == 0 {
            commands.entity(player).remove::<IntroPresence>();
            continue;
        }
        let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
        let point = Vec3::new(x, y, z);
        let inside = triggers
            .0
            .iter()
            .enumerate()
            .filter(|(_, trigger)| trigger.shape.contains(trigger.center, point))
            .fold(0u8, |mask, (index, _)| mask | 1 << index);
        let before = presence
            .filter(|last| last.map == map)
            .map(|last| last.inside);
        commands
            .entity(player)
            .insert(IntroPresence { map, inside });
        // MiscHandler.cpp ignores an in-flight player's trigger packet before scripts.
        let Some(before) = before else { continue };
        if control.is_some_and(|control| control.controlled) {
            continue;
        }
        for (index, trigger) in triggers.0.iter().enumerate() {
            if inside & !before & (1 << index) != 0 {
                let authored = trigger.authored;
                commands.queue(move |world: &mut World| handle_once(world, map, authored));
            }
        }
    }
}

/// `OnlyOnceAreaTriggerScript::OnTrigger`: TryHandleOnce finds the boss, DoActions it,
/// and only then marks the trigger handled for this instance.
fn handle_once(world: &mut World, map: InMap, authored: &Authored) {
    if instance_script::get_boss_state(world, map, authored.boss_id).is_none() {
        return;
    }
    let scripts = world.resource::<instance_script::InstanceScripts>();
    if scripts.0[&map]
        .handled_area_triggers
        .contains(&authored.trigger)
    {
        return;
    }
    let boss = world.query_filtered::<(Entity, &InMap, &Npc),
        (With<CreatureAiState>, Without<crate::combat::DeathState>)>()
        .iter(world)
        .find_map(|(entity, copy, npc)| (*copy == map && npc.template_id == authored.boss).then_some(entity));
    let Some(boss) = boss else { return };
    dispatch(world, boss, Hook::DoAction(authored.action));
    world
        .resource_mut::<instance_script::InstanceScripts>()
        .0
        .get_mut(&map)
        .expect("Ragefire instance script")
        .handled_area_triggers
        .insert(authored.trigger);
}
