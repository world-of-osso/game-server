//! Versioned run record and event-driven metrics, measured on simulation time.
use std::collections::HashMap;

use serde::Serialize;
use shared::protocol::{CombatLogEvent, CombatLogKind};

#[derive(Debug, Serialize)]
pub(super) struct ItemLevel {
    pub slot: String,
    pub item_id: u32,
    pub level: u16,
}

#[derive(Debug, Serialize)]
pub(super) struct PartyMember {
    pub entity_id: String,
    pub spec_id: u32,
    pub item_levels: Vec<ItemLevel>,
    /// Arithmetic mean of the recorded equipped item levels, not a Retail UI score.
    pub mean_equipped_item_level: f64,
}

#[derive(Debug, Default, Serialize)]
pub(super) struct ActorMetrics {
    pub entity_id: String,
    pub spec_id: u32,
    pub raw_damage: u64,
    pub effective_damage: u64,
    pub raw_healing: u64,
    pub effective_healing: u64,
}

#[derive(Debug, Serialize)]
pub(super) struct Death {
    pub elapsed_ms: u64,
    pub victim_entity_id: String,
    pub victim_spec_id: u32,
    pub killer_npc_entry: Option<u32>,
    pub killer_spell_id: Option<u32>,
}

#[derive(Debug, Serialize)]
pub(super) struct Pull {
    pub start_ms: u64,
    pub duration_ms: u64,
    pub ended: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct Outcome {
    pub status: String,
    pub reason: String,
    pub elapsed_ms: u64,
    pub timer_ms: u64,
    pub in_time: Option<bool>,
    pub upgrade_levels: Option<u32>,
    pub resulting_key_level: Option<u32>,
}

#[derive(Debug, Serialize)]
pub(super) struct RunRecord {
    pub schema_version: u32,
    pub map_id: u32,
    pub challenge_id: u32,
    pub key_level: u32,
    pub affixes: Vec<u32>,
    pub party: Vec<PartyMember>,
    pub route: super::keystone_route::PlannedRoute,
    pub outcome: Outcome,
    pub deaths: Vec<Death>,
    pub pulls: Vec<Pull>,
    pub actors: Vec<ActorMetrics>,
}

/// The caller supplies observed simulation time and actual controlled-unit ownership.
pub(super) struct Tracker {
    actors: Vec<ActorMetrics>,
    actor_indices: HashMap<u64, usize>,
    deaths: Vec<Death>,
    pulls: Vec<Pull>,
    active_pull: Option<u64>,
    cursor: usize,
}

impl Tracker {
    pub fn new(party: &[(u64, u32)]) -> Self {
        let actors = party
            .iter()
            .map(|&(entity, spec_id)| ActorMetrics {
                entity_id: entity.to_string(),
                spec_id,
                ..Default::default()
            })
            .collect();
        let actor_indices = party
            .iter()
            .enumerate()
            .map(|(index, &(entity, _))| (entity, index))
            .collect();
        Self {
            actors,
            actor_indices,
            deaths: Vec::new(),
            pulls: Vec::new(),
            active_pull: None,
            cursor: 0,
        }
    }

    pub fn observe(
        &mut self,
        elapsed_ms: u64,
        in_combat: bool,
        events: &[CombatLogEvent],
        owners: &HashMap<u64, u64>,
        npc_entries: &HashMap<u64, u32>,
    ) {
        match (self.active_pull, in_combat) {
            (None, true) => self.active_pull = Some(elapsed_ms),
            (Some(start), false) => {
                self.pulls.push(Pull {
                    start_ms: start,
                    duration_ms: elapsed_ms.saturating_sub(start),
                    ended: true,
                });
                self.active_pull = None;
            }
            _ => {}
        }
        for event in &events[self.cursor..] {
            if event.kind == CombatLogKind::Death {
                self.record_death(elapsed_ms, event, npc_entries);
            }
            let Some(source) = event.source else { continue };
            let owner = owners.get(&source).copied().unwrap_or(source);
            let Some(&index) = self.actor_indices.get(&owner) else {
                continue;
            };
            let raw = event.amount.max(0) as u64;
            let effective = raw.saturating_sub(event.overflow.max(0) as u64);
            let actor = &mut self.actors[index];
            match event.kind {
                CombatLogKind::Damage => {
                    actor.raw_damage += raw;
                    actor.effective_damage += effective;
                }
                CombatLogKind::Heal => {
                    actor.raw_healing += raw;
                    actor.effective_healing += effective;
                }
                _ => {}
            }
        }
        self.cursor = events.len();
    }

