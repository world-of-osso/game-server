//! Bounded PlayerCondition semantics: existing ContentTuning levels and earned achievements.
//! Other gates remain explicit unknowns. ClientExecutable is metadata; PvPFaction0 is unset.
use rusqlite::{Connection, OptionalExtension, types::Value};
use std::collections::{BTreeMap, BTreeSet};

const CLIENT_EXECUTABLE: u32 = 0x1;
const WITHIN_OR_ABOVE: u32 = 0x20;
const DISABLED: u32 = 0x100;

#[derive(Clone, Debug)]
pub(super) enum ConditionRule {
    Always,
    Level { min: u8, max: u8 },
    Achievement { id: u32 },
    Unsupported(String),
}
impl ConditionRule {
    pub(super) fn evaluate(&self, level: u8, earned: Option<&BTreeSet<u32>>) -> Result<bool, &str> {
        match self {
            Self::Always => Ok(true),
            Self::Level { min, max } => Ok(level >= *min && (*max == 0 || level <= *max)),
            Self::Achievement { id } => earned
                .map(|earned| earned.contains(id))
                .ok_or("PlayerCondition achievement authority unavailable"),
            Self::Unsupported(reason) => Err(reason),
        }
    }
}

pub(super) fn load_conditions(
    conn: &Connection,
    ids: BTreeSet<u32>,
) -> Result<BTreeMap<u32, ConditionRule>, String> {
    let mut result = BTreeMap::new();
    for id in ids {
        if id == 0 {
            continue;
        }
        result.insert(id, load_condition(conn, id)?);
    }
    Ok(result)
}

fn load_condition(conn: &Connection, id: u32) -> Result<ConditionRule, String> {
    let mut query = conn
        .prepare("SELECT * FROM player_condition WHERE ID=?1")
        .map_err(|e| e.to_string())?;
    let names: Vec<_> = query.column_names().iter().map(|s| s.to_string()).collect();
    let values = query
        .query_row([id], |row| {
            (0..names.len())
                .map(|index| row.get::<_, Value>(index))
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .optional()
        .map_err(|e| e.to_string())?;
    let Some(values) = values else {
        return Ok(ConditionRule::Unsupported(format!(
            "PlayerCondition {id} missing"
        )));
    };
    let fields: BTreeMap<_, _> = names.into_iter().zip(values).collect();
    let number = |name: &str| match fields.get(name) {
        Some(Value::Integer(value)) => Ok(*value),
        _ => Err(format!(
            "PlayerCondition {id}: required integer {name} unavailable"
        )),
    };
    let flags = u32::try_from(number("Flags")?)
        .map_err(|_| format!("PlayerCondition {id}: invalid flags"))?;
    // Same TC flags as maps/store.rs: Disabled is unconditional; WithinOrAbove drops max.
    if flags & DISABLED != 0 {
        return Ok(ConditionRule::Always);
    }
    // TC PlayerConditionFlags::ClientExecutable0x1 adds no predicate.
    if flags & !(CLIENT_EXECUTABLE | WITHIN_OR_ABOVE) != 0 {
        return Ok(ConditionRule::Unsupported(format!(
            "PlayerCondition {id}: unsupported flags {flags}"
        )));
    }
    let races = (number("RaceMasks_0")?, number("RaceMasks_1")?);
    if !matches!(races, (0, 0) | (-1, -1)) {
        return Ok(ConditionRule::Unsupported(format!(
            "PlayerCondition {id}: unsupported race mask"
        )));
    }
    if let Some((name, _)) = fields
        .iter()
        .find(|(name, value)| !supported_field(name, value))
    {
        return Ok(ConditionRule::Unsupported(format!(
            "PlayerCondition {id}: unsupported {name}"
        )));
    }
    let tuning = u32::try_from(number("ContentTuningID")?)
        .map_err(|_| format!("PlayerCondition {id}: invalid ContentTuningID"))?;
    let achievement = u32::try_from(number("Achievement_0")?)
        .map_err(|_| format!("PlayerCondition {id}: invalid Achievement_0"))?;
    if achievement != 0 {
        // Source33559 is a single earned-achievement predicate with no other constraint.
        return Ok(if tuning == 0 {
            ConditionRule::Achievement { id: achievement }
        } else {
            ConditionRule::Unsupported(format!(
                "PlayerCondition {id}: combined ContentTuning/achievement unavailable"
            ))
        });
    }
    if tuning == 0 {
        return Ok(ConditionRule::Always);
    }
    let range = conn
        .query_row(
            "SELECT minlevel,maxlevel FROM content_tuning_level WHERE ID=?1",
            [tuning],
            |r| Ok((r.get::<_, u8>(0)?, r.get::<_, u8>(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    match range {
        Some((min, max)) => Ok(ConditionRule::Level {
            min,
            max: if flags & WITHIN_OR_ABOVE != 0 { 0 } else { max },
        }),
        None => Ok(ConditionRule::Unsupported(format!(
            "PlayerCondition {id}: ContentTuning {tuning} missing"
        ))),
    }
}

fn supported_field(name: &str, value: &Value) -> bool {
    if matches!(
        name,
        "ID" | "Failure_description_lang"
            | "Flags"
            | "ContentTuningID"
            | "MinLevel"
            | "MaxLevel"
            | "Achievement_0"
    ) {
        return true;
    }
    let Value::Integer(value) = value else {
        return false;
    };
    let minus_one_is_unconstrained = matches!(
        name,
        "Gender"
            | "NativeGender"
            | "ChrSpecializationIndex"
            | "ChrSpecializationRole"
            | "PowerType"
            | "MinExpansionLevel"
            | "MaxExpansionLevel"
            | "MinExpansionTier"
            | "MaxExpansionTier"
            | "ClassMask"
            | "RaceMasks_0"
            | "RaceMasks_1"
    );
    if *value == -1 && minus_one_is_unconstrained {
        return true;
    }
    let zero_is_a_constraint = matches!(
        name,
        "Gender"
            | "NativeGender"
            | "ChrSpecializationIndex"
            | "ChrSpecializationRole"
            | "PowerType"
            | "MaxExpansionLevel"
            | "MaxExpansionTier"
    );
    *value == 0 && !zero_is_a_constraint
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggested_provider_client_executable_level_gate_is_not_unknown() {
        let conn = Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        // Authored Random Battleground34154: ClientExecutable, PvPFaction0, tuning982(10–90).
        let condition = load_condition(&conn, 34154).unwrap();
        for (level, eligible) in [(9, false), (10, true), (90, true), (91, false)] {
            assert_eq!(
                condition.evaluate(level, None),
                Ok(eligible),
                "level{level}"
            );
        }
        // Wailing Caverns33956 also requires ModifierTree143498; keep it unknown.
        let unknown = load_condition(&conn, 33956).unwrap();
        assert!(
            unknown
                .evaluate(20, None)
                .unwrap_err()
                .contains("ModifierTreeID")
        );
    }

    #[test]
    fn suggested_provider_zero_pvp_faction_is_no_predicate_not_missing_authority() {
        let conn = Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        // Existing Grim Batol115171: WithinOrAbove, PvPFaction0, tuning2720(80).
        let condition = load_condition(&conn, 115171).unwrap();
        assert_eq!(condition.evaluate(79, None), Ok(false));
        assert_eq!(condition.evaluate(80, None), Ok(true));
        assert_eq!(condition.evaluate(90, None), Ok(true));
    }
}
