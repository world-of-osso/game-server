use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::group::GroupRole;

use super::Profile;
use crate::maps::WorldLocation;

pub(crate) const FOLLOWER_DIFFICULTY: u32 = 205;

#[derive(Clone)]
pub(crate) struct Dungeon {
    pub id: u32,
    pub name: String,
    pub entrance: WorldLocation,
}

#[derive(Resource, Default)]
pub(crate) struct FollowerCatalog {
    pub profiles: Vec<Profile>,
    pub follower_ids: HashSet<u32>,
    pub dungeons: HashMap<u32, Dungeon>,
    pub(super) kits: Vec<super::kit::Entry>,
}

pub(crate) fn load_catalog(db: &Connection) -> rusqlite::Result<FollowerCatalog> {
    let mut ids = db.prepare("SELECT id FROM follower_lfg_dungeons WHERE difficulty_id=205")?;
    let follower_ids = ids
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<HashSet<u32>>>()?;
    let mut profiles = db.prepare("SELECT entry,name,class_id,spec_id,role,attack_spell,cooldown_spell,heal_spell,taunt_spell,form_spell,melee FROM follower_profiles ORDER BY role,entry")?;
    let profiles = profiles
        .query_map([], |row| {
            let role = match row.get::<_, u8>(4)? {
                0 => GroupRole::Tank,
                1 => GroupRole::Healer,
                2 => GroupRole::Dps,
                value => {
                    return Err(rusqlite::Error::IntegralValueOutOfRange(
                        4,
                        i64::from(value),
                    ));
                }
            };
            Ok(Profile {
                entry: row.get(0)?,
                name: row.get(1)?,
                class_id: row.get(2)?,
                spec_id: row.get(3)?,
                role,
                attack_spell: row.get(5)?,
                cooldown_spell: row.get(6)?,
                heal_spell: row.get(7)?,
                taunt_spell: row.get(8)?,
                form_spell: row.get(9)?,
                melee: row.get(10)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut query = db.prepare("SELECT d.id,d.name,d.map_id,l.LocX,l.LocY,l.LocZ,l.Facing FROM follower_lfg_dungeons d JOIN tdb_world_safe_locs l ON l.MapID=d.map_id WHERE d.difficulty_id=205 AND d.tanks=1 AND d.healers=1 AND d.damage=3 AND l.Comment LIKE '%Entrance%' AND EXISTS(SELECT 1 FROM content_creature c WHERE c.map=d.map_id AND (','||c.spawnDifficulties||',') LIKE '%,1,%') ORDER BY d.id,l.ID")?;
    let rows = query.query_map([], |row| {
        Ok(Dungeon {
            id: row.get(0)?,
            name: row.get(1)?,
            entrance: WorldLocation {
                map_id: row.get(2)?,
                position: Vec3::new(row.get(3)?, row.get(4)?, row.get(5)?),
                orientation: row.get::<_, f32>(6)?.to_radians(),
            },
        })
    })?;
    let mut dungeons = HashMap::new();
    for dungeon in rows {
        let dungeon = dungeon?;
        dungeons.entry(dungeon.id).or_insert(dungeon);
    }
    Ok(FollowerCatalog {
        profiles,
        follower_ids,
        dungeons,
        kits: super::kit::load(db)?,
    })
}

pub(super) fn load_startup(mut commands: Commands) {
    let db = Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open follower catalog world.db");
    match load_catalog(&db) {
        Ok(catalog) => {
            info!(
                "Follower bridge: {} sourced profiles, {} follower dungeons with Normal population and authored entrance",
                catalog.profiles.len(),
                catalog.dungeons.len()
            );
            commands.insert_resource(catalog);
        }
        Err(error) => warn!(
            "Follower queue unavailable: apply follower_import.py and follower_profiles.py SQL plus instance population: {error}"
        ),
    }
}
