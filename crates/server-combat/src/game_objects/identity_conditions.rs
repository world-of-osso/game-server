//! Bounded native PlayerCondition class/race predicates for spellcaster objects.
//! TC a352b1fa ConditionMgr.cpp:2903-2959, DBCEnums.h:2189-2203. Other authored predicates
//! remain explicit unsupported conditions, never an unconditional allow.
use rusqlite::{Connection, OptionalExtension, Row};
use shared::components::Player;

const CLIENT_EXECUTABLE: u32 = 0x1;

#[derive(Debug, Clone)]
pub(crate) struct IdentityCondition {
    class_mask: u32,
    race_mask: u64,
}

impl IdentityCondition {
    pub(super) fn permits(&self, player: &Player) -> bool {
        self.permits_race_class(player.race, player.class)
    }

    pub(crate) fn permits_race_class(&self, race: u8, class: u8) -> bool {
        let class = class
            .checked_sub(1)
            .and_then(|bit| 1_u32.checked_shl(u32::from(bit)));
        let race =
            crate::race_class::race_bit(race).and_then(|bit| 1_u64.checked_shl(u32::from(bit)));
        (self.class_mask == 0 || class.is_some_and(|bit| self.class_mask & bit != 0))
            && (self.race_mask == 0 || race.is_some_and(|bit| self.race_mask & bit != 0))
    }
}

pub(crate) fn load_condition(
    conn: &Connection,
    id: u32,
) -> rusqlite::Result<Result<IdentityCondition, String>> {
    if id == 0 {
        return Ok(Ok(IdentityCondition {
            class_mask: 0,
            race_mask: 0,
        }));
    }
    let extra: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM tdb_conditions WHERE SourceTypeOrReferenceId=34 AND SourceEntry=?1)", [id], |row| row.get(0))?;
    if extra {
        return Ok(Err(format!(
            "PlayerCondition {id} requires TDB source34 predicates"
        )));
    }
    let found = conn
        .query_row(
            "SELECT * FROM player_condition WHERE ID=?1",
            [id],
            read_condition,
        )
        .optional()?;
    Ok(found.unwrap_or_else(|| Err(format!("missing PlayerCondition {id}"))))
}

fn read_condition(row: &Row) -> rusqlite::Result<Result<IdentityCondition, String>> {
    let id: u32 = row.get("ID")?;
    let flags: u32 = row.get("Flags")?;
    if flags & !CLIENT_EXECUTABLE != 0 {
        return Ok(Err(format!(
            "PlayerCondition {id} requires flags {flags:#x}"
        )));
    }
    if let Some(field) = unsupported_field(row)? {
        return Ok(Err(format!("PlayerCondition {id} requires {field}")));
    }
    let low = row.get::<_, i64>("RaceMasks_0")? as u32;
    let high = row.get::<_, i64>("RaceMasks_1")? as u32;
    Ok(Ok(IdentityCondition {
        class_mask: row.get::<_, i64>("ClassMask")? as u32,
        race_mask: u64::from(low) | (u64::from(high) << 32),
    }))
}

fn unsupported_field(row: &Row) -> rusqlite::Result<Option<String>> {
    const IDENTITY_FIELDS: &[&str] = &[
        "ID",
        "Flags",
        "ClassMask",
        "RaceMasks_0",
        "RaceMasks_1",
        "Failure_description_lang",
    ];
    const UNSET_FIELDS: &[&str] = &[
        "Gender",
        "NativeGender",
        "MinExpansionLevel",
        "MaxExpansionLevel",
        "ChrSpecializationIndex",
        "ChrSpecializationRole",
        "PowerType",
        "MaxExpansionTier",
        "MinExpansionTier",
    ];
    for name in row.as_ref().column_names() {
        if IDENTITY_FIELDS.contains(&name) {
            continue;
        }
        let unset = if UNSET_FIELDS.contains(&name) {
            -1.0
        } else {
            0.0
        };
        if row.get::<_, f64>(name)? != unset {
            return Ok(Some(name.into()));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_completeness_native_death_gate_condition_accepts_only_death_knights() {
        let conn = Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        // Native50977 creates190942; TDB Data5=40073 authors ClassMask32.
        let condition_id: u32 = conn
            .query_row(
                "SELECT Data5 FROM tdb_gameobject_template WHERE entry=190942",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let condition = load_condition(&conn, condition_id).unwrap().unwrap();
        for class in crate::race_class::PLAYABLE_CLASSES {
            let player = Player {
                name: "Gateuser".into(),
                race: 1,
                class,
                appearance: Default::default(),
            };
            assert_eq!(
                condition.permits(&player),
                class == 6,
                "native class {class}"
            );
        }
    }

    #[test]
    fn class_completeness_native_portal_condition_never_drops_authored_modifier_tree() {
        let conn = Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        // Authored114864 contains ModifierTree382363 plus InvertModifierTree.
        assert!(load_condition(&conn, 114864).unwrap().is_err());
    }
}
