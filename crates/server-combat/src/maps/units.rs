//! Entities by map copy, so per-copy scans do not walk every loaded map.
use bevy::ecs::entity::{EntityHashMap, EntityHashSet};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use shared::components::Position;

use super::InMap;

/// Every entity with an `InMap`, grouped by that copy. Maintained by observers,
/// so it is exact at any point a system or command can observe.
#[derive(Resource, Default)]
pub struct MapUnits(HashMap<InMap, EntityHashSet>);

impl MapUnits {
    /// The entities currently in `map`, in no particular order.
    pub fn in_map(&self, map: InMap) -> impl Iterator<Item = Entity> + '_ {
        self.0.get(&map).into_iter().flatten().copied()
    }
}

/// Installs the index and observers once. Must run before any `InMap` is inserted.
pub(crate) fn install(app: &mut App) {
    if app.world().contains_resource::<MapUnits>() {
        return;
    }
    app.init_resource::<MapUnits>()
        .init_resource::<AreaGrid>()
        .init_resource::<crate::interest::InterestDirtyQueue>()
        .add_observer(index_unit)
        .add_observer(unindex_unit)
        .add_observer(index_position)
        .add_observer(unindex_position);
}

fn index_unit(
    trigger: On<Insert, InMap>,
    maps: Query<(&InMap, Option<&Position>)>,
    mut units: ResMut<MapUnits>,
    mut grid: ResMut<AreaGrid>,
) {
    if let Ok((map, position)) = maps.get(trigger.entity) {
        units.0.entry(*map).or_default().insert(trigger.entity);
        if let Some(position) = position {
            grid.place(trigger.entity, *map, position);
        }
    }
}

/// Fires with the old value still present: on replacement, removal and despawn.
fn unindex_unit(
    trigger: On<Discard, InMap>,
    maps: Query<&InMap>,
    mut units: ResMut<MapUnits>,
    mut grid: ResMut<AreaGrid>,
) {
    grid.remove(trigger.entity);
    let Ok(map) = maps.get(trigger.entity) else {
        return;
    };
    if let Some(members) = units.0.get_mut(map) {
        members.remove(&trigger.entity);
        if members.is_empty() {
            units.0.remove(map);
        }
    }
}

const AREA_CELL_SIZE: f32 = 64.0;
type AreaCell = (InMap, i32, i32);

#[derive(Resource, Default)]
struct AreaGrid {
    cells: HashMap<AreaCell, EntityHashSet>,
    membership: EntityHashMap<AreaCell>,
}

impl AreaGrid {
    fn remove(&mut self, entity: Entity) {
        if let Some(cell) = self.membership.remove(&entity)
            && let Some(members) = self.cells.get_mut(&cell)
        {
            members.remove(&entity);
            if members.is_empty() {
                self.cells.remove(&cell);
            }
        }
    }

    fn place(&mut self, entity: Entity, map: InMap, position: &Position) {
        let cell = (
            map,
            (position.x / AREA_CELL_SIZE).floor() as i32,
            (position.z / AREA_CELL_SIZE).floor() as i32,
        );
        if self.membership.get(&entity) == Some(&cell) {
            return;
        }
        self.remove(entity);
        self.membership.insert(entity, cell);
        self.cells.entry(cell).or_default().insert(entity);
    }
}

fn index_position(
    trigger: On<Insert, Position>,
    units: Query<(&InMap, &Position)>,
    mut grid: ResMut<AreaGrid>,
) {
    if let Ok((map, position)) = units.get(trigger.entity) {
        grid.place(trigger.entity, *map, position);
    }
}

fn unindex_position(trigger: On<Discard, Position>, mut grid: ResMut<AreaGrid>) {
    grid.remove(trigger.entity);
}

