use super::Result;
use redb::{ReadTransaction, ReadableTable};
use rusqlite::Connection;
use serde_json::{Value, json};
use server::raid_history_store::{FIRST_KILLS, FirstKill};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct Rankings {
    pub characters: Vec<Value>,
    pub guilds: Vec<Value>,
}

type GroupKey = (bool, String, u32, u32);
pub fn read_rankings(
    txn: &ReadTransaction,
    world: &Connection,
    public: &BTreeSet<u64>,
    as_of: i64,
    realm: &str,
) -> Result<Rankings> {
    let table = match txn.open_table(FIRST_KILLS) {
        Ok(t) => t,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Rankings::default()),
        Err(e) => return Err(e.into()),
    };
    let mut groups: BTreeMap<GroupKey, Vec<FirstKill>> = BTreeMap::new();
    for row in table.iter()? {
        let (_, bytes) = row?;
        let kill: FirstKill = serde_json::from_slice(bytes.value())?;
        if kill.first_kill_at > as_of {
            continue;
        }
        let subject = match (&kill.character_id, &kill.guild_id) {
            (Some(id), None) if public.contains(&id.parse()?) => Some((false, id.clone())),
            (None, Some(id)) if kill.guild_group_credit => {
                let members: Vec<u64> = kill
                    .participants
                    .iter()
                    .map(|p| p.character_id.parse())
                    .collect::<std::result::Result<_, _>>()?;
                (!members.is_empty() && members.iter().all(|id| public.contains(id)))
                    .then(|| (true, id.clone()))
            }
            (Some(_), None) | (None, Some(_)) => None,
            _ => return Err("raid first kill requires exactly one subject".into()),
        };
        if let Some((guild, id)) = subject {
            groups
                .entry((guild, id, kill.map_id, kill.difficulty_id))
                .or_default()
                .push(kill);
        }
    }
    let mut rankings = Rankings::default();
    for ((guild, id, map, difficulty), kills) in groups {
        let mut row = query_progression(world, &id, guild, map, difficulty, &kills)?;
        row["realm"] = json!(realm);
        if guild {
            row["name"] = row["guild_name"].clone();
            rankings.guilds.push(row);
        } else {
            let data = super::store::bytes(txn, "characters", id.parse()?)?
                .ok_or_else(|| format!("ranked character {id} missing"))?;
            row["name"] =
                json!(crate::character_data::CharacterData::decode_compatible(&data)?.name);
            rankings.characters.push(row);
        }
    }
    rank_progression(&mut rankings.characters, "character_id");
    rank_progression(&mut rankings.guilds, "guild_id");
    Ok(rankings)
}
fn query_progression(
    world: &Connection,
    id: &str,
    guild: bool,
    map: u32,
    difficulty: u32,
    kills: &[FirstKill],
) -> Result<Value> {
    let name: String = world.query_row(
        "SELECT MapName_lang FROM map WHERE ID=?1 AND InstanceType=2",
        [map],
        |r| r.get(0),
    )?;
    let mut by_bit: BTreeMap<u32, i64> = BTreeMap::new();
    for kill in kills {
        let bit:u32=world.query_row("SELECT Bit FROM dungeon_encounter WHERE ID=?1 AND MapID=?2 AND (DifficultyID<=0 OR DifficultyID=?3)",[kill.encounter_id,map,difficulty],|r|r.get(0))?;
        by_bit
            .entry(bit)
            .and_modify(|time| *time = (*time).min(kill.first_kill_at))
            .or_insert(kill.first_kill_at);
    }
    let mut query=world.prepare("SELECT ID,Name_lang,Bit FROM dungeon_encounter WHERE MapID=?1 AND (DifficultyID<=0 OR DifficultyID=?2) ORDER BY Bit,DifficultyID DESC,ID")?;
    let rows = query.query_map([map, difficulty], |r| {
        Ok((
            r.get::<_, u32>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, u32>(2)?,
        ))
    })?;
    let mut seen = BTreeSet::new();
    let mut encounters = Vec::new();
    for row in rows {
        let (encounter, name, bit) = row?;
        if seen.insert(bit) {
            encounters.push(json!({"id":encounter,"name":name,"killed":by_bit.contains_key(&bit),"first_kill_at":by_bit.get(&bit)}));
        }
    }
    let mut row = json!({"raid":{"id":map,"name":name},"difficulty_id":difficulty,"scope":"post_deploy_lifetime","kills":by_bit.len(),"total":encounters.len(),"encounters":encounters,"first_kill_at":by_bit.values().min(),"progression_at":by_bit.values().max()});
    if guild {
        row["guild_id"] = json!(id);
        row["guild_name"] = json!(kills[0].guild_name);
        row["guild_group_credit"] = json!(true);
    } else {
        row["character_id"] = json!(id);
    }
    Ok(row)
}
fn rank_progression(rows: &mut [Value], subject: &str) {
    rows.sort_by_key(|r| {
        (
            r["raid"]["id"].as_u64(),
            r["difficulty_id"].as_u64(),
            std::cmp::Reverse(r["kills"].as_u64()),
            r["first_kill_at"].as_i64(),
            r[subject].as_str().and_then(|s| s.parse::<u64>().ok()),
        )
    });
    let mut ranks = BTreeMap::new();
    for row in rows {
        let rank = ranks
            .entry((row["raid"]["id"].as_u64(), row["difficulty_id"].as_u64()))
            .or_insert(0_u32);
        *rank += 1;
        row["rank"] = json!(rank);
    }
}
