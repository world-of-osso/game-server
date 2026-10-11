//! Authored Retail gathering metadata; no name-based classification or synthetic IDs.
use bevy::prelude::*;
use rusqlite::Connection;
use std::collections::HashMap;

pub(crate) const SKINNING: u32 = 393;
pub(crate) const OPEN_LOCK: u32 = 33;
pub(crate) const SKIN: u32 = 95;
const TDB_LOOT_OFFSET: u32 = 20_000_000;
/// TC Player::GetProfessionSkillForExp, BASE_PARENT_TIER_INDEX.
const BASE_PARENT_TIER: i32 = 4;
const CURRENT_EXPANSION: i32 = 11;
const SKIN_WITH_OTHER_PROFESSION: u32 = 0x100 | 0x200 | 0x8000;

#[derive(Clone, Debug)]
pub(crate) struct Ability {
    pub spell: u32,
    pub effect: u32,
    pub lock_type: u32,
    pub parent: u32,
    pub skill: u32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LockRequirement {
    pub lock_type: u32,
    pub rank: u16,
}

#[derive(Clone, Debug)]
pub(crate) struct NodeTemplate {
    pub lock: u32,
    pub loot: u32,
    pub low: u16,
    pub high: u16,
    pub despawn_delay: f32,
    pub max_loots: u32,
    pub no_combat: bool,
    pub require_los: bool,
    pub condition: u32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SkinTemplate {
    pub loot: u32,
    pub skill: u32,
    pub spell: u32,
    pub elite: bool,
}

#[derive(Resource, Default)]
pub(crate) struct GatheringData {
    pub nodes: HashMap<u32, NodeTemplate>,
    pub locks: HashMap<u32, Vec<LockRequirement>>,
    pub abilities: Vec<Ability>,
    pub skins: HashMap<(u32, u32), SkinTemplate>,
    pub respawn_seconds: HashMap<u32, f32>,
}

pub(crate) fn load_data(conn: &Connection) -> rusqlite::Result<GatheringData> {
    let mut data = GatheringData::default();
    let mut statement = conn.prepare("SELECT e.SpellID,e.Effect,e.EffectMiscValue_0,a.SkillLine,CASE WHEN a.SkillupSkillLineID!=0 THEN a.SkillupSkillLineID ELSE a.SkillLine END FROM spell_effect e JOIN skill_line_ability a ON a.Spell=e.SpellID WHERE e.Effect IN (33,95) AND a.SkillLine IN (182,186,393) ORDER BY a.ID")?;
    data.abilities = statement
        .query_map([], |row| {
            Ok(Ability {
                spell: row.get(0)?,
                effect: row.get(1)?,
                lock_type: row.get(2)?,
                parent: row.get(3)?,
                skill: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    load_locks(conn, &mut data)?;
    load_nodes(conn, &mut data)?;
    load_skins(conn, &mut data)?;
    let mut statement =
        conn.prepare("SELECT guid,spawntimesecs FROM content_gameobject WHERE spawntimesecs>0")?;
    data.respawn_seconds = statement
        .query_map([], |row| Ok((row.get(0)?, row.get::<_, f64>(1)? as f32)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(data)
}

fn load_locks(conn: &Connection, data: &mut GatheringData) -> rusqlite::Result<()> {
    let columns = (0..8)
        .map(|slot| format!("Type_{slot},Index_{slot},Skill_{slot}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut statement = conn.prepare(&format!(
        "SELECT ID,{columns} FROM gathering_lock ORDER BY ID"
    ))?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let mut alternatives = Vec::new();
        for slot in 0..8 {
            // TC LOCK_KEY_SKILL; item/spell keys are not gathering requirements.
            if row.get::<_, u32>(1 + slot * 3)? == 2 {
                alternatives.push(LockRequirement {
                    lock_type: row.get(2 + slot * 3)?,
                    rank: row.get(3 + slot * 3)?,
                });
            }
        }
        data.locks.insert(row.get(0)?, alternatives);
    }
    Ok(())
}

fn load_nodes(conn: &Connection, data: &mut GatheringData) -> rusqlite::Result<()> {
    let mut statement = conn.prepare("SELECT entry,Data0,Data1,Data3,Data4,Data5,Data6,Data8,Data11,Data18 FROM content_gameobject_template WHERE type=50 ORDER BY entry")?;
    data.nodes = statement
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                NodeTemplate {
                    lock: row.get(1)?,
                    loot: row.get(2)?,
                    no_combat: row.get::<_, u32>(3)? != 0,
                    low: row.get(4)?,
                    high: row.get(5)?,
                    despawn_delay: row.get::<_, f64>(6)? as f32,
                    require_los: row.get::<_, u32>(7)? != 0,
                    condition: row.get(8)?,
                    max_loots: row.get(9)?,
                },
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(())
}

fn load_skins(conn: &Connection, data: &mut GatheringData) -> rusqlite::Result<()> {
    let mut statement = conn.prepare("SELECT d.Entry,d.DifficultyID,d.SkinLootID,s.ID,t.Classification FROM tdb_creature_template_difficulty d JOIN tdb_creature_template t ON t.entry=d.Entry JOIN content_tuning c ON c.ID=d.ContentTuningID JOIN skill_line s ON s.ParentSkillLineID=393 AND s.ParentTierIndex-?1=CASE WHEN c.ExpansionID<0 THEN ?2 ELSE c.ExpansionID END WHERE d.SkinLootID!=0 AND (d.TypeFlags & ?3)=0 ORDER BY d.Entry,d.DifficultyID")?;
    let rows = statement.query_map(
        [
            BASE_PARENT_TIER,
            CURRENT_EXPANSION,
            SKIN_WITH_OTHER_PROFESSION as i32,
        ],
        |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, u32>(3)?,
                row.get::<_, u8>(4)?,
            ))
        },
    )?;
    for row in rows {
        let (entry, difficulty, loot, skill, rank) = row?;
        if let Some(ability) = data.abilities.iter().find(|ability| {
            ability.effect == SKIN && ability.parent == SKINNING && ability.skill == SKINNING
        }) {
            data.skins.insert(
                (entry, difficulty),
                SkinTemplate {
                    loot: loot + TDB_LOOT_OFFSET,
                    skill,
                    spell: ability.spell,
                    elite: matches!(rank, 1..=3),
                },
            );
        }
    }
    Ok(())
}
