//! Per-class spell lists. Derivation rules match `scripts/spell_coverage.py`
//! (documented in `docs/spell-coverage.md`).

use bevy::prelude::Resource;
use rusqlite::Connection;
use serde::Serialize;

use super::load::{int, uint};
use super::trait_data::{TraitTree, load_trait_tree};

/// ChrClasses ID -> class skill line (skill_line CategoryID 7). Character creation classes only.
const CLASS_SKILL_LINES: [(u8, u32); 13] = [
    (1, 840),
    (2, 800),
    (3, 795),
    (4, 921),
    (5, 804),
    (6, 796),
    (7, 924),
    (8, 904),
    (9, 849),
    (10, 829),
    (11, 798),
    (12, 1848),
    (13, 2810),
];
pub(crate) const MAX_LEVEL: u32 = 90;
/// SkillLineAbilityAcquireMethod::NeverLearned (TrinityCore DBCEnums.h).
const ACQUIRE_NEVER_LEARNED: u32 = 3;
/// SPELL_ATTR0_PASSIVE in `spell_misc.Attributes_0`.
pub(super) const ATTR0_PASSIVE: u32 = 0x40;
/// SPELL_AURA_MOUNTED in `spell_effect.EffectAura`.
const AURA_MOUNTED: u32 = 78;
/// `spell_misc.Attributes_13` bit 16 (TrinityCore `SPELL_ATTR13_UNK16`): set on exactly the
/// 34 General (skill 183) content abilities (Garrison, Covenant, Lorewalking, Pocopoc,
/// Throw 385265...) and on no other auto-learned spell, so a fresh character is not taught
/// them (docs/wiki/investigations/general-spell-grants.md).
const ATTR13_CONTENT_ABILITY: u32 = 0x10000;

#[derive(Resource, Debug, Default, Serialize)]
#[cfg_attr(test, derive(Clone))]
pub struct ClassSpells {
    /// Sorted by class ID.
    pub classes: Vec<ClassInfo>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(Clone))]
pub struct ClassInfo {
    pub class_id: u8,
    pub skill_line: u32,
    /// `chr_classes.SpellClassSet`, used by class-based casting restrictions.
    pub spell_class_set: u32,
    /// `chr_classes_x_power_types.PowerType` raw DB IDs, ascending.
    pub power_types: Vec<i32>,
    /// `chr_classes.AttackPowerPerStrength` / `AttackPowerPerAgility`.
    pub attack_power_per_strength: f32,
    pub attack_power_per_agility: f32,
    /// `chr_classes.RangedAttackPowerPerAgility` (StatSystem.cpp:377).
    pub ranged_attack_power_per_agility: f32,
    /// `chr_classes.PrimaryStatPriority`: the main stat without a spec.
    pub primary_stat_priority: u32,
    /// Sorted by required level, then spell ID.
    pub learnable: Vec<LearnableSpell>,
    /// Sorted by `OrderIndex`.
    pub specs: Vec<SpecInfo>,
    /// Class talent tree (class, spec and hero nodes).
    pub traits: TraitTree,
    /// The default skill lines this class gets besides `skill_line` (`default_skill_lines`).
    pub default_skills: Vec<DefaultSkillLine>,
}

/// A default skill line a `skill_race_class_info` row with Availability 1 gives a class:
/// TrinityCore's player-create skills (ObjectMgr.cpp:4064-4078), learned by
/// `Player::LearnDefaultSkills` and their abilities by `LearnSkillRewardedSpells`
/// (Player.cpp:25275-25291, 25407-25457).
#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(Clone))]
pub struct DefaultSkillLine {
    pub skill_line: u32,
    /// `RaceMasks_0/_1`: the races that get the line (all-zero: every race).
    pub race_masks: [u32; 2],
    /// `MinLevel`: the line is learned from this level.
    pub min_level: u32,
    /// How the line's starting skill value is set.
    pub range: SkillRange,
    /// Its `skill_line_ability` rows for the class (`ClassMask` 0 or with its bit), not
    /// NeverLearned, sorted by required level then spell ID.
    pub abilities: Vec<LearnableSpell>,
}