    fn record_death(
        &mut self,
        elapsed_ms: u64,
        event: &CombatLogEvent,
        npc_entries: &HashMap<u64, u32>,
    ) {
        let Some(victim) = event.target else { return };
        let Some(&index) = self.actor_indices.get(&victim) else {
            return;
        };
        self.deaths.push(Death {
            elapsed_ms,
            victim_entity_id: victim.to_string(),
            victim_spec_id: self.actors[index].spec_id,
            killer_npc_entry: event
                .source
                .and_then(|source| npc_entries.get(&source).copied()),
            killer_spell_id: event.spell_id,
        });
    }

    pub fn finish(mut self, elapsed_ms: u64) -> (Vec<ActorMetrics>, Vec<Death>, Vec<Pull>) {
        if let Some(start) = self.active_pull {
            self.pulls.push(Pull {
                start_ms: start,
                duration_ms: elapsed_ms.saturating_sub(start),
                ended: false,
            });
        }
        (self.actors, self.deaths, self.pulls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(
        kind: CombatLogKind,
        source: u64,
        target: u64,
        amount: i32,
        overflow: i32,
        spell_id: Option<u32>,
    ) -> CombatLogEvent {
        CombatLogEvent {
            source: Some(source),
            target: Some(target),
            spell_id,
            school_mask: 1,
            amount,
            overflow,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit: false,
            glancing: false,
            periodic: false,
            extra_spell_id: None,
            timestamp_unix_ms: 999999,
            kind,
        }
    }

    #[test]
    fn serializes_complete_record_with_explicit_equipment_levels() {
        let record = RunRecord {
            schema_version: 1,
            map_id: 2515,
            challenge_id: 401,
            key_level: 2,
            affixes: vec![],
            party: vec![PartyMember {
                entity_id: "11".into(),
                spec_id: 253,
                item_levels: vec![ItemLevel {
                    slot: "Head".into(),
                    item_id: 271474,
                    level: 334,
                }],
                mean_equipped_item_level: 334.0,
            }],
            route: super::super::keystone_route::PlannedRoute {
                steps: vec![],
                added_packs: vec![10],
                planned_forces: 410,
            },
            outcome: Outcome {
                status: "completed".into(),
                reason: String::new(),
                elapsed_ms: 1000,
                timer_ms: 2250000,
                in_time: Some(true),
                upgrade_levels: Some(3),
                resulting_key_level: Some(5),
            },
            deaths: vec![],
            pulls: vec![],
            actors: vec![],
        };
        let json = serde_json::to_value(record).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["party"][0]["spec_id"], 253);
        assert_eq!(json["party"][0]["item_levels"][0]["level"], 334);
        assert_eq!(json["route"]["planned_forces"], 410);
        assert_eq!(json["outcome"]["resulting_key_level"], 5);
    }

    #[test]
    fn records_simulated_time_effective_amounts_and_pet_attribution() {
        let mut tracker = Tracker::new(&[(11, 253), (12, 105)]);
        let owners = HashMap::from([(21, 11)]);
        let npcs = HashMap::from([(99, 187160)]);
        let events = vec![
            event(CombatLogKind::Damage, 21, 99, 100, 20, Some(123)),
            event(CombatLogKind::Heal, 12, 11, 70, 30, Some(456)),
            event(CombatLogKind::Death, 99, 11, 0, 0, Some(789)),
        ];
        tracker.observe(250, true, &events[..2], &owners, &npcs);
        tracker.observe(750, false, &events, &owners, &npcs);
        let (actors, deaths, pulls) = tracker.finish(1000);
        assert_eq!(
            (actors[0].raw_damage, actors[0].effective_damage),
            (100, 80)
        );
        assert_eq!(
            (actors[1].raw_healing, actors[1].effective_healing),
            (70, 40)
        );
        assert_eq!(
            (
                deaths[0].elapsed_ms,
                deaths[0].victim_spec_id,
                deaths[0].killer_npc_entry,
                deaths[0].killer_spell_id
            ),
            (750, 253, Some(187160), Some(789))
        );
        assert_eq!(
            (pulls[0].start_ms, pulls[0].duration_ms, pulls[0].ended),
            (250, 500, true)
        );
    }
}
