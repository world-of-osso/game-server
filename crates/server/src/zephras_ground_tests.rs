//! Opt-in real Forever build acceptance. Set GAME_SERVER_GROUND_DIR to its cache.
use super::*;

const ZEPHRAS_MAP: u32 = 2991;
const ZEPHRAS_WDT: u32 = 7_198_644;
const ZEPHRAS_TILE_COUNT: usize = 72;

fn zephras_cache() -> PathBuf {
    std::env::var_os(GROUND_DIR_ENV)
        .map(PathBuf::from)
        .expect("set GAME_SERVER_GROUND_DIR to the extracted Forever cache")
}

#[test]
#[ignore = "requires extracted Forever build 1.60.1.70205 ground assets"]
fn skyborne_zephras_approved_starts_have_supported_ground() {
    let mut ground = GroundMap::new(
        &zephras_cache(),
        HashMap::from([(ZEPHRAS_MAP, ZEPHRAS_WDT)]),
    );
    for race in [95, 96] {
        let start = crate::player_start::approved_start(race).unwrap();
        assert_eq!(start.map_id, ZEPHRAS_MAP);
        let (x, y, z) = crate::terrain_height::wow_to_bevy(start.x, start.y, start.z);
        let GroundProbe::Supported(height) = ground.ground_at(start.map_id, Vec3::new(x, y, z))
        else {
            panic!("race {race}: approved start has no ground support");
        };
        assert!((height - start.z).abs() < 0.01);
    }
}

fn tile_center(row: usize, col: usize) -> (f32, f32) {
    let x = (32.0 - col as f32 - 0.5) * TILE_SIZE;
    let y = (32.0 - row as f32 - 0.5) * TILE_SIZE;
    (x, y)
}

#[test]
#[ignore = "requires extracted Forever build 1.60.1.70205 ground assets"]
fn zephras_ground_centers_and_height_report() {
    let cache = zephras_cache();
    let wdt = read_client_file(&cache, "terrain", ZEPHRAS_WDT, "wdt").unwrap();
    let tiles = parse_maid(&wdt).unwrap();
    let mut ground = GroundMap::new(&cache, HashMap::from([(ZEPHRAS_MAP, ZEPHRAS_WDT)]));
    let mut records = Vec::new();
    let mut supported = 0;
    for (index, files) in tiles
        .iter()
        .enumerate()
        .filter(|(_, files)| files.root != 0)
    {
        let (row, col) = (index % MAP_TILES, index / MAP_TILES);
        let (x, y) = tile_center(row, col);
        let mut record = serde_json::json!({"row":row, "col":col, "root_fdid":files.root,
            "obj0_fdid":files.obj0, "x":x, "y":y, "z":null, "status":"missing_root"});
        if cache.join(format!("terrain/{}.adt", files.root)).is_file() {
            let (terrain, _) = load_terrain(&cache, files.root).unwrap();
            let center_feet = Vec3::new(x, 0.0, -y);
            let minimum = ground
                .min_height(ZEPHRAS_MAP, center_feet)
                .expect("minimum height");
            assert!(minimum.is_finite());
            record["min_height"] = serde_json::json!(minimum);
            if let Some(liquid) = ground.liquid_at(ZEPHRAS_MAP, center_feet) {
                assert!(liquid.level.is_finite() && liquid.depth_level.is_finite());
                record["liquid_level"] = serde_json::json!(liquid.level);
            }
            let Some(height) = terrain.height_at(x, y) else {
                record["status"] = serde_json::json!("terrain_hole");
                records.push(record);
                continue;
            };
            assert!(height.is_finite(), "tile {row},{col}: {height}");
            let feet = Vec3::new(x, height, -y);
            assert_eq!(bevy_to_tile(feet.x, feet.z), (row as u32, col as u32));
            let GroundProbe::Supported(surface) = ground.ground_at(ZEPHRAS_MAP, feet) else {
                panic!("tile {row},{col}: no center support");
            };
            assert!(surface.is_finite());
            record["above_min_height"] = serde_json::json!(height >= minimum);
            record["z"] = serde_json::json!(height);
            record["ground_z"] = serde_json::json!(surface);
            record["min_height"] = serde_json::json!(minimum);
            record["status"] = serde_json::json!("supported");
            supported += 1;
        } else {
            assert_eq!(
                ground.ground_at(ZEPHRAS_MAP, Vec3::new(x, 0.0, -y)),
                GroundProbe::Unloaded
            );
        }
        records.push(record);
    }
    assert_eq!(records.len(), ZEPHRAS_TILE_COUNT);
    assert!(supported > 0);
    for root in [7_199_994, 7_199_999, 7_200_004] {
        let sample = records
            .iter()
            .find(|record| record["root_fdid"] == root)
            .unwrap();
        assert_eq!(sample["status"], "supported", "sample tile: {sample}");
    }
    println!(
        "ZEPHRAS_HEIGHTS={}",
        serde_json::json!({"map_id":ZEPHRAS_MAP,
        "build":"1.60.1.70205", "coordinates":"WoW X,Y,Z (Z up)", "tiles":records})
    );
}

#[test]
#[ignore = "requires extracted Forever build 1.60.1.70205 ground assets"]
fn zephras_ground_liquid_matches_authored_surface() {
    let cache = zephras_cache();
    let mut ground = GroundMap::new(&cache, HashMap::from([(ZEPHRAS_MAP, ZEPHRAS_WDT)]));
    // First MH2O layer of root 7199999, two yards inside MCNK (0,0).
    let feet = Vec3::new(3198.0, 756.0, -1598.0);
    let liquid = ground.liquid_at(ZEPHRAS_MAP, feet).expect("sample water");
    assert!((liquid.level - 756.5897).abs() < 0.001);
    assert!(liquid.depth_level.is_finite() && liquid.depth_level < liquid.level);
    assert_eq!(liquid.liquid_type, 1251);
}

#[test]
#[ignore = "requires extracted Forever build 1.60.1.70205 ground assets"]
fn zephras_sample_placements_and_wmo_groups_parse() {
    let cache = zephras_cache();
    let placements = load_wmo_placements(&cache, 7_200_000).unwrap();
    let ids: HashSet<_> = placements.iter().map(|placement| placement.fdid).collect();
    assert_eq!(
        ids,
        HashSet::from([333_477, 7_704_156, 7_704_158, 7_749_471])
    );
    for root in [333_477, 7_704_156, 7_704_158, 7_749_471] {
        let groups = load_wmo_groups(&cache, root).unwrap();
        assert!(!groups.is_empty(), "WMO {root}: no collision groups");
        let collision = WmoCollision::new(Affine3A::IDENTITY, groups);
        let mut hits = Vec::new();
        for group in collision.groups() {
            for triangle in group.collidable_triangles() {
                let center = (triangle[0] + triangle[1] + triangle[2]) / 3.0;
                collision.floor_heights(
                    center.x,
                    center.z,
                    center.y + 1.0,
                    center.y - 1.0,
                    &mut hits,
                );
            }
        }
        assert!(!hits.is_empty(), "WMO {root}: no walkable floor hits");
        assert!(hits.iter().all(|height| height.is_finite()));
        println!(
            "Zephras WMO {root}: {} parsed groups, {} floor hits",
            collision.groups().len(),
            hits.len()
        );
    }
}
