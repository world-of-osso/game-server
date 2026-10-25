fn map_store() -> Option<&'static MapStore> {
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

fn entrant(character_id: u64, group: Option<u64>) -> InstanceEntrant {
    InstanceEntrant {
        character_id,
        group,
        leader: if group.is_some() { 1 } else { character_id },
        difficulty_id: store::DIFFICULTY_NORMAL,
        ..Default::default()
    }
}

fn at(elapsed: f64) -> instances::Now {
    instances::Now {
        elapsed,
        unix: SUNDAY_NOON,
    }
}

fn entrant_at(character_id: u64, difficulty_id: u32) -> InstanceEntrant {
    InstanceEntrant {
        difficulty_id,
        ..entrant(character_id, None)
    }
}

fn close(a: Vec3, b: Vec3) -> bool {
    a.distance(b) < 0.01
}

const STOCKADE: u32 = 34;

const STOCKADE_ENTRANCE_TRIGGER: Vec3 = Vec3::new(-8761.85, 848.557, 87.8052);

const STOCKADE_ARRIVAL: Vec3 = Vec3::new(56.6821, 0.62376, -19.2691);

const STOCKADE_EXIT_TARGET: Vec3 = Vec3::new(-8766.11, 845.499, 87.9952);

const STOCKADE_GRAVEYARD: Vec3 = Vec3::new(-8786.88, 829.083, 97.6504);

const GRIM_BATOL: u32 = 670;

const DIFFICULTY_HEROIC: u32 = 2;

const DIFFICULTY_MYTHIC: u32 = 23;

const SUNDAY_RESET: i64 = 1_790_521_200;

const SUNDAY_NOON: i64 = 1_790_510_400;

const CULLING_OF_STRATHOLME: u32 = 595;

