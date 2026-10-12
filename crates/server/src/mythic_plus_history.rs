//! Attributable completion history for offline public snapshots. Existing wire/bitcode
//! ChallengeRun ABI stays untouched; old un-attributed history is never backfilled.
use redb::TableDefinition;
use serde::{Deserialize, Serialize};
use shared::protocol::{ChallengePhase, ChallengeRun};

pub const COMPLETED_RUNS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("mythic_completed_runs_v1");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Participant {
    // Decimal string preserves u64 identity in JavaScript.
    pub character_id: String,
    pub name: String,
    pub class_id: u8,
    pub spec_id: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletedRun {
    pub id: String,
    pub season_id: Option<u32>,
    pub challenge_id: u32,
    pub map_id: u32,
    pub name: String,
    pub level: u32,
    pub affixes: Vec<u32>,
    pub elapsed_ms: u64,
    pub timer_ms: u64,
    pub time_lost_ms: u64,
    pub in_time: bool,
    pub completed_at: i64,
    pub party: Vec<Participant>,
}
impl CompletedRun {
    pub fn from_completed(
        summary: &ChallengeRun,
        mut party: Vec<Participant>,
        completed_at: i64,
    ) -> Option<Self> {
        if summary.phase != ChallengePhase::Completed {
            return None;
        }
        party.sort_by_key(|p| {
            p.character_id
                .parse::<u64>()
                .expect("server-generated character id")
        });
        Some(Self {
            id: summary.run_id.to_string(),
            // Imported historical seasons are not an active-season selection policy.
            season_id: None,
            challenge_id: summary.key.challenge_id,
            map_id: summary.map_id,
            name: summary.name.clone(),
            level: summary.key.level,
            affixes: summary.key.affixes.clone(),
            elapsed_ms: summary.elapsed_ms,
            timer_ms: summary.time_limit_ms,
            time_lost_ms: summary.time_lost_ms,
            in_time: summary.elapsed_ms <= summary.time_limit_ms,
            completed_at,
            party,
        })
    }
}
