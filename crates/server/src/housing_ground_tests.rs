//! Opt-in housing geometry proof on an explicitly private local-CASC cache.
use super::*;

fn housing_cache() -> PathBuf {
    std::env::var_os(GROUND_DIR_ENV)
        .map(PathBuf::from)
        .expect("set GAME_SERVER_GROUND_DIR to the private housing ground cache")
}

#[test]
#[ignore = "requires locally extracted Retail housing map geometry"]
fn housing_ground_closure_keeps_all_authored_tiles_and_wmo_dependencies() {
    let cache = housing_cache();
    let ground = GroundMap::new(
        &cache,
        HashMap::from([(2735, 5660542), (2736, 5660543), (2783, 5890928)]),
    );
    for (map, wdt, count) in [
        (2735, 5660542, 210),
        (2736, 5660543, 225),
        (2783, 5890928, 25),
    ] {
        let missing = ground.missing_files(map);
        assert!(missing.is_empty(), "map{map} missing geometry: {missing:?}");
        let bytes = read_client_file(&cache, "terrain", wdt, "wdt").unwrap();
        let tiles = parse_maid(&bytes).unwrap();
        let active: Vec<_> = tiles.iter().filter(|tile| tile.root != 0).collect();
        assert_eq!(active.len(), count, "actual WDT tiles on map{map}");
        for tile in active {
            load_terrain(&cache, tile.root).expect("authored root has parseable real height data");
        }
    }
}

#[test]
#[ignore = "requires locally extracted Retail housing map geometry"]
fn housing_ground_home_interior_authored_entry_has_real_support() {
    let mut ground = GroundMap::new(&housing_cache(), HashMap::from([(2783, 5890928)]));
    // Authored NeighborhoodMap7 Home Interior entry, not a fabricated room spawn.
    let (x, y, z) = crate::terrain_height::wow_to_bevy(-1000.0, -1000.0, 0.1);
    let probe = ground.ground_at(2783, Vec3::new(x, y, z));
    println!("HOUSING_INTERIOR_ENTRY {probe:?}");
    assert!(matches!(probe, GroundProbe::Supported(height) if height.is_finite()));
}

#[test]
#[ignore = "requires locally extracted Retail housing map geometry"]
fn housing_ground_founders_authored_entry_cornerstone_and_plot_teleport_are_supported() {
    let mut ground = GroundMap::new(&housing_cache(), HashMap::from([(2735, 5660542)]));
    // NeighborhoodMap1, actual GameObjects527268 and NeighborhoodPlot417 teleport.
    for [x, y, z] in [
        [3807.76, -160.427, 194.111],
        [2941.2317, -214.5868, 113.15639],
        [2939.3438, -212.2554, 113.321],
    ] {
        let (bx, by, bz) = crate::terrain_height::wow_to_bevy(x, y, z);
        let GroundProbe::Supported(height) = ground.ground_at(2735, Vec3::new(bx, by, bz)) else {
            panic!("authored point({x},{y},{z}) has no authentic floor");
        };
        assert!(height.is_finite());
        // EntryPosition is an authored spawn above its actual floor, not a terrain vertex.
        // Supported is the production ground rule; do not invent a spawn-height tolerance.
        println!("HOUSING_FLOOR map2735 point({x},{y},{z}) height{height}");
    }
}
