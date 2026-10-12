//! Authored preparation gates. TC map scripts determine exactly which doors open
//! and whether they disappear; TDB1210 supplies their actual placements/models.
use super::PvpServerState;
pub(crate) use super::arena_sight::{SightCatalog, SightTimers};
use crate::maps::InMap;
use bevy::prelude::*;
use rusqlite::Connection;
use shared::protocol::{GameObjectAnimation, GameObjectCollision, GameObjectInfo};
use std::collections::BTreeMap;
const CLOSED_ANIMATION: u16 = 147;
const OPEN_ANIMATION: u16 = 148;

#[derive(Clone)]
struct Gate {
    delete: bool,
    despawn_secs: u32,
}
#[derive(Resource, Default)]
pub(crate) struct GateCatalog(BTreeMap<(u32, u32), Gate>);
impl GateCatalog {
    pub(crate) fn load(db: &Connection) -> rusqlite::Result<Self> {
        let mut rows = db.prepare("SELECT map_id,entry,action,despawn_secs FROM arena_gate")?;
        let gates = rows
            .query_map([], |row| {
                let action: String = row.get(2)?;
                if action != "open" && action != "delete" {
                    return Err(rusqlite::Error::InvalidQuery);
                }
                Ok((
                    (row.get(0)?, row.get(1)?),
                    Gate {
                        delete: action == "delete",
                        despawn_secs: row.get(3)?,
                    },
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Self(gates))
    }
}
pub(super) fn prepare_map(world: &mut World, map: InMap) {
    let gates = world.resource::<GateCatalog>().0.clone();
    let mut query = world.query::<(Entity, &InMap, &GameObjectInfo)>();
    let doors: Vec<_> = query
        .iter(world)
        .filter(|(_, copy, info)| {
            **copy == map
                && gates
                    .get(&(map.map_id, info.entry))
                    .is_some_and(|gate| !gate.delete)
        })
        .map(|(entity, _, _)| entity)
        .collect();
    for entity in doors {
        world.entity_mut(entity).insert((
            GameObjectAnimation {
                animation_id: CLOSED_ANIMATION,
                looping: true,
            },
            GameObjectCollision(true),
        ));
    }
}

pub(super) fn update_gates(world: &mut World, now: f64) {
    let maps: BTreeMap<_, _> = world
        .resource::<PvpServerState>()
        .battleground_matches
        .values()
        .filter_map(|matched| Some((matched.map?, matched.arena.as_ref()?.started)))
        .collect();
    if maps.is_empty() {
        return;
    }
    let gates = world.resource::<GateCatalog>().0.clone();
    let mut query = world.query::<(
        Entity,
        &InMap,
        &GameObjectInfo,
        Option<&GameObjectAnimation>,
        Option<&GameObjectCollision>,
    )>();
    let changes: Vec<_> = query
        .iter(world)
        .filter_map(|(entity, map, info, animation, collision)| {
            let started = maps.get(map)?;
            let source = gates.get(&(map.map_id, info.entry))?;
            let remove = started.is_some_and(|start| {
                source.delete
                    || (source.despawn_secs > 0 && now - start >= f64::from(source.despawn_secs))
            });
            if source.delete && started.is_none() {
                return None;
            }
            let desired = GameObjectAnimation {
                animation_id: if started.is_some() {
                    OPEN_ANIMATION
                } else {
                    CLOSED_ANIMATION
                },
                looping: started.is_none(),
            };
            let desired_collision = GameObjectCollision(started.is_none());
            (remove || animation != Some(&desired) || collision != Some(&desired_collision))
                .then_some((entity, remove, desired, desired_collision))
        })
        .collect();
    for (entity, remove, animation, collision) in changes {
        if remove {
            world.despawn(entity);
        } else {
            world.entity_mut(entity).insert((animation, collision));
        }
    }
}
