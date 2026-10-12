//! Source fissure positions + ADT areas, dynamic Azerite objects and normal Gathering casts.
use super::{InvitedBattleground, PlayerPvpState, PvpServerState};
use crate::game_objects::{GameObjectBounds, wow_position};
use crate::maps::InMap;
use crate::spell_cast::CastFailure;
use bevy::prelude::*;
use lightyear::prelude::*;
use rand::Rng;
use rusqlite::Connection;
use shared::battleground::{
    AZERITE_COLLECTED_STAT, AZERITE_GATHERING_SPELL, AzeriteEvent, BgPhase, SEETHING_AREAS,
    SEETHING_OBJECTS, SeethingBattle,
};
use shared::components::{Health, Position};
use shared::protocol::GameObjectInfo;
use std::collections::BTreeMap;

#[derive(Resource, Debug, Clone, Default)]
pub(crate) struct AzeriteTemplates(BTreeMap<u32, (GameObjectInfo, GameObjectBounds)>);

pub(crate) fn load_azerite_templates(conn: &Connection) -> rusqlite::Result<AzeriteTemplates> {
    let mut query=conn.prepare("SELECT t.entry,t.type,t.displayId,t.name,t.size,d.GeoBox_0,d.GeoBox_1,d.GeoBox_2,d.GeoBox_3,d.GeoBox_4,d.GeoBox_5 FROM tdb_gameobject_template t JOIN game_object_display_info d ON d.ID=t.displayId WHERE t.entry IN (272471,281306,281307)")?;
    let rows = query.query_map([], |row| {
        let scale = row.get::<_, f64>(4)? as f32;
        let geo = |i| row.get::<_, f64>(i).map(|value| value as f32);
        let info = GameObjectInfo {
            entry: row.get(0)?,
            go_type: row.get(1)?,
            display_id: row.get(2)?,
            name: row.get(3)?,
            scale,
        };
        let bounds = GameObjectBounds {
            min: Vec3::new(geo(5)?, geo(6)?, geo(7)?),
            max: Vec3::new(geo(8)?, geo(9)?, geo(10)?),
            scale,
            orientation: 0.0,
        };
        Ok((info.entry, (info, bounds)))
    })?;
    Ok(AzeriteTemplates(rows.collect::<rusqlite::Result<_>>()?))
}

#[derive(Component)]
struct AzeriteNode {
    generation: u64,
    available: bool,
}

#[derive(Debug)]
pub(super) struct SeethingRuntime {
    pub battle: SeethingBattle,
    sites: BTreeMap<u32, Vec3>,
    templates: AzeriteTemplates,
    objects: BTreeMap<u64, Entity>,
}

impl SeethingRuntime {
    pub fn from_source(world: &mut World) -> Result<Self, String> {
        let templates = world
            .get_resource::<AzeriteTemplates>()
            .ok_or("Azerite template catalog missing")?
            .clone();
        for entry in SEETHING_OBJECTS {
            if !templates.0.contains_key(&entry) {
                return Err(format!("source Azerite template/display {entry} missing"));
            }
        }
        let fissures: Vec<_> = world
            .get_resource::<crate::world_data::WorldData>()
            .ok_or("Seething source creature catalog missing")?
            .spawns
            .iter()
            .filter(|spawn| spawn.map == 1803 && spawn.template_entry == 125253)
            .map(|spawn| Vec3::new(spawn.position_x, spawn.position_y, spawn.position_z))
            .collect();
        let mut ground = world
            .get_resource_mut::<crate::ground::GroundMap>()
            .ok_or("Seething source terrain missing")?;
        let mut sites = BTreeMap::new();
        for position in fissures {
            let (x, y, z) = crate::terrain_height::wow_to_bevy(position.x, position.y, position.z);
            let area = ground
                .area_at(1803, Vec3::new(x, y, z))
                .ok_or_else(|| format!("Seething fissure {position:?} has no authored ADT area"))?;
            if !SEETHING_AREAS.contains(&area) || sites.insert(area, position).is_some() {
                return Err(format!(
                    "Seething fissure duplicate/unsupported area {area}"
                ));
            }
        }
        if sites.len() != SEETHING_AREAS.len() {
            return Err(format!(
                "Seething source requires twelve fissure areas; found {}",
                sites.len()
            ));
        }
        Ok(Self {
            battle: SeethingBattle::new(),
            sites,
            templates,
            objects: BTreeMap::new(),
        })
    }

    pub fn advance(&mut self, world: &mut World, map: InMap, delta_ms: u64) {
        let mut rng = rand::thread_rng();
        for event in self
            .battle
            .advance(delta_ms, &mut |count| rng.gen_range(0..count))
        {
            match event {
                AzeriteEvent::Spawned {
                    area,
                    generation,
                    entry,
                } => self.spawn_object(world, map, area, generation, entry),
                AzeriteEvent::Removed { generation } => {
                    if let Some(entity) = self.objects.remove(&generation) {
                        world.despawn(entity);
                    }
                }
                AzeriteEvent::Activated { .. } => {}
            }
        }
    }