/// `GetSkillRangeType` (ObjectMgr.cpp:9009-9030): the starting value
/// `Player::LearnDefaultSkill` (Player.cpp:25293-25336) gives a default skill line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SkillRange {
    /// A `tdb_skill_tiers` row: its first tier value is the maximum.
    Rank { max: u32, always_max: bool },
    /// Runeforging and armor lines: always 1.
    Mono,
    /// Languages: 300.
    Language,
    /// Everything else: up to 5 per level.
    Level { always_max: bool },
}

impl SkillRange {
    /// Starting value: 1, the maximum for `SKILL_FLAG_ALWAYS_MAX_VALUE` lines, and
    /// `(level - 1) * 5` (at least 1, at most the maximum) for a Death Knight.
    pub fn value(self, class_id: u8, level: u8) -> u32 {
        let start = |max: u32, always_max: bool| {
            if always_max {
                max
            } else if class_id == CLASS_DEATH_KNIGHT {
                ((u32::from(level) - 1) * 5).clamp(1, max.max(1))
            } else {
                1
            }
        };
        match self {
            Self::Rank { max, always_max } => start(max, always_max),
            Self::Mono => 1,
            Self::Language => 300,
            Self::Level { always_max } => start(u32::from(level) * 5, always_max),
        }
    }
}

/// A `skill_line_ability` row on the class skill line.
#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(Clone))]
pub struct LearnableSpell {
    pub spell_id: u32,
    pub class_mask: u32,
    pub acquire_method: u32,
    /// `spell_levels.SpellLevel` (DifficultyID 0); 0 when the spell has no levels row.
    pub required_level: u32,
    pub supersedes_spell: u32,
    pub race_masks: [u32; 2],
    /// `MinSkillLineRank`: an AutomaticSkillRank spell needs this skill value.
    pub min_skill_rank: u32,
    /// SPELL_ATTR0_PASSIVE.
    pub passive: bool,
    /// An effect applies SPELL_AURA_MOUNTED (class mounts such as Summon Warhorse 13819).
    pub mount: bool,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(Clone))]
pub struct SpecInfo {
    pub spec_id: u32,
    pub name: String,
    pub order_index: u32,
    /// ChrSpecializationRole: 0 tank, 1 healer, 2 damage.
    pub role: u32,
    pub primary_stat_priority: u32,
    pub flags: u32,
    /// Non-zero `MasterySpellID_0/1`.
    pub mastery_spells: Vec<u32>,
    /// `specialization_spells`, sorted by `DisplayOrder`.
    pub spells: Vec<SpecSpell>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(Clone))]
pub struct SpecSpell {
    pub spell_id: u32,
    pub overrides_spell_id: u32,
    pub display_order: u32,
    /// `spell_levels.SpellLevel` (DifficultyID 0); 0 when the spell has no levels row.
    pub required_level: u32,
    /// SPELL_ATTR0_PASSIVE.
    pub passive: bool,
}

pub(crate) fn load_class_spells(conn: &Connection) -> rusqlite::Result<ClassSpells> {
    let classes = CLASS_SKILL_LINES
        .iter()
        .map(|&(class_id, skill_line)| {
            let (
                attack_power_per_strength,
                attack_power_per_agility,
                ranged_attack_power_per_agility,
                primary_stat_priority,
                spell_class_set,
            ) = load_class_properties(conn, class_id)?;
            Ok(ClassInfo {
                class_id,
                skill_line,
                spell_class_set,
                power_types: load_power_types(conn, class_id)?,
                attack_power_per_strength,
                attack_power_per_agility,
                ranged_attack_power_per_agility,
                primary_stat_priority,
                learnable: load_learnable(conn, class_id, skill_line)?,
                specs: load_specs(conn, class_id)?,
                traits: load_trait_tree(conn, skill_line)?,
                default_skills: default_skill_lines(conn, class_id, skill_line)?,
            })
        })
        .collect::<rusqlite::Result<_>>()?;
    Ok(ClassSpells { classes })
}

/// `SKILL_FLAG_ALWAYS_MAX_VALUE` (DBCEnums.h:2394).
const SKILL_FLAG_ALWAYS_MAX_VALUE: u32 = 0x10;
/// `SKILL_RUNEFORGING` (SharedDefines.h:6015).
const SKILL_RUNEFORGING: u32 = 960;
/// `SKILL_CATEGORY_ARMOR` / `SKILL_CATEGORY_LANGUAGES` (SharedDefines.h:6380-6382).
const SKILL_CATEGORY_ARMOR: u32 = 8;
const SKILL_CATEGORY_LANGUAGES: u32 = 10;
const CLASS_DEATH_KNIGHT: u8 = 6;

