//! Real-DB tests against data/world.db; each test skips when the DB is absent.
use std::path::PathBuf;

use rusqlite::OpenFlags;

use super::*;

/// Stormwind bank Guild Vault (TrinityCore entry 205107, guid 220030, display 7607); Retail
/// Stormwind has one entry per vault, 205104-205107.
const STORMWIND_VAULT: u32 = 205107;
const STORMWIND_VAULTS: std::ops::RangeInclusive<u32> = 205104..=205107;

fn spawns() -> Option<GameObjectSpawns> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
    if !path.exists() {
        eprintln!("skipping: {} not found", path.display());
        return None;
    }
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    Some(load_game_object_spawns(&conn).unwrap())
}

#[test]
fn eastern_kingdoms_guild_vaults_load_with_retail_display_boxes() {
    let Some(spawns) = spawns() else { return };
    // Stormwind 4, Ironforge 2, Undercity 4, Booty Bay 2 (TrinityCore map 0 type 34).
    let vaults = spawns
        .0
        .iter()
        .filter(|s| s.map_id == 0 && s.info.go_type == 34)
        .count();
    assert_eq!(vaults, 12);
    let stormwind: Vec<_> = spawns
        .0
        .iter()
        .filter(|s| STORMWIND_VAULTS.contains(&s.info.entry))
        .collect();
    assert_eq!(stormwind.len(), 4);
    let vault = stormwind
        .iter()
        .find(|s| s.info.entry == STORMWIND_VAULT)
        .unwrap();
    assert_eq!(vault.info.name, "Guild Vault");
    assert_eq!(vault.info.display_id, 7607);
    // Retail GameObjectDisplayInfo 7607 GeoBox.
    assert!((vault.bounds.min.y - -1.9534).abs() < 1e-3);
    assert!((vault.bounds.max.z - 5.8225).abs() < 1e-3);
}

#[test]
fn reach_is_the_model_box_grown_by_five_yards() {
    let bounds = GameObjectBounds {
        min: Vec3::new(-0.7, -1.95, 0.0),
        max: Vec3::new(1.1, 1.93, 5.8),
        scale: 1.0,
        orientation: 0.0,
    };
    let origin = Vec3::new(-8934.91, 618.27, 100.59);
    // 6 yd in front of the box face at x = 1.1: inside 1.1 + 5.
    assert!(bounds.reaches(origin, origin + Vec3::new(6.0, 0.0, 0.0), 5.0));
    assert!(!bounds.reaches(origin, origin + Vec3::new(6.2, 0.0, 0.0), 5.0));
    // Turned a quarter: the long y extent now lies along world x.
    let turned = GameObjectBounds {
        orientation: FRAC_PI_2,
        ..bounds
    };
    assert!(turned.reaches(origin, origin + Vec3::new(6.9, 0.0, 0.0), 5.0));
    assert!(!bounds.reaches(origin, origin + Vec3::new(0.0, 7.0, 0.0), 5.0));
}

#[test]
fn bundle_places_the_vault_in_server_space() {
    let Some(spawns) = spawns() else { return };
    let vault = &spawns.0[0];
    let (position, _, info, _) = game_object_bundle(vault);
    assert_eq!(wow_position(&position), vault.position);
    assert_eq!(info, vault.info);
}

const GOLDSHIRE_MAILBOX: u32 = 20_209_585;

#[test]
fn eastern_kingdoms_mailboxes_spawn_with_retail_display_boxes() {
    let Some(spawns) = spawns() else { return };
    // TrinityCore map 0 base-phase type 19 rows with a Retail GameObjectDisplayInfo box.
    let mailboxes: Vec<_> = spawns
        .0
        .iter()
        .filter(|s| s.map_id == 0 && s.info.go_type == 19)
        .collect();
    assert_eq!(mailboxes.len(), 75);
    // The Lion's Pride Inn mailbox in Goldshire (TrinityCore guid 209585 + 20,000,000).
    let goldshire = mailboxes
        .iter()
        .find(|s| s.guid == GOLDSHIRE_MAILBOX)
        .unwrap();
    assert_eq!(goldshire.info.name, "Mailbox");
    assert_eq!(goldshire.info.display_id, 1907);
    assert!((goldshire.position.x - -9455.99).abs() < 1e-2);
    assert!((goldshire.bounds.max.z - 3.0802).abs() < 1e-3);
}
