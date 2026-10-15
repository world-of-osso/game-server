//! Grid activation: world-spawned creatures only run their per-tick systems
//! while a player is in or next to one of their grids.
//!
//! AzerothCore divides a map into grids of `SIZE_OF_GRIDS` = 533.3333 yd
//! (`src/common/Collision/Maps/MapDefines.h`) and buckets a position by
//! `x / SIZE_OF_GRIDS` (`src/server/game/Grids/GridDefines.h`
//! `ComputeGridCoord`). Each `Map::Update` (`src/server/game/Maps/Map.cpp`)
//! marks the cells around every player (`MarkNearbyCellsOf`), and only
//! objects that are `IsUpdateNeeded` get `Update(diff)`; a creature outside
//! the marked cells is skipped (`Creature::IsUpdateNeeded`,
//! `src/server/game/Entities/Creature/Creature.cpp`). AzerothCore marks
//! 66.7 yd cells within the visibility range; here the unit is the grid and
//! a creature is active when one of its home grids is within one grid (the
//! 3x3 neighbourhood) of any player's grid.
//!
//! Home grids are the spawn grid plus every grid the creature's patrol path
//! (or its formation leader's path) crosses. AzerothCore keeps waypoint
//! movers and formation members updating everywhere; covering the path's
//! grids keeps a patroller active wherever a player could see it instead.
//!
//! Dormant state is kept as is and resumes on reactivation: a wanderer or
//! patroller continues its leg. Aura expiry and periodic ticks are absolute
//! (`auras::aura_now`), so on reactivation expired auras end and missed ticks
//! run at once, like AzerothCore's catch-up with the accumulated diff.
//! Respawn is not gated: `combat::respawn_system` only visits dead creatures,
//! so a dormant creature respawns at its spawn point on its normal timer.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use shared::components::{Player, Position};

use crate::creature_ai::{FormationMember, SpawnPoint, WaypointState};
use crate::maps::InMap;
use crate::terrain_height::{bevy_to_wow, wow_to_bevy};
use crate::world_data::WorldData;

/// AzerothCore `SIZE_OF_GRIDS` (MapDefines.h).
pub const SIZE_OF_GRIDS: f32 = 533.3333;

/// A world-spawned creature with no player within one grid of its home grids.
/// Per-unit creature systems skip entities carrying it. Sparse storage avoids
/// copying the creature's wide component table when its grid changes activity.
#[derive(Component, Debug)]
#[component(storage = "SparseSet")]
pub struct GridDormant;

/// A grid of one map copy.
type GridCoord = (InMap, i32, i32);

pub struct GridActivityPlugin;

impl Plugin for GridActivityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GridIndex>()
            .add_observer(queue_new_creature)
            .add_observer(forget_creature)
            .add_systems(
                FixedPreUpdate,
                (register_creatures, update_grid_activity).chain(),
            );
    }
}

#[derive(Resource, Default)]
pub(crate) struct GridIndex {
    /// Creatures whose `SpawnPoint` was added since the last registration.
    pending: Vec<Entity>,
    creatures_by_grid: HashMap<GridCoord, Vec<Entity>>,
    home_grids: HashMap<Entity, Vec<GridCoord>>,
    player_grids: HashMap<Entity, GridCoord>,
    active_grids: HashSet<GridCoord>,
}

