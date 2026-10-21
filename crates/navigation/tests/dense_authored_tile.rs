//! Real authored collision regression for dense multi-level dungeon tiles.
#![cfg(feature = "bake")]

use navigation::{NavigationMap, bake::bake_collected, read_walkable_polygons};
use std::path::PathBuf;

#[test]
#[ignore = "requires FOLLOWER_NAV_DATA and FOLLOWER_NAV_WORLD_DB local assets"]
fn city_of_threads_dense_authored_tile_bakes_walkable_navigation() {
    let data = PathBuf::from(std::env::var_os("FOLLOWER_NAV_DATA").unwrap());
    let db = rusqlite::Connection::open_with_flags(
        std::env::var_os("FOLLOWER_NAV_WORLD_DB").unwrap(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let wdt = db
        .query_row("SELECT WdtFileDataID FROM map WHERE ID=2669", [], |row| {
            row.get(0)
        })
        .unwrap();
    let mut baker = los::bake::Baker::for_navigation(&data);
    let tiles = baker.map_tiles(wdt).unwrap();
    let tile = (31, 37);
    let (geometry, _) = baker.collect_tile(&tiles, tile).unwrap().unwrap();
    let water = baker
        .collect_water(tiles[(tile.1 * 64 + tile.0) as usize])
        .unwrap();
    assert!(
        baker.problems.is_empty(),
        "asset gaps: {:?}",
        baker.problems
    );
    let bytes = bake_collected(&geometry, &water, tile).expect(
        "complete authored City of Threads tile must bake without changing agent parameters",
    );
    assert!(!bytes.is_empty(), "authored walkable floors must survive");
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("31_37.nav"), bytes).unwrap();
    let polygons = read_walkable_polygons(dir.path()).unwrap();
    assert!(!polygons.is_empty());
    let mut nav = NavigationMap::open(dir.path()).unwrap();
    let polygon = &polygons[polygons.len() / 2];
    let center = polygon
        .iter()
        .copied()
        .map(glam::Vec3::from)
        .sum::<glam::Vec3>()
        / polygon.len() as f32;
    let path = nav
        .path(center, center)
        .expect("baked floor must be queryable");
    assert!(!path.is_empty());
}
