use super::Result;
use redb::{ReadTransaction, ReadableTable};
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};
use server::mythic_plus_history::{COMPLETED_RUNS, CompletedRun};
use std::collections::{BTreeMap, BTreeSet};

pub fn read_rankings(
    txn: &ReadTransaction,
    world: &Connection,
    public: &BTreeSet<u64>,
    as_of: i64,
) -> Result<Vec<Value>> {
    let table = match txn.open_table(COMPLETED_RUNS) {
        Ok(t) => t,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut runs = Vec::new();
    for row in table.iter()? {
        let (id, bytes) = row?;
        let run: CompletedRun = serde_json::from_slice(bytes.value())?;
        if run.id != id.value().to_string()
            || run.timer_ms == 0
            || run.in_time != (run.elapsed_ms <= run.timer_ms)
        {
            return Err(format!("invalid completion record {}", id.value()).into());
        }
        let party_ids: Vec<u64> = run
            .party
            .iter()
            .map(|p| p.character_id.parse())
            .collect::<std::result::Result<_, _>>()?;
        let eligible = !party_ids.is_empty() && party_ids.iter().all(|id| public.contains(id));
        if eligible && run.completed_at <= as_of {
            runs.push(run);
        }
    }
    runs.sort_by_key(|r| {
        (
            r.season_id,
            r.challenge_id,
            !r.in_time,
            std::cmp::Reverse(r.level),
            r.elapsed_ms,
            r.id.parse::<u64>().expect("validated row id"),
        )
    });
    let mut ranks = BTreeMap::new();
    runs.into_iter().map(|run| {
        let rank=ranks.entry((run.season_id,run.challenge_id)).or_insert(0_u32); *rank+=1;
        let party=run.party.into_iter().map(|p| {
            let role=match p.spec_id {
                Some(spec)=>world.query_row("SELECT Role FROM chr_specialization WHERE ID=?1",[spec],|r|r.get::<_,u32>(0)).optional()?,
                None=>None,
            };
            Ok(json!({"character_id":p.character_id,"name":p.name,"class_id":p.class_id,"spec_id":p.spec_id,"role":role}))
        }).collect::<Result<Vec<_>>>()?;
        Ok(json!({"id":run.id,"season_id":run.season_id,"dungeon":{"id":run.challenge_id,"map_id":run.map_id,"name":run.name},"map_id":run.map_id,"level":run.level,"affixes":run.affixes,"elapsed_ms":run.elapsed_ms,"time_lost_ms":run.time_lost_ms,"timer_ms":run.timer_ms,"in_time":run.in_time,"completed_at":run.completed_at,"party":party,"rank":rank}))
    }).collect()
}

pub fn character_best(rankings: &[Value], id: u64) -> Vec<Value> {
    let id = id.to_string();
    let mut selected = BTreeSet::new();
    rankings
        .iter()
        .filter(|run| {
            run["party"]
                .as_array()
                .is_some_and(|party| party.iter().any(|p| p["character_id"] == id))
        })
        .filter(|run| selected.insert((run["season_id"].as_u64(), run["dungeon"]["id"].as_u64())))
        .cloned()
        .collect()
}