impl GridIndex {
    pub(crate) fn preparation_entities(&self, map: InMap, position: &Position) -> Vec<Entity> {
        let grid = grid_of(map, position.x, position.z);
        let active = neighbourhoods(std::iter::once(&grid));
        let entities: HashSet<_> = active
            .iter()
            .flat_map(|grid| {
                self.creatures_by_grid
                    .get(grid)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect();
        entities.into_iter().collect()
    }

    fn is_active(&self, creature: Entity) -> bool {
        self.home_grids
            .get(&creature)
            .is_some_and(|grids| grids.iter().any(|grid| self.active_grids.contains(grid)))
    }
}

/// Grid of a Bevy-space position (WoW x, y = Bevy x, -z) on `map`.
fn grid_of(map: InMap, x: f32, z: f32) -> GridCoord {
    let (wow_x, wow_y, _) = bevy_to_wow(x, 0.0, z);
    (
        map,
        (wow_x / SIZE_OF_GRIDS).floor() as i32,
        (wow_y / SIZE_OF_GRIDS).floor() as i32,
    )
}

fn queue_new_creature(trigger: On<Add, SpawnPoint>, mut index: ResMut<GridIndex>) {
    index.pending.push(trigger.entity);
}

fn forget_creature(trigger: On<Remove, SpawnPoint>, mut index: ResMut<GridIndex>) {
    let creature = trigger.entity;
    index.pending.retain(|&pending| pending != creature);
    let Some(grids) = index.home_grids.remove(&creature) else {
        return;
    };
    for grid in grids {
        if let Some(creatures) = index.creatures_by_grid.get_mut(&grid) {
            creatures.retain(|&other| other != creature);
        }
    }
}

type CreatureHomes<'w, 's> = Query<
    'w,
    's,
    (
        &'static SpawnPoint,
        Option<&'static WaypointState>,
        Option<&'static FormationMember>,
        &'static InMap,
    ),
>;

/// Index newly spawned creatures by their home grids; those with no player
/// nearby start dormant.
fn register_creatures(
    mut commands: Commands,
    mut index: ResMut<GridIndex>,
    creatures: CreatureHomes,
    world_data: Option<Res<WorldData>>,
) {
    for creature in std::mem::take(&mut index.pending) {
        let Ok(home) = creatures.get(creature) else {
            continue;
        };
        let grids = home_grids(home, &creatures, world_data.as_deref());
        for &grid in &grids {
            index
                .creatures_by_grid
                .entry(grid)
                .or_default()
                .push(creature);
        }
        index.home_grids.insert(creature, grids);
        if !index.is_active(creature) {
            commands.entity(creature).insert(GridDormant);
        }
    }
}

fn home_grids(
    (spawn, waypoints, member, &map): (
        &SpawnPoint,
        Option<&WaypointState>,
        Option<&FormationMember>,
        &InMap,
    ),
    creatures: &CreatureHomes,
    world_data: Option<&WorldData>,
) -> Vec<GridCoord> {
    let mut grids = vec![grid_of(map, spawn.x, spawn.z)];
    let leader = member.and_then(|member| creatures.get(member.leader).ok());
    if let Some((leader_spawn, _, _, _)) = leader {
        grids.push(grid_of(map, leader_spawn.x, leader_spawn.z));
    }
    let paths = [waypoints, leader.and_then(|(_, path, _, _)| path)];
    for path in paths.into_iter().flatten() {
        if let Some(path) = world_data.and_then(|data| data.waypoints.get(&path.path_id)) {
            grids.extend(path_grids(map, path));
        }
    }
    grids.sort_unstable();
    grids.dedup();
    grids
}

/// Every grid a closed patrol path crosses, sampled at a quarter grid along
/// each leg (the last point walks back to the first).
fn path_grids(map: InMap, path: &crate::world_data::WaypointPath) -> Vec<GridCoord> {
    let points: Vec<(f32, f32)> = path
        .points
        .iter()
        .map(|point| {
            let (x, _, z) = wow_to_bevy(point.position_x, point.position_y, point.position_z);
            (x, z)
        })
        .collect();
    let mut grids = Vec::new();
    for (i, &(x0, z0)) in points.iter().enumerate() {
        let (x1, z1) = points[(i + 1) % points.len()];
        let steps = ((x1 - x0).hypot(z1 - z0) / (SIZE_OF_GRIDS / 4.0))
            .ceil()
            .max(1.0) as usize;
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            grids.push(grid_of(map, x0 + (x1 - x0) * t, z0 + (z1 - z0) * t));
        }
    }
    grids
}

/// Recompute the active grids when a player changes grid, enters or leaves
/// the world, and toggle `GridDormant` only on creatures whose activity changed.
fn update_grid_activity(
    mut commands: Commands,
    mut index: ResMut<GridIndex>,
    players: Query<(Entity, &Position, &InMap), With<Player>>,
) {
    let player_grids: HashMap<Entity, GridCoord> = players
        .iter()
        .map(|(player, position, &map)| (player, grid_of(map, position.x, position.z)))
        .collect();
    if player_grids == index.player_grids {
        return;
    }
    let active = neighbourhoods(player_grids.values());
    let woken: Vec<GridCoord> = active.difference(&index.active_grids).copied().collect();
    let slept: Vec<GridCoord> = index.active_grids.difference(&active).copied().collect();
    index.player_grids = player_grids;
    index.active_grids = active;
    for grid in woken {
        for &creature in index.creatures_by_grid.get(&grid).into_iter().flatten() {
            commands.entity(creature).remove::<GridDormant>();
        }
    }
    for grid in slept {
        for &creature in index.creatures_by_grid.get(&grid).into_iter().flatten() {
            if !index.is_active(creature) {
                commands.entity(creature).insert(GridDormant);
            }
        }
    }
}

/// The 3x3 grids around each player grid.
fn neighbourhoods<'a>(player_grids: impl Iterator<Item = &'a GridCoord>) -> HashSet<GridCoord> {
    let mut active = HashSet::new();
    for &(map, gx, gy) in player_grids {
        for dx in -1..=1 {
            for dy in -1..=1 {
                active.insert((map, gx + dx, gy + dy));
            }
        }
    }
    active
}

#[cfg(test)]
#[path = "grid_activity_tests.rs"]
mod tests;
