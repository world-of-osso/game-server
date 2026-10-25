/// The map copy an entity is in.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InMap {
    pub map_id: u32,
    pub instance_id: u32,
}

impl InMap {
    /// A world map (continent): its only copy is instance 0.
    pub const fn continent(map_id: u32) -> Self {
        Self {
            map_id,
            instance_id: 0,
        }
    }
}

/// A point on a map in WoW coordinates, orientation in radians (TrinityCore
/// `WorldLocation`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldLocation {
    pub map_id: u32,
    pub position: Vec3,
    pub orientation: f32,
}

impl WorldLocation {
    /// The server (Bevy-space) position of this location.
    pub fn bevy_position(&self) -> Position {
        let (x, y, z) =
            crate::terrain_height::wow_to_bevy(self.position.x, self.position.y, self.position.z);
        Position { x, y, z }
    }
}


/// `Player::m_dungeonDifficulty`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DungeonDifficulty(pub u32);

impl Default for DungeonDifficulty {
    fn default() -> Self {
        Self(DIFFICULTY_NORMAL)
    }
}


/// The wall clock of instance locks and auction expiry (`GameTime::GetSystemTime`), shifted by
/// `offset_secs` (`game-server-admin clock-advance`) or pinned by tests.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ServerClock {
    pub offset_secs: i64,
    pub fixed_unix: Option<i64>,
}

impl ServerClock {
    pub fn unix(&self) -> i64 {
        let system = self.fixed_unix.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time after 1970")
                .as_secs() as i64
        });
        system + self.offset_secs
    }
}


#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, bitcode::Encode, bitcode::Decode)]
pub struct RaidDifficulty {
    pub primary: u32,
    pub legacy: u32,
}
impl Default for RaidDifficulty {
    fn default() -> Self {
        Self {
            primary: 14,
            legacy: 3,
        }
    }
}


