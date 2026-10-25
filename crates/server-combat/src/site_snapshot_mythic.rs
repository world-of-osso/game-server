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
    realm: &str,
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
    let has_affixes: bool = world.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='keystone_affix')",
        [],
        |r| r.get(0),
    )?;
    runs.into_iter().map(|run| {
        let rank=ranks.entry((run.season_id,run.challenge_id)).or_insert(0_u32); *rank+=1;
        let party=run.party.into_iter().map(|p| {
            let role=match p.spec_id {
                Some(spec)=>world.query_row("SELECT Role FROM chr_specialization WHERE ID=?1",[spec],|r|r.get::<_,u32>(0)).optional()?,
                None=>None,
            };
            let role=match role { Some(0)=>Some("tank"),Some(1)=>Some("healer"),Some(2)=>Some("damage"),_=>None };
            let class=super::catalog::label(world,"chr_classes","Name_lang",u32::from(p.class_id))?;
            let spec=p.spec_id.map(|id|super::catalog::label(world,"chr_specialization","Name_lang",id)).transpose()?;
            Ok(json!({"character_id":p.character_id,"name":p.name,"realm":realm,"class_id":p.class_id,"class_name":class["name"],"spec_id":p.spec_id,"spec_name":spec.map(|s|s["name"].clone()),"role":role}))
        }).collect::<Result<Vec<_>>>()?;
        let affixes=run.affixes.into_iter().map(|id| {
            let name:Option<String>=if has_affixes { world.query_row("SELECT Name_lang FROM keystone_affix WHERE ID=?1",[id],|r|r.get(0)).optional()? } else { None };
            Ok(json!({"id":id,"name":name}))
        }).collect::<Result<Vec<_>>>()?;
        Ok(json!({"id":run.id,"season_id":run.season_id,"dungeon":{"id":run.challenge_id,"map_id":run.map_id,"name":run.name},"level":run.level,"affixes":affixes,"elapsed_ms":run.elapsed_ms,"time_lost_ms":run.time_lost_ms,"timer_ms":run.timer_ms,"in_time":run.in_time,"completed_at":run.completed_at,"party":party,"rank":rank}))
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
