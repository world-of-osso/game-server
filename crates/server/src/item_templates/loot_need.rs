//! Read-only Need eligibility metadata. TrinityCore a352b1fa
//! Player::CanRollNeedForItem, ChrClasses.ArmorTypeMask and EffectProficiency.
use rusqlite::Connection;
use std::collections::HashMap;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct LootNeedMetadata {
    pub armor_masks: HashMap<u8, u32>,
    pub required_skills: HashMap<u32, (u32, u16)>,
    pub weapon_proficiencies: HashMap<u32, u32>,
}

pub(super) fn load(conn: &Connection) -> rusqlite::Result<LootNeedMetadata> {
    let armor_masks = conn
        .prepare("SELECT ID, ArmorTypeMask FROM chr_classes")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let required_skills = conn
        .prepare(
            "SELECT ID, RequiredSkill, RequiredSkillRank FROM content_item WHERE RequiredSkill > 0",
        )?
        .query_map([], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))))?
        .collect::<rusqlite::Result<_>>()?;
    let weapon_proficiencies = conn.prepare("SELECT DISTINCT e.SpellID, e.EquippedItemSubclass FROM spell_equipped_items e JOIN spell_effect f ON f.SpellID = e.SpellID WHERE f.Effect = 60 AND e.EquippedItemClass = 2")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
    Ok(LootNeedMetadata {
        armor_masks,
        required_skills,
        weapon_proficiencies,
    })
}
