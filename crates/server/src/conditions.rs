//! TrinityCore conditions (`ConditionMgr`, game/Conditions/) for spell implicit targets
//! (`CONDITION_SOURCE_TYPE_SPELL_IMPLICIT_TARGET` 13, TDB `conditions`).
//!
//! A row's `SourceEntry` is the spell, `SourceGroup` the mask of its effects whose
//! implicit target search the condition filters (`addToSpellImplicitTargetConditions`).
//! A unit meets an effect's list when every condition of one `ElseGroup` holds
//! (`IsObjectMeetToConditionList`), each negated by `NegativeCondition`; an effect
//! without conditions takes every unit. Object-entry conditions (current51 and
//! source-supported legacy31) without a spawn id and creature StringId (58) are
//! modelled for units/players. Relation33 target0/target1/OwnedBy3 reads the real
//! caster and represented temporary-summon ownership, not any summoner association.
//! Other ownership representations/forms remain unmodelled; unsupported forms
//! are reported when loaded.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::components::{Npc, Player};

/// `CONDITION_SOURCE_TYPE_SPELL_IMPLICIT_TARGET`.
const SOURCE_SPELL_IMPLICIT_TARGET: u32 = 13;
/// `CONDITION_OBJECT_ENTRY_GUID`.
const CONDITION_OBJECT_ENTRY_GUID: u32 = 51;
const CONDITION_OBJECT_ENTRY_GUID_LEGACY: u32 = 31;
/// `CONDITION_STRING_ID`.
const CONDITION_STRING_ID: i64 = 58;
const CONDITION_RELATION_TO: i64 = 33;
const RELATION_OWNED_BY: u32 = 3;
/// `TypeID` values `CONDITION_OBJECT_ENTRY_GUID` compares (ObjectGuid.h).
const TYPEID_UNIT: u32 = 5;
const TYPEID_PLAYER: u32 = 6;
const LEGACY_TYPEID_UNIT: u32 = 3;
const LEGACY_TYPEID_PLAYER: u32 = 4;

#[derive(Debug, Clone, PartialEq)]
enum Check {
    /// `ConditionValue1` type id, `ConditionValue2` entry (0 = any).
    ObjectEntry { type_id: u32, entry: u32 },
    /// `ConditionStringValue1`: `Creature::HasStringId`.
    StringId(String),
    /// ConditionTarget0 is a represented temporary summon owned by caster target1.
    SummonOwnedByCaster,
    /// A condition type or form not modelled: never met.
    Unsupported,
}

#[derive(Debug, Clone, PartialEq)]
struct Condition {
    /// The authored shared-list identity, not just equal condition values.
    effect_mask: u32,
    else_group: u32,
    check: Check,
    negative: bool,
}

/// Implicit target conditions: (spell, effect index) -> conditions.
#[derive(Resource, Debug, Default)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct SpellTargetConditions(HashMap<(u32, u8), Vec<Condition>>);

impl SpellTargetConditions {
    pub(crate) fn share_targets(&self, spell: u32, first: u8, second: u8) -> bool {
        self.0.get(&(spell, first)) == self.0.get(&(spell, second))
    }

    /// `ConditionMgr::IsObjectMeetToConditions` for `unit` as the target of effect
    /// `effect_index` of `spell_id`.
    pub(crate) fn target_meets(
        &self,
        world: &World,
        spell_id: u32,
        effect_index: u8,
        unit: Entity,
        caster: Entity,
    ) -> bool {
        let Some(conditions) = self.0.get(&(spell_id, effect_index)) else {
            return true;
        };
        let mut groups: HashMap<u32, bool> = HashMap::new();
        for condition in conditions {
            let met = condition_meets(world, &condition.check, unit, caster) != condition.negative;
            *groups.entry(condition.else_group).or_insert(true) &= met;
        }
        groups.values().any(|&met| met)
    }
}

