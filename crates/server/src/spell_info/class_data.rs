//! Per-class spell lists. Derivation rules match `scripts/spell_coverage.py`
//! (documented in `docs/spell-coverage.md`).

use bevy::prelude::Resource;
use rusqlite::Connection;
use serde::Serialize;

use super::load::{int, uint};

/// ChrClasses ID -> class skill line (skill_line CategoryID 7). Character creation classes only.
const CLASS_SKILL_LINES: [(u8, u32); 10] = [
    (1, 840),
    (2, 800),
    (3, 795),
    (4, 921),
    (5, 804),
    (6, 796),
    (7, 924),
    (8, 904),
    (9, 849),
    (11, 798),
];
pub(crate) const MAX_LEVEL: u32 = 90;
/// SkillLineAbilityAcquireMethod::NeverLearned (TrinityCore DBCEnums.h).
const ACQUIRE_NEVER_LEARNED: u32 = 3;
/// SPELL_ATTR0_PASSIVE in `spell_misc.Attributes_0`.
const ATTR0_PASSIVE: u32 = 0x40;

#[derive(Resource, Debug, Default, Serialize)]
pub struct ClassSpells {
    /// Sorted by class ID.
    pub classes: Vec<ClassInfo>,
}

#[derive(Debug, Serialize)]
pub struct ClassInfo {
    pub class_id: u8,
    pub skill_line: u32,
    /// `chr_classes_x_power_types.PowerType` raw DB IDs, ascending.
    pub power_types: Vec<i32>,
    /// Sorted by required level, then spell ID.
    pub learnable: Vec<LearnableSpell>,
    /// Sorted by `OrderIndex`.
    pub specs: Vec<SpecInfo>,
}

/// A `skill_line_ability` row on the class skill line.
#[derive(Debug, Serialize)]
pub struct LearnableSpell {
    pub spell_id: u32,
    pub class_mask: u32,
    pub acquire_method: u32,
    /// `spell_levels.SpellLevel` (DifficultyID 0); 0 when the spell has no levels row.
    pub required_level: u32,
    pub supersedes_spell: u32,
    pub race_masks: [u32; 2],
    /// SPELL_ATTR0_PASSIVE.
    pub passive: bool,
}

#[derive(Debug, Serialize)]
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
pub struct SpecSpell {
    pub spell_id: u32,
    pub overrides_spell_id: u32,
    pub display_order: u32,
    /// SPELL_ATTR0_PASSIVE.
    pub passive: bool,
}

pub(crate) fn load_class_spells(conn: &Connection) -> rusqlite::Result<ClassSpells> {
    let classes = CLASS_SKILL_LINES
        .iter()
        .map(|&(class_id, skill_line)| {
            Ok(ClassInfo {
                class_id,
                skill_line,
                power_types: load_power_types(conn, class_id)?,
                learnable: load_learnable(conn, class_id, skill_line)?,
                specs: load_specs(conn, class_id)?,
            })
        })
        .collect::<rusqlite::Result<_>>()?;
    Ok(ClassSpells { classes })
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

/// Rule: ClassMask is 0 or has the class bit; AcquireMethod != NeverLearned;
/// SpellLevel <= MAX_LEVEL. RaceMasks is kept but not filtered.
fn load_learnable(
    conn: &Connection,
    class_id: u8,
    skill_line: u32,
) -> rusqlite::Result<Vec<LearnableSpell>> {
    let mut stmt = conn.prepare(
        "SELECT a.Spell, a.ClassMask, a.AcquireMethod, COALESCE(l.SpellLevel, 0),
                a.SupercedesSpell, a.RaceMasks_0, a.RaceMasks_1, COALESCE(m.Attributes_0, 0)
         FROM skill_line_ability a
         JOIN spell_name n ON n.ID = a.Spell
         LEFT JOIN spell_levels l ON l.SpellID = a.Spell AND l.DifficultyID = 0
         LEFT JOIN spell_misc m ON m.SpellID = a.Spell AND m.DifficultyID = 0
         WHERE a.SkillLine = ?1
         ORDER BY 4, a.Spell",
    )?;
    let rows = stmt.query_map([skill_line], |r| {
        Ok(LearnableSpell {
            spell_id: uint(r, 0)?,
            class_mask: uint(r, 1)?,
            acquire_method: uint(r, 2)?,
            required_level: uint(r, 3)?,
            supersedes_spell: uint(r, 4)?,
            race_masks: [uint(r, 5)?, uint(r, 6)?],
            passive: uint(r, 7)? & ATTR0_PASSIVE != 0,
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
        "SELECT s.SpellID, s.OverridesSpellID, s.DisplayOrder, COALESCE(m.Attributes_0, 0)
         FROM specialization_spells s JOIN spell_name n ON n.ID = s.SpellID
         LEFT JOIN spell_misc m ON m.SpellID = s.SpellID AND m.DifficultyID = 0
         WHERE s.SpecID = ?1 ORDER BY s.DisplayOrder, s.ID",
    )?;
    let rows = stmt.query_map([spec_id], |r| {
        Ok(SpecSpell {
            spell_id: uint(r, 0)?,
            overrides_spell_id: uint(r, 1)?,
            display_order: uint(r, 2)?,
            passive: uint(r, 3)? & ATTR0_PASSIVE != 0,
        })
    })?;
    rows.collect()
}
