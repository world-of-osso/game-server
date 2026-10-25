//! Challenge lifecycle over authored MapChallengeMode thresholds and scenario criteria.
use shared::protocol::{ChallengeBoss, ChallengePhase, ChallengeRun, KeystoneInfo};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UnsourcedChallengeConfig {
    pub unsourced_pre_start_countdown_ms: u64,
    pub unsourced_death_penalty_ms: u64,
    pub unsourced_minimum_keystone_level: u32,
    pub unsourced_seasonal_affix_rotation: Vec<HashMap<u32, Vec<u32>>>,
    pub unsourced_season_period_secs: u64,
    pub unsourced_season_epoch_unix: i64,
    pub unsourced_rating_by_level: HashMap<u32, u32>,
}
impl UnsourcedChallengeConfig {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.unsourced_season_period_secs > 0
            && self.unsourced_seasonal_affix_rotation.is_empty()
        {
            return Err("Enabled unsourced season period requires explicit rotation rows");
        }
        Ok(())
    }
    pub fn affixes_at(&self, unix: i64, level: u32) -> Vec<u32> {
        if self.unsourced_season_period_secs == 0 || unix < self.unsourced_season_epoch_unix {
            return Vec::new();
        }
        let elapsed = unix.saturating_sub(self.unsourced_season_epoch_unix) as u64;
        let period = elapsed / self.unsourced_season_period_secs;
        let index = (period % self.unsourced_seasonal_affix_rotation.len() as u64) as usize;
        self.unsourced_seasonal_affix_rotation[index]
            .get(&level)
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub struct ChallengeDefinition {
    pub challenge_id: u32,
    pub map_id: u32,
    pub name: String,
    pub timers_ms: [u64; 3],
    pub forces_required: u64,
    pub creature_weights: HashMap<u32, u64>,
    pub scenario_event_weights: HashMap<u32, u64>,
    pub bosses: Vec<ChallengeBoss>,
}
#[derive(Clone, Debug)]
pub struct Run {
    pub snapshot: ChallengeRun,
    pub definition: ChallengeDefinition,
    pub config: UnsourcedChallengeConfig,
    start_ms: u64,
    killed: HashSet<u64>,
    scenario_events: HashSet<u64>,
}
impl Run {
    pub fn new(
        id: u64,
        instance: u32,
        key: KeystoneInfo,
        definition: ChallengeDefinition,
        config: UnsourcedChallengeConfig,
    ) -> Self {
        Self {
            snapshot: ChallengeRun {
                run_id: id,
                map_id: definition.map_id,
                instance_id: instance,
                name: definition.name.clone(),
                key,
                phase: ChallengePhase::Slotted,
                countdown_ms: 0,
                elapsed_ms: 0,
                time_limit_ms: definition.timers_ms[0],
                deaths: 0,
                time_lost_ms: 0,
                forces: 0,
                forces_required: definition.forces_required,
                bosses: definition.bosses.clone(),
                upgrade_levels: 0,
                resulting_level: None,
                rating: None,
            },
            definition,
            config,
            start_ms: 0,
            killed: HashSet::new(),
            scenario_events: HashSet::new(),
        }
    }
    pub fn start(&mut self, now: u64) -> Result<(), &'static str> {
        if self.snapshot.phase != ChallengePhase::Slotted {
            return Err("This keystone run has already started");
        }
        self.start_ms = now.saturating_add(self.config.unsourced_pre_start_countdown_ms);
        self.snapshot.phase = ChallengePhase::Countdown;
        self.tick(now);
        Ok(())
    }
    pub fn tick(&mut self, now: u64) {
        if self.snapshot.phase == ChallengePhase::Countdown {
            self.snapshot.countdown_ms = self.start_ms.saturating_sub(now);
            if now < self.start_ms {
                return;
            }
            self.snapshot.phase = ChallengePhase::Running;
        }
        if self.snapshot.phase != ChallengePhase::Running {
            return;
        }
        self.snapshot.elapsed_ms = now
            .saturating_sub(self.start_ms)
            .saturating_add(self.snapshot.time_lost_ms);
        if self.snapshot.forces >= self.snapshot.forces_required
            && self.snapshot.bosses.iter().all(|boss| boss.defeated)
        {
            self.complete();
        }
    }
    pub fn kill(&mut self, spawn: u64, entry: u32) {
        if self.snapshot.phase != ChallengePhase::Running || !self.killed.insert(spawn) {
            return;
        }
        if let Some(weight) = self.definition.creature_weights.get(&entry) {
            self.snapshot.forces = self.snapshot.forces.saturating_add(*weight);
        }
    }
    pub fn scenario_event(&mut self, receipt: u64, event: u32) {
        if self.snapshot.phase != ChallengePhase::Running || !self.scenario_events.insert(receipt) {
            return;
        }
        if let Some(weight) = self.definition.scenario_event_weights.get(&event) {
            self.snapshot.forces = self.snapshot.forces.saturating_add(*weight);
        }
    }
    pub fn boss(&mut self, encounter: u32) {
        if self.snapshot.phase != ChallengePhase::Running {
            return;
        }
        if let Some(boss) = self
            .snapshot
            .bosses
            .iter_mut()
            .find(|boss| boss.encounter_id == encounter)
        {
            boss.defeated = true;
        }
    }
    pub fn death(&mut self) {
        if self.snapshot.phase != ChallengePhase::Running {
            return;
        }
        self.snapshot.deaths += 1;
        self.snapshot.time_lost_ms = self
            .snapshot
            .time_lost_ms
            .saturating_add(self.config.unsourced_death_penalty_ms);
    }
    pub fn depleted_level(&self) -> u32 {
        self.snapshot
            .key
            .level
            .saturating_sub(1)
            .max(self.config.unsourced_minimum_keystone_level)
    }

    pub fn abandon(&mut self) {
        if !matches!(
            self.snapshot.phase,
            ChallengePhase::Countdown | ChallengePhase::Running
        ) {
            return;
        }
        self.snapshot.phase = ChallengePhase::Abandoned;
        self.snapshot.resulting_level = Some(self.depleted_level());
    }
    fn complete(&mut self) {
        self.snapshot.phase = ChallengePhase::Completed;
        self.snapshot.upgrade_levels = self
            .definition
            .timers_ms
            .iter()
            .filter(|&&limit| self.snapshot.elapsed_ms <= limit)
            .count() as u32;
        let level = self.snapshot.key.level;
        self.snapshot.resulting_level = Some(if self.snapshot.upgrade_levels == 0 {
            self.depleted_level()
        } else {
            level.saturating_add(self.snapshot.upgrade_levels)
        });
        self.snapshot.rating = self.config.unsourced_rating_by_level.get(&level).copied();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run() -> Run {
        Run::new(
            1,
            7,
            KeystoneInfo {
                item_guid: 42,
                item_id: 180653,
                challenge_id: 401,
                level: 8,
                affixes: vec![],
            },
            ChallengeDefinition {
                challenge_id: 401,
                map_id: 2515,
                name: "The Azure Vault".into(),
                timers_ms: [2250000, 1800000, 1350000],
                forces_required: 410,
                creature_weights: HashMap::from([(187160, 5)]),
                scenario_event_weights: HashMap::from([(77282, 5)]),
                bosses: [2582, 2585, 2583, 2584]
                    .into_iter()
                    .map(|encounter_id| ChallengeBoss {
                        encounter_id,
                        name: encounter_id.to_string(),
                        defeated: false,
                    })
                    .collect(),
            },
            UnsourcedChallengeConfig::default(),
        )
    }
    fn clear(run: &mut Run) {
        for spawn in 0..82 {
            run.kill(spawn, 187160);
        }
        for id in [2582, 2585, 2583, 2584] {
            run.boss(id);
        }
    }
    #[test]
    fn mythic_runtime_weighted_forces_and_boss_completion() {
        let mut r = run();
        r.start(1000).unwrap();
        r.kill(1, 187160);
        r.kill(1, 187160);
        r.kill(2, 999);
        assert_eq!(r.snapshot.forces, 5);
        r.scenario_event(9, 77282);
        r.scenario_event(9, 77282);
        assert_eq!(r.snapshot.forces, 10);
        r.boss(2582);
        assert_eq!(r.snapshot.phase, ChallengePhase::Running);
        r.tick(901000);
        clear(&mut r);
        r.tick(901000);
        assert_eq!(r.snapshot.phase, ChallengePhase::Completed);
        assert_eq!(r.snapshot.upgrade_levels, 3);
        assert_eq!(r.snapshot.resulting_level, Some(11));
        assert_eq!(r.snapshot.rating, None);
        let completed = r.snapshot.clone();
        r.death();
        r.tick(3000000);
        clear(&mut r);
        assert_eq!(r.snapshot, completed);
    }
    #[test]
    fn mythic_runtime_overtime_depletes_once() {
        let mut r = run();
        r.start(1000).unwrap();
        r.tick(2251001);
        clear(&mut r);
        r.tick(2251001);
        assert_eq!(r.snapshot.resulting_level, Some(7));
        assert_eq!(r.snapshot.upgrade_levels, 0);
        assert_eq!(r.snapshot.phase, ChallengePhase::Completed);
        assert!(r.start(3000000).is_err());
    }
    #[test]
    fn mythic_runtime_countdown_death_and_abandon_policy() {
        let mut r = run();
        r.config.unsourced_pre_start_countdown_ms = 4000;
        r.config.unsourced_death_penalty_ms = 15000;
        r.start(1000).unwrap();
        r.kill(1, 187160);
        r.death();
        assert_eq!(r.snapshot.forces, 0);
        assert_eq!(r.snapshot.deaths, 0);
        r.tick(4999);
        assert_eq!(r.snapshot.countdown_ms, 1);
        r.tick(5000);
        r.death();
        r.tick(6000);
        assert_eq!(r.snapshot.elapsed_ms, 16000);
        assert_eq!(r.snapshot.time_lost_ms, 15000);
        r.abandon();
        assert_eq!(r.snapshot.resulting_level, Some(7));
        assert_eq!(r.snapshot.phase, ChallengePhase::Abandoned);
    }
    #[test]
    fn mythic_runtime_unsourced_rotation_is_disabled_until_a_period_is_supplied() {
        let mut config = UnsourcedChallengeConfig::default();
        config.unsourced_seasonal_affix_rotation = vec![
            HashMap::from([(8, vec![147])]),
            HashMap::from([(8, vec![165])]),
        ];
        config.unsourced_season_epoch_unix = 100;
        assert!(config.affixes_at(100, 8).is_empty());
        config.unsourced_season_period_secs = 2; // Fixture policy input, not a Retail season duration.
        assert_eq!(config.affixes_at(100, 8), vec![147]);
        assert_eq!(config.affixes_at(102, 8), vec![165]);
        assert_eq!(config.affixes_at(104, 8), vec![147]);
        assert!(config.affixes_at(99, 8).is_empty());
    }

    #[test]
    fn mythic_runtime_minimum_key_level_requires_explicit_unsourced_policy() {
        let mut r = run();
        r.snapshot.key.level = 2;
        assert_eq!(r.depleted_level(), 1);
        r.config.unsourced_minimum_keystone_level = 2; // Supplied fixture policy, not a sourced Retail minimum.
        assert_eq!(r.depleted_level(), 2);
    }

    #[test]
    fn mythic_runtime_unknown_policy_defaults_disabled() {
        let mut r = run();
        r.start(1000).unwrap();
        r.death();
        r.tick(2000);
        assert_eq!(r.snapshot.phase, ChallengePhase::Running);
        assert_eq!(r.snapshot.elapsed_ms, 1000);
        assert_eq!(r.snapshot.deaths, 1);
        assert_eq!(r.snapshot.time_lost_ms, 0);
        assert!(r.snapshot.key.affixes.is_empty());
    }
}
