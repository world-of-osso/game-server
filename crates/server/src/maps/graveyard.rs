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

fn keep_closest(best: &mut Option<(f32, WorldLocation)>, distance: f32, grave: WorldLocation) {
    if best.is_none_or(|(best_distance, _)| distance < best_distance) {
        *best = Some((distance, grave));
    }
}
