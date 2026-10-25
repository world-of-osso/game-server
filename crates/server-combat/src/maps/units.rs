//! Entities by map copy, so per-copy scans do not walk every loaded map.
use bevy::ecs::entity::EntityHashSet;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

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
        .add_observer(index_unit)
        .add_observer(unindex_unit);
}

fn index_unit(trigger: On<Insert, InMap>, maps: Query<&InMap>, mut units: ResMut<MapUnits>) {
    if let Ok(map) = maps.get(trigger.entity) {
        units.0.entry(*map).or_default().insert(trigger.entity);
    }
}

/// Fires with the old value still present: on replacement, removal and despawn.
fn unindex_unit(trigger: On<Discard, InMap>, maps: Query<&InMap>, mut units: ResMut<MapUnits>) {
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
