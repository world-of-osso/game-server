#![cfg(feature = "bake")]
//! Legacy local-CASC WDT placement and terrain-less map regressions.
use los::bake::{Baker, adt};
use std::path::PathBuf;

fn data() -> PathBuf {
    PathBuf::from(std::env::var_os("GAME_SERVER_GROUND_DIR").expect("local extracted data"))
}

#[test]
fn legacy_global_wmo_bakes_real_blacksmith_doodad_floors() {
    let mut baker = Baker::for_m2_floors(&data());
    let geometry = baker.collect_global(952203).unwrap().unwrap();
    assert!(
        geometry
            .iter()
            .any(|(_, geometry)| !geometry.triangles.is_empty())
    );
    assert!(baker.problems.is_empty(), "{:?}", baker.problems);
}

#[test]
fn legacy_empty_main_without_maid_has_no_terrain() {
    let mut baker = Baker::for_m2_floors(&data());
    assert!(baker.collect_global(953470).unwrap().is_none());
    let tiles = baker.map_tiles(953470).unwrap();
    assert_eq!(tiles.len(), 4096);
    assert!(tiles.iter().all(|tile| tile.root == 0 && tile.obj0 == 0));
}

#[test]
fn active_main_without_maid_remains_an_asset_gap() {
    let mut wdt = std::fs::read(data().join("terrain/953470.wdt")).unwrap();
    let offset = wdt.windows(4).position(|tag| tag == b"NIAM").unwrap();
    wdt[offset + 8..offset + 12].copy_from_slice(&1u32.to_le_bytes());
    assert!(adt::wdt_tiles(&wdt).err().unwrap().contains("MAID"));
}