    fn spawn_object(
        &mut self,
        world: &mut World,
        map: InMap,
        area: u32,
        generation: u64,
        entry: u32,
    ) {
        let (info, bounds) = &self.templates.0[&entry];
        let at = self.sites[&area];
        let (x, y, z) = crate::terrain_height::wow_to_bevy(at.x, at.y, at.z);
        let entity = world
            .spawn((
                Position { x, y, z },
                shared::components::Rotation {
                    x: 0.0,
                    y: std::f32::consts::FRAC_PI_2,
                    z: 0.0,
                },
                info.clone(),
                *bounds,
                map,
                AzeriteNode {
                    generation,
                    available: true,
                },
                Replicate::to_clients(NetworkTarget::All),
                crate::interest::NetworkVisibility::default(),
            ))
            .id();
        self.objects.insert(generation, entity);
    }
}

pub(super) fn update_seething(
    world: &mut World,
    matched: &mut InvitedBattleground,
    map: InMap,
    delta_ms: u64,
) {
    let runtime = matched.seething.as_mut().expect("Seething runtime");
    runtime.advance(world, map, delta_ms);
    // Scores change only at a completed Gathering cast, which sends snapshots.
}

pub(crate) fn check_azerite_target(
    world: &World,
    player: Entity,
    object: Entity,
) -> Result<(), CastFailure> {
    let map = check_participant(world, player)?;
    let available = world
        .get::<AzeriteNode>(object)
        .is_some_and(|node| node.available);
    if world.get::<InMap>(object) != Some(&map) || !available {
        return Err(CastFailure::InvalidTarget);
    }
    check_object_reach(world, player, object)
}

fn check_participant(world: &World, player: Entity) -> Result<InMap, CastFailure> {
    let snapshot = world
        .get::<PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
        .ok_or(CastFailure::InvalidTarget)?;
    let state = world.resource::<PvpServerState>();
    let matched = state
        .battleground_matches
        .get(&snapshot.match_id)
        .ok_or(CastFailure::InvalidTarget)?;
    let map = matched.map.ok_or(CastFailure::InvalidTarget)?;
    let character = world
        .get::<crate::persistence::ClientId>(player)
        .ok_or(CastFailure::InvalidTarget)?
        .0;
    if matched.instance.phase != BgPhase::InProgress
        || !matched.accepted.contains(&character)
        || world.get::<InMap>(player) != Some(&map)
        || world
            .get::<Health>(player)
            .is_none_or(|health| health.current <= 0.0)
    {
        return Err(CastFailure::InvalidTarget);
    }
    Ok(map)
}

fn check_object_reach(world: &World, player: Entity, object: Entity) -> Result<(), CastFailure> {
    let at = world
        .get::<Position>(object)
        .ok_or(CastFailure::InvalidTarget)?;
    let from = world
        .get::<Position>(player)
        .ok_or(CastFailure::InvalidTarget)?;
    let bounds = world
        .get::<GameObjectBounds>(object)
        .ok_or(CastFailure::InvalidTarget)?;
    if !bounds.reaches(
        wow_position(at),
        wow_position(from),
        crate::game_objects::GAME_OBJECT_INTERACT_DISTANCE,
    ) {
        return Err(CastFailure::OutOfRange);
    }
    Ok(())
}

pub(super) fn is_azerite_object(world: &World, object: Entity) -> bool {
    world.get::<AzeriteNode>(object).is_some()
}

pub(super) fn begin_gathering(
    world: &mut World,
    player: Entity,
    object: Entity,
) -> Result<String, String> {
    check_azerite_target(world, player, object)
        .map_err(|error| format!("Azerite Gathering refused: {error:?}"))?;
    crate::spell_cast::cast_granted(world, player, AZERITE_GATHERING_SPELL, Some(object))
        .map_err(|error| format!("Azerite Gathering cast refused: {error:?}"))?;
    Ok("Gathering Azerite".into())
}

pub(crate) fn capture_azerite(world: &mut World, player: Entity, object: Entity) {
    if let Err(error) = check_azerite_target(world, player, object) {
        error!("Azerite completion refused: {error:?}");
        return;
    }
    let generation = world
        .get::<AzeriteNode>(object)
        .expect("validated Azerite")
        .generation;
    let result = world.resource_scope(|world, mut state: Mut<PvpServerState>| {
        apply_azerite_capture(world, &mut state, player, object, generation)
    });
    match result {
        Ok(match_id) => {
            world
                .get_mut::<AzeriteNode>(object)
                .expect("captured Azerite")
                .available = false;
            super::pvp_objective::send_match_snapshot(world, match_id);
        }
        Err(error) => error!("Azerite completion refused: {error}"),
    }
}

fn apply_azerite_capture(
    world: &World,
    state: &mut PvpServerState,
    player: Entity,
    object: Entity,
    generation: u64,
) -> Result<u64, String> {
    let (team, character, matched) =
        super::pvp_objective::active_match(world, state, player, object)?;
    let template = shared::battleground::seething_shore();
    matched
        .seething
        .as_mut()
        .ok_or("Seething runtime missing")?
        .battle
        .capture(generation, team, &mut matched.instance, &template)?;
    super::pvp_objective::credit(
        &mut matched.scoreboard,
        character,
        AZERITE_COLLECTED_STAT,
        1,
    );
    Ok(world
        .get::<PlayerPvpState>(player)
        .expect("validated participant")
        .battleground
        .as_ref()
        .expect("validated match")
        .match_id)
}
