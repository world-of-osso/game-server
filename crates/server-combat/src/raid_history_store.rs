//! Lifetime first-kill rows, separate from reset/instance locks.
use redb::{ReadableTable, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const FIRST_KILLS: TableDefinition<&str, &[u8]> = TableDefinition::new("raid_first_kills_v1");
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Participant {
    pub character_id: String,
    pub guild_id: Option<String>,
    pub guild_name: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FirstKill {
    pub character_id: Option<String>,
    pub guild_id: Option<String>,
    pub guild_name: Option<String>,
    pub map_id: u32,
    pub difficulty_id: u32,
    pub encounter_id: u32,
    pub first_kill_at: i64,
    pub participants: Vec<Participant>,
    pub guild_group_credit: bool,
}

pub fn qualifying_guilds(party: &[Participant]) -> Vec<(String, String)> {
    let mut guilds: BTreeMap<_, (usize, String)> = BTreeMap::new();
    for participant in party {
        if let (Some(id), Some(name)) = (&participant.guild_id, &participant.guild_name) {
            let entry = guilds.entry(id.clone()).or_insert((0, name.clone()));
            entry.0 += 1;
        }
    }
    guilds
        .into_iter()
        .filter(|(_, (count, _))| count * 5 >= party.len() * 4)
        .map(|(id, (_, name))| (id, name))
        .collect()
}

pub fn persist_first_kills(
    txn: &WriteTransaction,
    map_id: u32,
    difficulty_id: u32,
    encounter_id: u32,
    first_kill_at: i64,
    party: &[Participant],
) -> Result<(), Box<dyn std::error::Error>> {
    let participants = party.to_vec();
    let mut table = txn.open_table(FIRST_KILLS)?;
    let base = FirstKill {
        character_id: None,
        guild_id: None,
        guild_name: None,
        map_id,
        difficulty_id,
        encounter_id,
        first_kill_at,
        participants,
        guild_group_credit: false,
    };
    for participant in party {
        let row = FirstKill {
            character_id: Some(participant.character_id.clone()),
            ..base.clone()
        };
        let key = format!(
            "character:{}:{map_id}:{difficulty_id}:{encounter_id}",
            participant.character_id
        );
        if table.get(key.as_str())?.is_none() {
            table.insert(key.as_str(), serde_json::to_vec(&row)?.as_slice())?;
        }
    }
    for (id, name) in qualifying_guilds(party) {
        let key = format!("guild:{id}:{map_id}:{difficulty_id}:{encounter_id}");
        let row = FirstKill {
            guild_id: Some(id),
            guild_name: Some(name),
            guild_group_credit: true,
            ..base.clone()
        };
        if table.get(key.as_str())?.is_none() {
            table.insert(key.as_str(), serde_json::to_vec(&row)?.as_slice())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raid_first_kill_guild_group_exact_eighty_percent() {
        for (size, required) in [(5, 4), (10, 8), (20, 16), (25, 20)] {
            let mut party: Vec<_> = (0..size)
                .map(|id| Participant {
                    character_id: id.to_string(),
                    guild_id: (id < required).then(|| "7".into()),
                    guild_name: (id < required).then(|| "Proof Guild".into()),
                })
                .collect();
            assert_eq!(
                qualifying_guilds(&party),
                vec![("7".into(), "Proof Guild".into())]
            );
            party[required - 1].guild_id = None;
            party[required - 1].guild_name = None;
            assert!(qualifying_guilds(&party).is_empty());
        }
        assert!(qualifying_guilds(&[]).is_empty());
    }
}
