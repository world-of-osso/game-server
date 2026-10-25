//! Original immutable map fixture and literal inputs, shared with retained native tests.
use super::instances::InstanceEntrant;
use super::{MapStore, instances, store};
use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use std::sync::OnceLock;

pub fn map_store() -> Option<&'static MapStore> {
    static STORE: OnceLock<Option<MapStore>> = OnceLock::new();
    STORE
        .get_or_init(|| {
            let path = crate::test_data::world_db_path();
            let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .expect("open world.db");
            Some(
                store::load_map_store(&conn)
                    .expect("map tables in world.db; run scripts/import-azerothcore.sh"),
            )
        })
        .as_ref()
}

/// A Normal-difficulty entrant; a grouped one's leader is character 1.
pub fn entrant(character_id: u64, group: Option<u64>) -> InstanceEntrant {
    InstanceEntrant {
        character_id,
        group,
        leader: if group.is_some() { 1 } else { character_id },
        difficulty_id: store::DIFFICULTY_NORMAL,
        ..Default::default()
    }
}

/// Elapsed seconds `elapsed` on Sunday noon.
pub fn at(elapsed: f64) -> instances::Now {
    instances::Now {
        elapsed,
        unix: SUNDAY_NOON,
    }
}

pub fn entrant_at(character_id: u64, difficulty_id: u32) -> InstanceEntrant {
    InstanceEntrant {
        difficulty_id,
        ..entrant(character_id, None)
    }
}

pub fn close(a: Vec3, b: Vec3) -> bool {
    a.distance(b) < 0.01
}

pub const STOCKADE: u32 = 34;

/// `areatrigger_teleport` 101 "Stormwind Jail Entrance Target": its AreaTrigger box
/// center in Stormwind City.
pub const STOCKADE_ENTRANCE_TRIGGER: Vec3 = Vec3::new(-8761.85, 848.557, 87.8052);

/// `world_safe_locs` 3599 inside the Stockade.
pub const STOCKADE_ARRIVAL: Vec3 = Vec3::new(56.6821, 0.62376, -19.2691);

/// `world_safe_locs` 3618 "Stormwind Low Prison Exit Target".
pub const STOCKADE_EXIT_TARGET: Vec3 = Vec3::new(-8766.11, 845.499, 87.9952);

/// `world_safe_locs` 1854 "Stormwind Stockade, GY", the graveyard of zone 717.
pub const STOCKADE_GRAVEYARD: Vec3 = Vec3::new(-8786.88, 829.083, 97.6504);

pub const GRIM_BATOL: u32 = 670;

/// `Difficulty.db2` Heroic and (five-player) Mythic.
pub const DIFFICULTY_HEROIC: u32 = 2;

pub const DIFFICULTY_MYTHIC: u32 = 23;

/// 2026-09-27 15:00 UTC: the daily reset after `SUNDAY_NOON`.
pub const SUNDAY_RESET: i64 = 1_790_521_200;

/// 2026-09-27 12:00 UTC (a Sunday): the instance-lock clock of these tests.
pub const SUNDAY_NOON: i64 = 1_790_510_400;

/// The Culling of Stratholme (595) Normal: MapDifficulty 91's conditions (40817, 43808)
/// carry no ContentTuning, so no level is required.
pub const CULLING_OF_STRATHOLME: u32 = 595;
