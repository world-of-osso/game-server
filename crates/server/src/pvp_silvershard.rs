//! TC a352b1fa Silvershard: real template60140 NPCs driven by the shared source paths.
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use shared::battleground::{
    BgInstance, BgTemplate, CartEvent, CartPaths, SILVERSHARD_CARTS, SilvershardBattle,
};
use shared::components::{Position, Rotation};

use crate::maps::InMap;
use crate::world_data::{CreatureSpawn, WorldData};

#[derive(Debug)]
pub(super) struct CartRuntime {
    pub battle: SilvershardBattle,
    paths: CartPaths,
    entities: [Option<Entity>; 3],
}

impl CartRuntime {
    pub fn from_source(data: &WorldData) -> Result<Self, String> {
        if !data.templates.contains_key(&60140) || !data.templates.contains_key(&60283) {
            return Err(
                "Silvershard templates missing; run add_battleground_summon_templates.py".into(),
            );
        }
        let mut paths = CartPaths::new();
        for id in 6014000..=6014011 {
            let path = data
                .waypoints
                .get(&id)
                .filter(|path| !path.points.is_empty())
                .ok_or_else(|| format!("Silvershard source cart path {id} is absent"))?;
            paths.insert(
                id,
                path.points
                    .iter()
                    .map(|point| [point.position_x, point.position_y, point.position_z])
                    .collect(),
            );
        }
        Ok(Self {
            battle: SilvershardBattle::new(),
            paths,
            entities: [None; 3],
        })
    }

    pub fn advance(
        &mut self,
        world: &mut World,
        map: InMap,
        delta_ms: u64,
        players: &[(u64, shared::battleground::BgTeam, [f32; 3])],
        instance: &mut BgInstance,
        template: &BgTemplate,
    ) -> Vec<CartEvent> {
        let events = self
            .battle
            .advance(delta_ms, players, &self.paths, instance, template);
        self.synchronize(world, map);
        events
    }

    pub fn synchronize(&mut self, world: &mut World, map: InMap) {
        for (index, site) in self.battle.sites().into_iter().enumerate() {
            match (self.entities[index], site) {
                (Some(entity), None) => {
                    world.despawn(entity);
                    self.entities[index] = None;
                }
                (None, Some(at)) => {
                    self.entities[index] = Some(spawn_cart(world, map, index, at));
                }
                (Some(entity), Some(at)) => move_source_cart(world, entity, at, 2.0),
                (None, None) => {}
            }
        }
    }
}

#[derive(Component)]
pub(crate) struct TrackSwitch(pub u32);

pub(crate) fn insert_track_switch(entity: &mut EntityCommands, spawn: &CreatureSpawn) {
    if spawn.map != 727 || spawn.template_entry != 60283 {
        return;
    }
    // Promoted TDB1210 source GUIDs use the content namespace offset.
    const RETAIL_GUID_OFFSET: u32 = 20_000_000;
    let state = match spawn.guid.checked_sub(RETAIL_GUID_OFFSET) {
        Some(4000143) => 6467,
        Some(4000144) => 6468,
        _ => return,
    };
    entity.insert(TrackSwitch(state));
}

/// TC97372 hits the nearest source Track Switch; only an active same-copy participant changes it.
pub(crate) fn change_track(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    let Some(state) = world.get::<TrackSwitch>(target).map(|switch| switch.0) else {
        return;
    };
    let Some(map) = world
        .get::<InMap>(target)
        .copied()
        .filter(|map| Some(*map) == world.get::<InMap>(caster).copied())
    else {
        return;
    };
    let Some(id) = world
        .get::<crate::persistence::ClientId>(caster)
        .map(|id| id.0)
    else {
        return;
    };
    let changed = world.resource_scope(|_, mut matches: Mut<super::PvpServerState>| {
        matches
            .battleground_matches
            .iter_mut()
            .find_map(|(match_id, matched)| {
                let participant = matched.instance.team_a_players.contains(&id)
                    || matched.instance.team_b_players.contains(&id);
                if matched.map != Some(map)
                    || !participant
                    || matched.instance.phase != shared::battleground::BgPhase::InProgress
                {
                    return None;
                }
                let carts = matched.silvershard.as_mut()?;
                carts
                    .battle
                    .switch_track(state)
                    .expect("source Track Switch state");
                Some(*match_id)
            })
    });
    if let Some(match_id) = changed {
        super::pvp_objective::send_match_snapshot(world, match_id);
    }
}

fn spawn_cart(world: &mut World, map: InMap, index: usize, at: [f32; 3]) -> Entity {
    spawn_source_cart(world, map, 60140, at, SILVERSHARD_CARTS[index].orientation)
}

pub(super) fn spawn_source_cart(
    world: &mut World,
    map: InMap,
    entry: u32,
    at: [f32; 3],
    orientation: f32,
) -> Entity {
    // Script summon, not an invented static source placement.
    let spawn = CreatureSpawn {
        guid: 0,
        template_entry: entry,
        map: map.map_id as u16,
        zone_id: 0,
        position_x: at[0],
        position_y: at[1],
        position_z: at[2],
        orientation,
        wander_distance: 0.0,
        movement_type: 0,
        spawntimesecs: 0,
        display_id: None,
        unit_flags: None,
        spawn_difficulties: vec![0],
        equipment_id: 0,
    };
    let mut queue = CommandQueue::default();
    let difficulty_id = crate::zones::map_difficulty_id(world, map);
    let entity = {
        let data = world.resource::<WorldData>();
        let mut commands = Commands::new(&mut queue, world);
        crate::zones::spawn_creature(
            &mut commands,
            data,
            &data.templates[&entry],
            &mut crate::zones::CreatureSpawnContext {
                map,
                difficulty_id,
                ground: None,
                temporary: true,
            },
            true,
            &spawn,
            &mut rand::thread_rng(),
        )
    };
    queue.apply(world);
    entity
}

pub(super) fn move_source_cart(world: &mut World, entity: Entity, at: [f32; 3], speed: f32) {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(at[0], at[1], at[2]);
    let next = Position { x, y, z };
    let old = *world.get::<Position>(entity).expect("live cart position");
    if next == old {
        return;
    }
    let bearing = (next.x - old.x).atan2(next.z - old.z);
    world.entity_mut(entity).insert((
        next,
        Rotation {
            x: 0.0,
            y: bearing,
            z: 0.0,
        },
        shared::components::MovementSpeed(speed),
        shared::components::CreatureMotion::Walk,
    ));
    crate::interest::mark_interest_position_changed(
        world.get_resource::<crate::interest::InterestDirtyQueue>(),
        entity,
    );
}
