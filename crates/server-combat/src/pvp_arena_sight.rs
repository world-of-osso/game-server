//! Physical TDB1210 Shadow Sight: TC generic arena condition enables at90s;
//! trapData2/2 is activation radius, Data3 is the spell, source spawn time the respawn.
use super::PvpServerState;
use crate::game_objects::{GameObjectSpawn, GameObjectSpawns, bevy_position, game_object_bundle};
use crate::interest::NetworkVisibility;
use crate::maps::InMap;
use crate::persistence::ClientId;
use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::Connection;
use shared::battleground::BgPhase;
use shared::components::{Health, Position};
use shared::protocol::{GameObjectInfo, GameObjectVisible};
use std::collections::{BTreeMap, BTreeSet};
const ACTIVATION_SECONDS: f64 = 90.0;

#[derive(Clone)]
struct SightSource {
    spawn: GameObjectSpawn,
    radius: f32,
    spell: u32,
    respawn_secs: u32,
}
#[derive(Resource, Default)]
pub(crate) struct SightCatalog(BTreeMap<u32, Vec<SightSource>>);
#[derive(Resource, Default)]
pub(crate) struct SightTimers(BTreeMap<(InMap, u32), SightSlot>);
#[derive(Default)]
struct SightSlot {
    entity: Option<Entity>,
    ready_at: Option<f64>,
}
impl SightCatalog {
    pub(crate) fn load(db: &Connection, spawns: &GameObjectSpawns) -> rusqlite::Result<Self> {
        let mut query=db.prepare("SELECT g.guid,t.Data2,t.Data3,t.Data4,g.spawntimesecs FROM content_gameobject g JOIN content_gameobject_template t ON t.entry=g.id JOIN arena_map a ON a.map_id=g.map WHERE t.type=6 AND t.name='Shadow Sight'")?;
        let metadata: BTreeMap<u32, (f32, u32, u32)> = query
            .query_map([], |row| {
                let charges: u32 = row.get(3)?;
                if charges != 1 {
                    return Err(rusqlite::Error::InvalidQuery);
                }
                Ok((
                    row.get(0)?,
                    (row.get::<_, f32>(1)? / 2.0, row.get(2)?, row.get(4)?),
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut maps: BTreeMap<u32, Vec<SightSource>> = BTreeMap::new();
        for spawn in &spawns.0 {
            if let Some((radius, spell, respawn_secs)) = metadata.get(&spawn.guid) {
                maps.entry(spawn.map_id).or_default().push(SightSource {
                    spawn: spawn.clone(),
                    radius: *radius,
                    spell: *spell,
                    respawn_secs: *respawn_secs,
                });
            }
        }
        Ok(Self(maps))
    }
}
pub(super) fn prepare_map(world: &mut World, map: InMap) {
    let sources = world.resource::<SightCatalog>().0.get(&map.map_id).cloned();
    if let Some(sources) = sources {
        world.resource_scope(|world, mut timers: Mut<SightTimers>| {
            for source in &sources {
                let entity = find_orb(world, map, source);
                if let Some(entity) = entity {
                    set_visible(world, entity, false);
                }
                timers.0.insert(
                    (map, source.spawn.guid),
                    SightSlot {
                        entity,
                        ready_at: None,
                    },
                );
            }
        });
    }
}

#[derive(Clone)]
struct ActiveMatch {
    phase: BgPhase,
    started: Option<f64>,
    roster: BTreeSet<u64>,
}
struct Actor {
    entity: Entity,
    id: u64,
    map: InMap,
    position: Vec3,
}

pub(super) fn update_sight(world: &mut World, now: f64) {
    let maps: BTreeMap<_, _> = world
        .resource::<PvpServerState>()
        .battleground_matches
        .values()
        .filter_map(|matched| {
            let arena = matched.arena.as_ref()?;
            Some((
                matched.map?,
                ActiveMatch {
                    phase: matched.instance.phase,
                    started: arena.started,
                    roster: matched
                        .instance
                        .team_a_players
                        .iter()
                        .chain(&matched.instance.team_b_players)
                        .copied()
                        .collect(),
                },
            ))
        })
        .collect();
    if maps.is_empty() {
        world.resource_mut::<SightTimers>().0.clear();
        return;
    }
    let catalog = world.resource::<SightCatalog>().0.clone();
    let mut query = world.query::<(Entity, &ClientId, &InMap, &Position, &Health)>();
    let actors: Vec<_> = query
        .iter(world)
        .filter(|(_, _, _, _, health)| health.current > 0.0)
        .map(|(entity, id, map, pos, _)| Actor {
            entity,
            id: id.0,
            map: *map,
            position: Vec3::new(pos.x, pos.y, pos.z),
        })
        .collect();
    world.resource_scope(|world, mut timers: Mut<SightTimers>| {
        timers.0.retain(|(map, _), _| maps.contains_key(map));
        for (map, matched) in &maps {
            if let Some(sources) = catalog.get(&map.map_id) {
                for source in sources {
                    let slot = timers
                        .0
                        .entry((*map, source.spawn.guid))
                        .or_insert_with(|| SightSlot {
                            entity: find_orb(world, *map, source),
                            ready_at: None,
                        });
                    update_orb(world, slot, *map, matched, source, &actors, now);
                }
            }
        }
    });
}
fn set_visible(world: &mut World, entity: Entity, visible: bool) {
    let desired = GameObjectVisible(visible);
    if world.get::<GameObjectVisible>(entity) != Some(&desired) {
        world.entity_mut(entity).insert(desired);
    }
}
fn find_orb(world: &mut World, map: InMap, source: &SightSource) -> Option<Entity> {
    let position = bevy_position(source.spawn.position);
    let mut query = world.query::<(Entity, &InMap, &GameObjectInfo, &Position)>();
    query.iter(world).find_map(|(entity, copy, info, pos)| {
        (*copy == map && info.entry == source.spawn.info.entry && *pos == position)
            .then_some(entity)
    })
}
fn update_orb(
    world: &mut World,
    slot: &mut SightSlot,
    map: InMap,
    matched: &ActiveMatch,
    source: &SightSource,
    actors: &[Actor],
    now: f64,
) {
    let existing = slot.entity;
    let Some(start) = matched.started else {
        if let Some(entity) = existing {
            set_visible(world, entity, false);
        }
        return;
    };
    let ready = *slot.ready_at.get_or_insert(start + ACTIVATION_SECONDS);
    if matched.phase != BgPhase::InProgress || now < ready {
        if let Some(entity) = existing {
            set_visible(world, entity, false);
        }
        return;
    }
    let orb = match existing {
        Some(entity) => entity,
        None => world
            .spawn((
                game_object_bundle(&source.spawn),
                map,
                Replicate::to_clients(NetworkTarget::All),
                NetworkVisibility::default(),
            ))
            .id(),
    };
    slot.entity = Some(orb);
    set_visible(world, orb, true);
    let position = bevy_position(source.spawn.position);
    let position = Vec3::new(position.x, position.y, position.z);
    let target = actors
        .iter()
        .filter(|actor| actor.map == map && matched.roster.contains(&actor.id))
        .filter(|actor| actor.position.distance_squared(position) <= source.radius * source.radius)
        .min_by(|left, right| {
            left.position
                .distance_squared(position)
                .total_cmp(&right.position.distance_squared(position))
        });
    if let Some(target) = target {
        crate::auras::apply_aura(
            world,
            &crate::auras::AuraApplication {
                caster: orb,
                target: target.entity,
                spell_id: source.spell,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        set_visible(world, orb, false);
        slot.ready_at = Some(now + f64::from(source.respawn_secs));
    }
}