/// The default skill lines `class_id` gets besides its class line: Availability 1 rows
/// whose `ClassMask` is -1, 0 or has the class bit (ObjectMgr.cpp:4070-4075), by
/// skill line then row ID. Rows of lines without a `skill_line` entry are skipped
/// (`SKILL_RANGE_NONE`).
fn default_skill_lines(
    conn: &Connection,
    class_id: u8,
    class_skill_line: u32,
) -> rusqlite::Result<Vec<DefaultSkillLine>> {
    let mut stmt = conn.prepare(
        "SELECT r.SkillID, r.ClassMask, r.RaceMasks_0, r.RaceMasks_1, r.MinLevel, r.Flags,
                s.CategoryID, t.Value1
         FROM skill_race_class_info r
         JOIN skill_line s ON s.ID = r.SkillID
         LEFT JOIN tdb_skill_tiers t ON t.ID = r.SkillTierID
         WHERE r.Availability = 1 AND r.SkillID != ?1
         ORDER BY r.SkillID, r.ID",
    )?;
    let rows = stmt.query_map([class_skill_line], |r| {
        Ok((
            uint(r, 0)?,
            int(r, 1)?,
            [uint(r, 2)?, uint(r, 3)?],
            uint(r, 4)?,
            uint(r, 5)?,
            uint(r, 6)?,
            r.get::<_, Option<u32>>(7)?,
        ))
    })?;
    let bit = i64::from(class_bit(class_id));
    let mut lines = Vec::new();
    for row in rows {
        let (skill_line, class_mask, race_masks, min_level, flags, category, tier_max) = row?;
        if class_mask == -1 || class_mask == 0 || class_mask & bit != 0 {
            let always_max = flags & SKILL_FLAG_ALWAYS_MAX_VALUE != 0;
            let range = match tier_max {
                Some(max) => SkillRange::Rank { max, always_max },
                None if skill_line == SKILL_RUNEFORGING || category == SKILL_CATEGORY_ARMOR => {
                    SkillRange::Mono
                }
                None if category == SKILL_CATEGORY_LANGUAGES => SkillRange::Language,
                None => SkillRange::Level { always_max },
            };
            lines.push(DefaultSkillLine {
                skill_line,
                race_masks,
                min_level,
                range,
                abilities: load_learnable(conn, class_id, skill_line)?,
            });
        }
    }
    Ok(lines)
}

fn class_bit(class_id: u8) -> u32 {
    1 << (class_id - 1)
}

fn load_power_types(conn: &Connection, class_id: u8) -> rusqlite::Result<Vec<i32>> {
    let mut stmt = conn.prepare(
        "SELECT PowerType FROM chr_classes_x_power_types WHERE ClassID = ?1 ORDER BY PowerType",
    )?;
    let rows = stmt.query_map([class_id], |r| Ok(int(r, 0)? as i32))?;
    rows.collect()
}

fn load_class_properties(
    conn: &Connection,
    class_id: u8,
) -> rusqlite::Result<(f32, f32, f32, u32, u32)> {
    conn.query_row(
        "SELECT AttackPowerPerStrength, AttackPowerPerAgility, RangedAttackPowerPerAgility, PrimaryStatPriority, SpellClassSet
         FROM chr_classes WHERE ID = ?1",
        [class_id],
        |r| {
            Ok((
                int(r, 0)? as f32,
                int(r, 1)? as f32,
                int(r, 2)? as f32,
                uint(r, 3)?,
                uint(r, 4)?,
            ))
        },
    )
}