/// Synchronize notified relocations, then return candidates from intersecting cells.
pub(crate) fn nearby(world: &mut World, map: InMap, center: Vec3, radius: f32) -> Vec<Entity> {
    let relocated = world
        .resource::<crate::interest::InterestDirtyQueue>()
        .drain_area_relocated();
    for entity in relocated {
        let current = world
            .get::<InMap>(entity)
            .copied()
            .zip(world.get::<Position>(entity).copied());
        let mut grid = world.resource_mut::<AreaGrid>();
        if let Some((map, position)) = current {
            grid.place(entity, map, &position);
        } else {
            grid.remove(entity);
        }
    }
    if radius < 0.0 || !radius.is_finite() || !center.is_finite() {
        return Vec::new();
    }
    let min_x = ((center.x - radius) / AREA_CELL_SIZE).floor() as i32;
    let max_x = ((center.x + radius) / AREA_CELL_SIZE).floor() as i32;
    let min_z = ((center.z - radius) / AREA_CELL_SIZE).floor() as i32;
    let max_z = ((center.z + radius) / AREA_CELL_SIZE).floor() as i32;
    let grid = world.resource::<AreaGrid>();
    let cells = &grid.cells;
    (min_x..=max_x)
        .flat_map(|x| (min_z..=max_z).filter_map(move |z| cells.get(&(map, x, z))))
        .flatten()
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        install(&mut app);
        app
    }

    fn members(app: &App, map: InMap) -> Vec<Entity> {
        let mut found: Vec<_> = app.world().resource::<MapUnits>().in_map(map).collect();
        found.sort();
        found
    }

    const AZURE: InMap = InMap {
        map_id: 2515,
        instance_id: 1,
    };
    const OTHER_COPY: InMap = InMap {
        map_id: 2515,
        instance_id: 2,
    };
    const WORLD: InMap = InMap {
        map_id: 0,
        instance_id: 0,
    };

    #[test]
    fn follower_plugin_indexes_units_without_map_plugin() {
        let mut app = App::new();
        app.add_plugins(crate::followers::FollowerPlugin);
        let unit = app.world_mut().spawn(AZURE).id();
        assert_eq!(members(&app, AZURE), vec![unit]);
        app.world_mut().entity_mut(unit).insert(OTHER_COPY);
        assert!(members(&app, AZURE).is_empty());
        assert_eq!(members(&app, OTHER_COPY), vec![unit]);
        app.world_mut().despawn(unit);
        assert!(members(&app, OTHER_COPY).is_empty());
    }

    #[test]
    fn creature_ai_plugin_indexes_units_without_map_plugin() {
        let mut app = App::new();
        app.add_plugins(crate::creature_ai::CreatureAiPlugin);
        let unit = app.world_mut().spawn(AZURE).id();
        assert_eq!(members(&app, AZURE), vec![unit]);
        app.world_mut().entity_mut(unit).remove::<InMap>();
        assert!(members(&app, AZURE).is_empty());
    }

    #[test]
    fn spawned_units_are_listed_only_in_their_own_copy() {
        let mut app = app();
        let tender = app.world_mut().spawn(AZURE).id();
        let boss = app.world_mut().spawn(AZURE).id();
        let elsewhere = app.world_mut().spawn(OTHER_COPY).id();
        app.world_mut().spawn(WORLD);
        let mut expected = vec![tender, boss];
        expected.sort();
        assert_eq!(members(&app, AZURE), expected);
        assert_eq!(members(&app, OTHER_COPY), vec![elsewhere]);
    }

    #[test]
    fn transfer_moves_a_unit_between_copies() {
        let mut app = app();
        let player = app.world_mut().spawn(WORLD).id();
        app.world_mut().entity_mut(player).insert(AZURE);
        assert!(members(&app, WORLD).is_empty());
        assert_eq!(members(&app, AZURE), vec![player]);
        app.world_mut().entity_mut(player).insert(AZURE);
        assert_eq!(
            members(&app, AZURE),
            vec![player],
            "re-insert keeps one membership"
        );
    }

    #[test]
    fn removal_and_despawn_leave_the_copy() {
        let mut app = app();
        let removed = app.world_mut().spawn(AZURE).id();
        let despawned = app.world_mut().spawn(AZURE).id();
        let stays = app.world_mut().spawn(AZURE).id();
        app.world_mut().entity_mut(removed).remove::<InMap>();
        app.world_mut().despawn(despawned);
        assert_eq!(members(&app, AZURE), vec![stays]);
        app.world_mut().despawn(stays);
        assert!(members(&app, AZURE).is_empty());
    }
}
