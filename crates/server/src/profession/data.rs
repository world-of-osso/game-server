//! Profession data loaded from world.db: Retail DB2 skill lines, abilities,
//! reagents and tools (build 12.1.0.69933), and TrinityCore TDB1210 trainers and
//! skill tier caps (`tdb_*`, `scripts/tdb_schema.sql`).

#[cfg(test)]
#[path = "source_requirement_tests.rs"]
mod source_requirement_tests;

use std::collections::HashMap;

use rusqlite::{Connection, Row, params};

use crate::race_class::race_mask_allows;

/// `SkillLine.CategoryID` of primary professions (TrinityCore `SKILL_CATEGORY_PROFESSION`).
pub const SKILL_CATEGORY_PROFESSION: u32 = 11;
/// `SkillLineAbilityAcquireMethod::AutomaticSkillRank`.
pub const ACQUIRE_AUTOMATIC_SKILL_RANK: u32 = 1;
/// AutomaticSkillRank, AutomaticCharLevel, LearnedOrAutomaticCharLevel
/// (TrinityCore `Player::LearnSkillRewardedSpells`).
const AUTO_LEARN_METHODS: [u32; 3] = [1, 2, 4];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillLineInfo {
    pub name: String,
    pub category: u32,
    pub parent: u32,
    pub parent_tier_index: u16,
}

/// One `SkillLineAbility` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ability {
    pub id: u32,
    pub spell: u32,
    pub skill_line: u32,
    /// Line the recipe raises; 0 = `skill_line`.
    pub skillup_line: u32,
    pub min_rank: u16,
    pub acquire_method: u32,
    pub trivial_low: u16,
    pub trivial_high: u16,
    pub num_skill_ups: u16,
    pub class_mask: u32,
    pub race_masks: [u32; 2],
}

impl Ability {
    fn fits(&self, race: u8, class: u8) -> bool {
        let class_ok = self.class_mask == 0
            || class
                .checked_sub(1)
                .is_some_and(|bit| self.class_mask & (1 << bit) != 0);
        class_ok && race_mask_allows(self.race_masks, race)
    }

    pub fn auto_learned(&self) -> bool {
        AUTO_LEARN_METHODS.contains(&self.acquire_method)
    }
}

/// `SpellTotems`: tools that must be in the bags (not consumed).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tools {
    pub items: Vec<u32>,
    pub categories: Vec<u32>,
}

/// One `tdb_trainer_spell` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerSpell {
    pub spell: u32,
    pub cost: u64,
    pub req_skill_line: u32,
    pub req_skill_rank: u16,
    pub req_abilities: Vec<u32>,
    pub req_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trainer {
    pub id: u32,
    pub greeting: String,
    pub spells: Vec<TrainerSpell>,
}

/// A `SkillRaceClassInfo` row: the skill's tier for matching classes and races.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TierRow {
    class_mask: u32,
    race_masks: [u32; 2],
    tier: u32,
    flags: u32,
}

/// `SKILL_FLAG_ALWAYS_MAX_VALUE` (DBCEnums.h:2394): the skill's value is its maximum
/// (Riding).
const SKILL_FLAG_ALWAYS_MAX_VALUE: u32 = 0x10;

