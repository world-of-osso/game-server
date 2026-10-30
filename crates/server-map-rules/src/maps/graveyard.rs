//! Graveyard choice for deaths in dungeons (TrinityCore `ObjectMgr::GetClosestGraveyardInZone`,
//! ObjectMgr.cpp:6868-6970).

use super::WorldLocation;
use super::store::MapStore;

/// The `graveyard_zone` graveyard of `zone_id` closest to `location`: one on the same
/// map (or the parent map) by 3D distance, else one on the dungeon's `CorpseMapID`
/// closest (2D) to its `Corpse` entrance point, else any other. Graveyard conditions
/// (team, phase) are not imported.
pub fn closest_graveyard_in_zone(
    store: &MapStore,
    location: &WorldLocation,
    zone_id: u32,
) -> Option<WorldLocation> {
    let map = store.map(location.map_id)?;
    let mut near: Option<(f32, WorldLocation)> = None;
    let mut entrance: Option<(f32, WorldLocation)> = None;
    let mut far = None;
    for safe_loc in store.graveyard_zones.get(&zone_id).into_iter().flatten() {
        let Some(&grave) = store.safe_locs.get(safe_loc) else {
            continue;
        };
        if grave.map_id != location.map_id && grave.map_id as i32 != map.parent_map_id {
            let corpse_map = map.corpse_map_id;
            if corpse_map < 0 || corpse_map as u32 != grave.map_id || map.corpse == (0.0, 0.0) {
                far = Some(grave);
                continue;
            }
            let dx = grave.position.x - map.corpse.0;
            let dy = grave.position.y - map.corpse.1;
            keep_closest(&mut entrance, dx * dx + dy * dy, grave);
        } else {
            let distance = grave.position.distance_squared(location.position);
            keep_closest(&mut near, distance, grave);
        }
    }
    near.or(entrance).map(|(_, grave)| grave).or(far)
}

/// The nearest zone graveyard whose safe location is on the current map.
pub fn closest_graveyard_on_map(
    store: &MapStore,
    location: &WorldLocation,
    zone_id: u32,
) -> Option<WorldLocation> {
    store
        .graveyard_zones
        .get(&zone_id)?
        .iter()
        .filter_map(|id| store.safe_locs.get(id))
        .filter(|grave| grave.map_id == location.map_id)
        .min_by(|a, b| {
            a.position
                .distance_squared(location.position)
                .total_cmp(&b.position.distance_squared(location.position))
        })
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Vec3;

    #[test]
    fn same_map_graveyard_is_selected_before_a_closer_foreign_location() {
        let mut store = MapStore::continents();
        let location = WorldLocation {
            map_id: 0,
            position: Vec3::ZERO,
            orientation: 0.0,
        };
        let outside = WorldLocation {
            map_id: 1,
            position: Vec3::ZERO,
            orientation: 0.0,
        };
        let inside = WorldLocation {
            map_id: 0,
            position: Vec3::new(30.0, 0.0, 0.0),
            orientation: 0.0,
        };
        store.safe_locs.insert(1, outside);
        store.safe_locs.insert(2, inside);
        store.graveyard_zones.insert(7, vec![1, 2]);
        assert_eq!(closest_graveyard_on_map(&store, &location, 7), Some(inside));
    }
}

fn keep_closest(best: &mut Option<(f32, WorldLocation)>, distance: f32, grave: WorldLocation) {
    if best.is_none_or(|(best_distance, _)| distance < best_distance) {
        *best = Some((distance, grave));
    }
}
