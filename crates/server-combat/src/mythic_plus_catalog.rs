//! Azure Vault qualification from the imported catalog and existing criteria trees.
use crate::mythic_plus_runtime::ChallengeDefinition;
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashMap;

/// Display metadata does not imply a qualified challenge run or active season.
#[derive(Clone, Debug)]
pub struct ChallengeSummary {
    pub name: String,
    pub time_limit_ms: u64,
}

fn summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChallengeSummary> {
    let name: String = row.get(0)?;
    let seconds: u64 = row.get(1)?;
    if name.is_empty() || seconds == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(ChallengeSummary {
        name,
        time_limit_ms: seconds * 1000,
    })
}

pub fn load_summary(
    conn: &Connection,
    challenge: u32,
) -> rusqlite::Result<Option<ChallengeSummary>> {
    conn.query_row(
        "SELECT Name_lang,CriteriaCount_0 FROM map_challenge_mode WHERE ID=?1 AND MaxMedals=3",
        [challenge],
        summary_from_row,
    )
    .optional()
}

pub fn load_summaries(conn: &Connection) -> rusqlite::Result<HashMap<u32, ChallengeSummary>> {
    let mut statement = conn
        .prepare("SELECT Name_lang,CriteriaCount_0,ID FROM map_challenge_mode WHERE MaxMedals=3")?;
    statement
        .query_map([], |row| Ok((row.get(2)?, summary_from_row(row)?)))?
        .collect()
}

pub fn load_azure(conn: &Connection) -> rusqlite::Result<Option<ChallengeDefinition>> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='map_challenge_mode')",
        [],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok(None);
    }
    let (name, map_id, timers): (String,u32,[u64;3]) = conn.query_row("SELECT Name_lang,MapID,CriteriaCount_0,CriteriaCount_1,CriteriaCount_2 FROM map_challenge_mode WHERE ID=401 AND MaxMedals=3",[],|r| Ok((r.get(0)?,r.get(1)?,[r.get::<_,u64>(2)?*1000,r.get::<_,u64>(3)?*1000,r.get::<_,u64>(4)?*1000])))?;
    let tree: u32 = conn.query_row(
        "SELECT CriteriatreeID FROM scenario_step WHERE ID=5383 AND ScenarioID=2133",
        [],
        |r| r.get(0),
    )?;
    let (force_tree, forces_required): (u32, u64) = conn.query_row(
        "SELECT ID,Amount FROM criteria_tree WHERE Parent=?1 AND Operator=9",
        [tree],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let mut statement=conn.prepare("SELECT c.Type,c.Asset,t.Amount FROM criteria_tree t JOIN criteria c ON c.ID=t.CriteriaID WHERE t.Parent=?1")?;
    let rows = statement.query_map([force_tree], |r| {
        Ok((
            r.get::<_, u32>(0)?,
            r.get::<_, u32>(1)?,
            r.get::<_, u64>(2)?,
        ))
    })?;
    let mut creature_weights = std::collections::HashMap::new();
    let mut scenario_event_weights = std::collections::HashMap::new();
    for row in rows {
        let (kind, asset, weight) = row?;
        match kind {
            0 => {
                creature_weights.insert(asset, weight);
            }
            92 => {
                scenario_event_weights.insert(asset, weight);
            }
            _ => return Err(rusqlite::Error::InvalidQuery),
        }
    }
    let mut statement=conn.prepare("SELECT c.Asset,e.Name_lang FROM criteria_tree t JOIN criteria c ON c.ID=t.CriteriaID JOIN dungeon_encounter e ON e.ID=c.Asset WHERE t.Parent=?1 AND c.Type=165 ORDER BY t.ID")?;
    let bosses = statement
        .query_map([tree], |r| {
            Ok(shared::protocol::ChallengeBoss {
                encounter_id: r.get(0)?,
                name: r.get(1)?,
                defeated: false,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if bosses.is_empty() || forces_required == 0 || timers.contains(&0) {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(Some(ChallengeDefinition {
        challenge_id: 401,
        map_id,
        name,
        timers_ms: timers,
        forces_required,
        creature_weights,
        scenario_event_weights,
        bosses,
    }))
}

#[derive(Clone, Copy, Debug)]
pub struct FontContext {
    pub font_type: u32,
    pub player_map: (u32, u32),
    pub font_map: (u32, u32),
    pub challenge_map: u32,
    pub difficulty: u32,
    pub reaches: bool,
    pub alive: bool,
    pub fresh: bool,
}
pub fn validate_font(c: FontContext) -> Result<(), &'static str> {
    if c.font_type != 49 {
        return Err("Not a Font of Power");
    }
    if c.player_map != c.font_map || c.player_map.0 != c.challenge_map || c.player_map.1 == 0 {
        return Err("Keystone and Font must belong to this dungeon copy");
    }
    if !matches!(c.difficulty, 23 | 8) {
        return Err("Requires Mythic difficulty before keystone activation");
    }
    if !c.alive {
        return Err("Cannot activate while dead");
    }
    if !c.reaches {
        return Err("Font of Power is too far away");
    }
    if !c.fresh {
        return Err("Dungeon copy already has encounter progress");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mythic_catalog_authored_azure_and_slot_authority() {
        let path =
            std::env::var("MYTHIC_PLUS_WORLD_COPY").expect("private imported SQLite copy required");
        let conn =
            Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let d = load_azure(&conn).unwrap().expect("qualified Azure Vault");
        assert_eq!(d.timers_ms, [2250000, 1800000, 1350000]);
        assert_eq!(d.forces_required, 410);
        assert_eq!(d.creature_weights.get(&187160), Some(&5));
        assert_eq!(d.scenario_event_weights.get(&77282), Some(&5));
        assert_eq!(
            d.bosses.iter().map(|b| b.encounter_id).collect::<Vec<_>>(),
            vec![2582, 2585, 2583, 2584]
        );
        let c = FontContext {
            font_type: 49,
            player_map: (2515, 7),
            font_map: (2515, 7),
            challenge_map: 2515,
            difficulty: 8,
            reaches: true,
            alive: true,
            fresh: true,
        };
        assert!(validate_font(c).is_ok());
        for bad in [
            FontContext {
                font_map: (2515, 8),
                ..c
            },
            FontContext { difficulty: 1, ..c },
            FontContext {
                reaches: false,
                ..c
            },
            FontContext { alive: false, ..c },
            FontContext { fresh: false, ..c },
            FontContext { font_type: 10, ..c },
        ] {
            assert!(validate_font(bad).is_err(), "{bad:?}");
        }
    }
}