impl TierRow {
    fn fits(&self, race: u8, class: u8) -> bool {
        let class_ok = self.class_mask == 0
            || self.class_mask == u32::MAX
            || class
                .checked_sub(1)
                .is_some_and(|bit| self.class_mask & (1 << bit) != 0);
        class_ok && race_mask_allows(self.race_masks, race)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotemCategory {
    pub name: String,
    kind: u32,
    mask: u32,
}

#[derive(bevy::prelude::Resource, Debug, Default)]
pub struct ProfessionData {
    skill_lines: HashMap<u32, SkillLineInfo>,
    tier_rows: HashMap<u32, Vec<TierRow>>,
    tiers: HashMap<u32, [u16; 16]>,
    abilities_by_spell: HashMap<u32, Vec<Ability>>,
    /// Abilities by `SkillupSkillLineID`, or `SkillLine` when that is 0
    /// (TrinityCore `DB2Manager::GetSkillLineAbilitiesBySkill`).
    abilities_by_skill: HashMap<u32, Vec<Ability>>,
    reagents: HashMap<u32, Vec<(u32, u32)>>,
    tools: HashMap<u32, Tools>,
    item_totem_categories: HashMap<u32, u32>,
    totem_categories: HashMap<u32, TotemCategory>,
    trainers: HashMap<u32, Trainer>,
    creature_trainers: HashMap<u32, Vec<u32>>,
}

impl ProfessionData {
    /// Static trainer rows for learned owner skill lines, independent of recipe knowledge.
    pub fn trainer_sources(
        &self,
        lines: &[shared::profession::ProfessionSkillLine],
    ) -> Vec<shared::protocol::ProfessionTrainerSource> {
        let learned: std::collections::HashSet<_> = lines
            .iter()
            .filter(|line| line.rank > 0)
            .map(|line| line.skill_line)
            .collect();
        let mut sources: Vec<_> = self
            .trainers
            .values()
            .flat_map(|trainer| {
                trainer
                    .spells
                    .iter()
                    .filter(|spell| learned.contains(&spell.req_skill_line))
                    .map(|spell| shared::protocol::ProfessionTrainerSource {
                        trainer_id: trainer.id,
                        spell_id: spell.spell,
                        cost: spell.cost,
                        req_skill_line: spell.req_skill_line,
                        req_skill_rank: spell.req_skill_rank,
                        req_abilities: spell.req_abilities.clone(),
                        req_level: spell.req_level,
                    })
            })
            .collect();
        sources.sort_by_key(|source| (source.spell_id, source.trainer_id));
        sources
    }

    pub fn skill_line(&self, id: u32) -> Option<&SkillLineInfo> {
        self.skill_lines.get(&id)
    }

    /// TrinityCore `IsPrimaryProfessionSkill`.
    pub fn is_primary_profession(&self, skill: u32) -> bool {
        self.skill_line(skill)
            .is_some_and(|line| line.category == SKILL_CATEGORY_PROFESSION && line.parent == 0)
    }

    /// TrinityCore `SkillTiersEntry::GetValueForTierIndex(step - 1)` (ObjectMgr.cpp:7955-7964)
    /// of the skill's `SkillRaceClassInfo` row for race/class: an unset tier falls back to
    /// the nearest lower set one. `None` without a row or tier (`EffectSkill` returns).
    pub fn tier_cap(&self, skill: u32, step: u16, race: u8, class: u8) -> Option<u16> {
        let index = usize::from(step.checked_sub(1)?);
        let row = self
            .tier_rows
            .get(&skill)?
            .iter()
            .find(|row| row.fits(race, class))?;
        let values = self.tiers.get(&row.tier)?;
        let index = index.min(values.len() - 1);
        values[..=index]
            .iter()
            .rev()
            .find(|value| **value != 0)
            .or(values.first())
            .copied()
    }

    /// Whether the race/class's `SkillRaceClassInfo` row of `skill` has
    /// `SKILL_FLAG_ALWAYS_MAX_VALUE`.
    pub fn always_max(&self, skill: u32, race: u8, class: u8) -> bool {
        self.tier_rows
            .get(&skill)
            .and_then(|rows| rows.iter().find(|row| row.fits(race, class)))
            .is_some_and(|row| row.flags & SKILL_FLAG_ALWAYS_MAX_VALUE != 0)
    }

    /// TC modifier39 stores SkillLineAbility identity, not the spell ID.
    /// Ambiguous/missing authored rows cannot establish an instance origin.
    pub fn crafting_origin(&self, spell: u32) -> Option<shared::protocol::CraftingOrigin> {
        let [ability] = self.abilities_for_spell(spell) else {
            return None;
        };
        Some(shared::protocol::CraftingOrigin {
            skill_line_ability_id: ability.id,
            recipe_spell_id: ability.spell,
            crafting_quality_id: None,
        })
    }

    pub fn abilities_for_spell(&self, spell: u32) -> &[Ability] {
        self.abilities_by_spell
            .get(&spell)
            .map_or(&[], Vec::as_slice)
    }

    /// Abilities `skill` rewards that fit race/class.
    pub fn abilities_for_skill(
        &self,
        skill: u32,
        race: u8,
        class: u8,
    ) -> impl Iterator<Item = &Ability> {
        self.abilities_by_skill
            .get(&skill)
            .into_iter()
            .flatten()
            .filter(move |ability| ability.fits(race, class))
    }

    /// TrinityCore `IsSpellFitByClassAndRace`: spells without abilities fit everyone.
    pub fn spell_fits(&self, spell: u32, race: u8, class: u8) -> bool {
        let abilities = self.abilities_for_spell(spell);
        abilities.is_empty() || abilities.iter().any(|ability| ability.fits(race, class))
    }

    pub fn reagents(&self, spell: u32) -> &[(u32, u32)] {
        self.reagents.get(&spell).map_or(&[], Vec::as_slice)
    }

    pub fn tools(&self, spell: u32) -> Option<&Tools> {
        self.tools.get(&spell)
    }

    /// TrinityCore `DB2Manager::IsTotemCategoryCompatibleWith`: same type and the
    /// item's mask covers the required mask.
    pub fn item_meets_totem_category(&self, item: u32, required: u32) -> bool {
        let Some(have) = self.item_totem_categories.get(&item) else {
            return false;
        };
        let (Some(have), Some(want)) = (
            self.totem_categories.get(have),
            self.totem_categories.get(&required),
        ) else {
            return false;
        };
        have.kind == want.kind && have.mask & want.mask == want.mask
    }

    pub fn totem_category(&self, id: u32) -> Option<&TotemCategory> {
        self.totem_categories.get(&id)
    }

    pub fn trainer(&self, id: u32) -> Option<&Trainer> {
        self.trainers.get(&id)
    }

    /// Trainer ids `tdb_creature_trainer` lists for a creature entry.
    pub fn trainers_for_creature(&self, entry: u32) -> &[u32] {
        self.creature_trainers
            .get(&entry)
            .map_or(&[], Vec::as_slice)
    }
}

fn uint(row: &Row, index: usize) -> rusqlite::Result<u32> {
    Ok(row.get::<_, i64>(index)? as u32)
}

fn small(row: &Row, index: usize) -> rusqlite::Result<u16> {
    Ok(row.get::<_, i64>(index)?.clamp(0, i64::from(u16::MAX)) as u16)
}

pub fn load_profession_data(conn: &Connection) -> rusqlite::Result<ProfessionData> {
    let mut data = ProfessionData {
        skill_lines: load_skill_lines(conn)?,
        tier_rows: load_tier_rows(conn)?,
        tiers: load_tiers(conn)?,
        reagents: load_reagents(conn)?,
        tools: load_tools(conn)?,
        item_totem_categories: load_pairs(
            conn,
            "SELECT ID, TotemCategoryID FROM item_sparse WHERE TotemCategoryID > 0",
        )?,
        totem_categories: load_totem_categories(conn)?,
        trainers: load_trainers(conn)?,
        creature_trainers: load_creature_trainers(conn)?,
        ..Default::default()
    };
    for ability in load_abilities(conn)? {
        let key = match ability.skillup_line {
            0 => ability.skill_line,
            line => line,
        };
        data.abilities_by_skill
            .entry(key)
            .or_default()
            .push(ability.clone());
        data.abilities_by_spell
            .entry(ability.spell)
            .or_default()
            .push(ability);
    }
    Ok(data)
}

fn load_skill_lines(conn: &Connection) -> rusqlite::Result<HashMap<u32, SkillLineInfo>> {
    let mut statement = conn.prepare(
        "SELECT ID, DisplayName_lang, CategoryID, ParentSkillLineID, ParentTierIndex FROM skill_line",
    )?;
    let rows = statement.query_map(params![], |row| {
        Ok((
            uint(row, 0)?,
            SkillLineInfo {
                name: row.get(1)?,
                category: uint(row, 2)?,
                parent: uint(row, 3)?,
                parent_tier_index: small(row, 4)?,
            },
        ))
    })?;
    rows.collect()
}

fn load_tier_rows(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<TierRow>>> {
    let mut statement = conn.prepare(
        "SELECT SkillID, ClassMask, RaceMasks_0, RaceMasks_1, SkillTierID, Flags
         FROM skill_race_class_info",
    )?;
    let mut rows: HashMap<u32, Vec<TierRow>> = HashMap::new();
    let mapped = statement.query_map(params![], |row| {
        Ok((
            uint(row, 0)?,
            TierRow {
                class_mask: uint(row, 1)?,
                race_masks: [uint(row, 2)?, uint(row, 3)?],
                tier: uint(row, 4)?,
                flags: uint(row, 5)?,
            },
        ))
    })?;
    for row in mapped {
        let (skill, tier) = row?;
        rows.entry(skill).or_default().push(tier);
    }
    Ok(rows)
}

fn load_tiers(conn: &Connection) -> rusqlite::Result<HashMap<u32, [u16; 16]>> {
    let columns = (1..=16)
        .map(|i| format!("Value{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut statement = conn.prepare(&format!("SELECT ID, {columns} FROM tdb_skill_tiers"))?;
    let rows = statement.query_map(params![], |row| {
        let mut values = [0u16; 16];
        for (index, value) in values.iter_mut().enumerate() {
            *value = small(row, index + 1)?;
        }
        Ok((uint(row, 0)?, values))
    })?;
    rows.collect()
}

fn load_abilities(conn: &Connection) -> rusqlite::Result<Vec<Ability>> {
    let mut statement = conn.prepare(
        "SELECT Spell, SkillLine, SkillupSkillLineID, MinSkillLineRank, AcquireMethod, \
         TrivialSkillLineRankLow, TrivialSkillLineRankHigh, NumSkillUps, ClassMask, \
         RaceMasks_0, RaceMasks_1, ID FROM skill_line_ability ORDER BY ID",
    )?;
    let rows = statement.query_map(params![], |row| {
        Ok(Ability {
            id: uint(row, 11)?,
            spell: uint(row, 0)?,
            skill_line: uint(row, 1)?,
            skillup_line: uint(row, 2)?,
            min_rank: small(row, 3)?,
            acquire_method: uint(row, 4)?,
            trivial_low: small(row, 5)?,
            trivial_high: small(row, 6)?,
            num_skill_ups: small(row, 7)?,
            class_mask: uint(row, 8)?,
            race_masks: [uint(row, 9)?, uint(row, 10)?],
        })
    })?;
    rows.collect()
}

fn load_reagents(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<(u32, u32)>>> {
    let items = (0..8).map(|i| format!("Reagent_{i}")).collect::<Vec<_>>();
    let counts = (0..8)
        .map(|i| format!("ReagentCount_{i}"))
        .collect::<Vec<_>>();
    let sql = format!(
        "SELECT SpellID, {}, {} FROM spell_reagents",
        items.join(", "),
        counts.join(", ")
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(params![], |row| {
        let mut reagents = Vec::new();
        for slot in 0..8 {
            let item = row.get::<_, i64>(1 + slot)?;
            let count = row.get::<_, i64>(9 + slot)?;
            if item > 0 && count > 0 {
                reagents.push((item as u32, count as u32));
            }
        }
        Ok((uint(row, 0)?, reagents))
    })?;
    let mut map: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
    for row in rows {
        let (spell, reagents) = row?;
        map.entry(spell).or_default().extend(reagents);
    }
    map.retain(|_, reagents| !reagents.is_empty());
    Ok(map)
}

fn load_tools(conn: &Connection) -> rusqlite::Result<HashMap<u32, Tools>> {
    let mut statement = conn.prepare(
        "SELECT SpellID, RequiredTotemCategoryID_0, RequiredTotemCategoryID_1, Totem_0, Totem_1 \
         FROM spell_totems",
    )?;
    let rows = statement.query_map(params![], |row| {
        let nonzero = |a: u32, b: u32| [a, b].into_iter().filter(|&v| v != 0).collect();
        Ok((
            uint(row, 0)?,
            Tools {
                categories: nonzero(uint(row, 1)?, uint(row, 2)?),
                items: nonzero(uint(row, 3)?, uint(row, 4)?),
            },
        ))
    })?;
    rows.collect()
}

fn load_pairs(conn: &Connection, sql: &str) -> rusqlite::Result<HashMap<u32, u32>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map(params![], |row| Ok((uint(row, 0)?, uint(row, 1)?)))?;
    rows.collect()
}

fn load_totem_categories(conn: &Connection) -> rusqlite::Result<HashMap<u32, TotemCategory>> {
    let mut statement = conn.prepare(
        "SELECT ID, Name_lang, TotemCategoryType, TotemCategoryMask FROM totem_category",
    )?;
    let rows = statement.query_map(params![], |row| {
        Ok((
            uint(row, 0)?,
            TotemCategory {
                name: row.get(1)?,
                kind: uint(row, 2)?,
                mask: uint(row, 3)?,
            },
        ))
    })?;
    rows.collect()
}

fn load_trainers(conn: &Connection) -> rusqlite::Result<HashMap<u32, Trainer>> {
    let mut trainers: HashMap<u32, Trainer> = conn
        .prepare("SELECT Id, COALESCE(Greeting, '') FROM tdb_trainer")?
        .query_map(params![], |row| {
            let id = uint(row, 0)?;
            Ok((
                id,
                Trainer {
                    id,
                    greeting: row.get(1)?,
                    spells: Vec::new(),
                },
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut statement = conn.prepare(
        "SELECT TrainerId, SpellId, MoneyCost, ReqSkillLine, ReqSkillRank, ReqAbility1, \
         ReqAbility2, ReqAbility3, ReqLevel FROM tdb_trainer_spell ORDER BY TrainerId, SpellId",
    )?;
    let rows = statement.query_map(params![], |row| {
        Ok((
            uint(row, 0)?,
            TrainerSpell {
                spell: uint(row, 1)?,
                cost: u64::from(uint(row, 2)?),
                req_skill_line: uint(row, 3)?,
                req_skill_rank: small(row, 4)?,
                req_abilities: [uint(row, 5)?, uint(row, 6)?, uint(row, 7)?]
                    .into_iter()
                    .filter(|&spell| spell != 0)
                    .collect(),
                req_level: row.get::<_, i64>(8)?.clamp(0, 255) as u8,
            },
        ))
    })?;
    for row in rows {
        let (trainer, spell) = row?;
        if let Some(trainer) = trainers.get_mut(&trainer) {
            trainer.spells.push(spell);
        }
    }
    Ok(trainers)
}

fn load_creature_trainers(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<u32>>> {
    let mut statement = conn.prepare(
        "SELECT DISTINCT CreatureID, TrainerID FROM tdb_creature_trainer ORDER BY CreatureID, TrainerID",
    )?;
    let mut map: HashMap<u32, Vec<u32>> = HashMap::new();
    let rows = statement.query_map(params![], |row| Ok((uint(row, 0)?, uint(row, 1)?)))?;
    for row in rows {
        let (creature, trainer) = row?;
        map.entry(creature).or_default().push(trainer);
    }
    Ok(map)
}