/// `Condition::Meets` (ConditionMgr.cpp:213-), condition target 0 being `unit`.
fn condition_meets(world: &World, check: &Check, unit: Entity, caster: Entity) -> bool {
    match *check {
        Check::ObjectEntry { type_id, entry } => {
            let (unit_type, unit_entry) = match world.get::<Npc>(unit) {
                Some(npc) => (TYPEID_UNIT, npc.template_id),
                None if world.get::<Player>(unit).is_some() => (TYPEID_PLAYER, 0),
                None => return false,
            };
            unit_type == type_id && (entry == 0 || unit_entry == entry)
        }
        Check::StringId(ref id) => world
            .get::<crate::zones::CreatureStringIds>(unit)
            .is_some_and(|ids| ids.has(id)),
        Check::SummonOwnedByCaster => world
            .get::<crate::spell_cast::summon::SummonedCreature>(unit)
            .is_some_and(|summon| summon.owner == caster && summon.properties.has_source_owner()),
        Check::Unsupported => false,
    }
}

/// ConditionMgr converts legacy TypeIDs before matching current ObjectGuid types.
fn decode_object_entry(kind: i64, type_id: u32, entry: u32) -> Check {
    let current_type = match (kind as u32, type_id) {
        (CONDITION_OBJECT_ENTRY_GUID, TYPEID_UNIT)
        | (CONDITION_OBJECT_ENTRY_GUID_LEGACY, LEGACY_TYPEID_UNIT) => TYPEID_UNIT,
        (CONDITION_OBJECT_ENTRY_GUID, TYPEID_PLAYER)
        | (CONDITION_OBJECT_ENTRY_GUID_LEGACY, LEGACY_TYPEID_PLAYER) => TYPEID_PLAYER,
        _ => return Check::Unsupported,
    };
    Check::ObjectEntry {
        type_id: current_type,
        entry,
    }
}

const CONDITION_QUERY: &str = "SELECT SourceEntry, SourceGroup, ElseGroup, ConditionTypeOrReference, \
     ConditionTarget, ConditionValue1, ConditionValue2, ConditionValue3, NegativeCondition, \
     ConditionStringValue1 \
     FROM tdb_conditions WHERE SourceTypeOrReferenceId = ?1 \
     ORDER BY SourceEntry, SourceGroup, ElseGroup";

pub(crate) fn load_spell_target_conditions(path: &Path) -> rusqlite::Result<SpellTargetConditions> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement = conn.prepare(CONDITION_QUERY)?;
    let rows = statement.query_map([SOURCE_SPELL_IMPLICIT_TARGET], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            row.get::<_, u32>(2)?,
            // Negative: a reference to a condition template.
            row.get::<_, i64>(3)?,
            row.get::<_, u32>(4)?,
            [row.get::<_, u32>(5)?, row.get(6)?, row.get(7)?],
            row.get::<_, u32>(8)? != 0,
            row.get::<_, Option<String>>(9)?.unwrap_or_default(),
        ))
    })?;
    let mut conditions = SpellTargetConditions::default();
    let mut unsupported = 0;
    for row in rows {
        let (
            spell,
            effect_mask,
            else_group,
            kind,
            target,
            [value1, value2, value3],
            negative,
            text,
        ) = row?;
        let check = match kind {
            _ if target != 0 => Check::Unsupported,
            CONDITION_STRING_ID => Check::StringId(text),
            CONDITION_RELATION_TO if [value1, value2, value3] == [1, RELATION_OWNED_BY, 0] => {
                Check::SummonOwnedByCaster
            }
            _ if value3 == 0 => decode_object_entry(kind, value1, value2),
            _ => Check::Unsupported,
        };
        if check == Check::Unsupported {
            unsupported += 1;
        }
        for effect_index in (0..32u8).filter(|index| effect_mask & (1 << index) != 0) {
            conditions
                .0
                .entry((spell, effect_index))
                .or_default()
                .push(Condition {
                    effect_mask,
                    else_group,
                    check: check.clone(),
                    negative,
                });
        }
    }
    if unsupported > 0 {
        warn!("{unsupported} spell implicit target conditions are not modelled and never met");
    }
    Ok(conditions)
}

pub struct ConditionsPlugin;

impl Plugin for ConditionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup_load_conditions);
    }
}

fn startup_load_conditions(mut commands: Commands) {
    let path = Path::new(crate::spell_info::WORLD_DB_PATH);
    let conditions = load_spell_target_conditions(path)
        .unwrap_or_else(|error| panic!("load tdb_conditions from {}: {error}", path.display()));
    info!(
        "Loaded spell implicit target conditions for {} spell effects",
        conditions.0.len()
    );
    commands.insert_resource(conditions);
}