/// Rule: ClassMask is 0 or has the class bit; AcquireMethod != NeverLearned;
/// SpellLevel <= MAX_LEVEL; not an [`ATTR13_CONTENT_ABILITY`]. RaceMasks is kept but not
/// filtered.
fn load_learnable(
    conn: &Connection,
    class_id: u8,
    skill_line: u32,
) -> rusqlite::Result<Vec<LearnableSpell>> {
    let mut stmt = conn.prepare(
        "SELECT a.Spell, a.ClassMask, a.AcquireMethod, COALESCE(l.SpellLevel, 0),
                a.SupercedesSpell, a.RaceMasks_0, a.RaceMasks_1, COALESCE(m.Attributes_0, 0),
                EXISTS(SELECT 1 FROM spell_effect e
                       WHERE e.SpellID = a.Spell AND e.EffectAura = ?2),
                a.MinSkillLineRank
         FROM skill_line_ability a
         JOIN spell_name n ON n.ID = a.Spell
         LEFT JOIN spell_levels l ON l.SpellID = a.Spell AND l.DifficultyID = 0
         LEFT JOIN spell_misc m ON m.SpellID = a.Spell AND m.DifficultyID = 0
         WHERE a.SkillLine = ?1 AND (COALESCE(m.Attributes_13, 0) & ?3) = 0
         ORDER BY 4, a.Spell",
    )?;
    let rows = stmt.query_map([skill_line, AURA_MOUNTED, ATTR13_CONTENT_ABILITY], |r| {
        Ok(LearnableSpell {
            spell_id: uint(r, 0)?,
            class_mask: uint(r, 1)?,
            acquire_method: uint(r, 2)?,
            required_level: uint(r, 3)?,
            supersedes_spell: uint(r, 4)?,
            race_masks: [uint(r, 5)?, uint(r, 6)?],
            min_skill_rank: uint(r, 9)?,
            passive: uint(r, 7)? & ATTR0_PASSIVE != 0,
            mount: uint(r, 8)? != 0,
        })
    })?;
    let bit = class_bit(class_id);
    let mut learnable = Vec::new();
    for row in rows {
        let spell = row?;
        if (spell.class_mask == 0 || spell.class_mask & bit != 0)
            && spell.acquire_method != ACQUIRE_NEVER_LEARNED
            && spell.required_level <= MAX_LEVEL
        {
            learnable.push(spell);
        }
    }
    Ok(learnable)
}

fn load_specs(conn: &Connection, class_id: u8) -> rusqlite::Result<Vec<SpecInfo>> {
    let mut stmt = conn.prepare(
        "SELECT ID, COALESCE(Name_lang, ''), OrderIndex, Role, PrimaryStatPriority, Flags,
                MasterySpellID_0, MasterySpellID_1
         FROM chr_specialization WHERE ClassID = ?1 ORDER BY OrderIndex, ID",
    )?;
    let rows = stmt.query_map([class_id], |r| {
        Ok(SpecInfo {
            spec_id: uint(r, 0)?,
            name: r.get(1)?,
            order_index: uint(r, 2)?,
            role: uint(r, 3)?,
            primary_stat_priority: uint(r, 4)?,
            flags: uint(r, 5)?,
            mastery_spells: [uint(r, 6)?, uint(r, 7)?]
                .into_iter()
                .filter(|&id| id != 0)
                .collect(),
            spells: Vec::new(),
        })
    })?;
    let mut specs = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    for spec in &mut specs {
        spec.spells = load_spec_spells(conn, spec.spec_id)?;
    }
    Ok(specs)
}

fn load_spec_spells(conn: &Connection, spec_id: u32) -> rusqlite::Result<Vec<SpecSpell>> {
    let mut stmt = conn.prepare_cached(
        "SELECT s.SpellID, s.OverridesSpellID, s.DisplayOrder, COALESCE(m.Attributes_0, 0),
                COALESCE(l.SpellLevel, 0)
         FROM specialization_spells s JOIN spell_name n ON n.ID = s.SpellID
         LEFT JOIN spell_misc m ON m.SpellID = s.SpellID AND m.DifficultyID = 0
         LEFT JOIN spell_levels l ON l.SpellID = s.SpellID AND l.DifficultyID = 0
         WHERE s.SpecID = ?1 ORDER BY s.DisplayOrder, s.ID",
    )?;
    let rows = stmt.query_map([spec_id], |r| {
        Ok(SpecSpell {
            spell_id: uint(r, 0)?,
            overrides_spell_id: uint(r, 1)?,
            display_order: uint(r, 2)?,
            required_level: uint(r, 4)?,
            passive: uint(r, 3)? & ATTR0_PASSIVE != 0,
        })
    })?;
    rows.collect()
}
