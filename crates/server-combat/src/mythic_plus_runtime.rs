//! Challenge lifecycle over authored MapChallengeMode thresholds and scenario criteria.
use crate::mythic_plus_policy::ChallengePolicy;
use shared::protocol::{ChallengeBoss, ChallengePhase, ChallengeRun, KeystoneInfo};
use std::collections::{HashMap, HashSet};

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
    pub policy: ChallengePolicy,
    start_ms: u64,
    combat_res_charges: u8,
    combat_res_next_ms: u64,
    killed: HashSet<u64>,
    scenario_events: HashSet<u64>,
    next_scenario_receipt: u64,
}
impl Run {
    pub fn new(id: u64, instance: u32, key: KeystoneInfo, definition: ChallengeDefinition) -> Self {
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
            policy: ChallengePolicy::retail(),
            start_ms: 0,
            combat_res_charges: 1,
            combat_res_next_ms: 0,
            killed: HashSet::new(),
            scenario_events: HashSet::new(),
            next_scenario_receipt: 0,
        }
    }
    pub fn start(&mut self, now: u64) -> Result<(), &'static str> {
        if self.snapshot.phase != ChallengePhase::Slotted {
            return Err("This keystone run has already started");
        }
        self.start_ms = now.saturating_add(self.policy.countdown_ms.value);
        self.combat_res_next_ms = self.start_ms.saturating_add(600_000);
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
    /// Shared Mythic+ battle-resurrection pool; Wowhead Mythic Keystone guide.
    pub fn combat_res_charges(&mut self, now: u64) -> u8 {
        if self.snapshot.phase != ChallengePhase::Running {
            return 0;
        }
        if now >= self.combat_res_next_ms {
            let earned = 1 + (now - self.combat_res_next_ms) / 600_000;
            self.combat_res_charges = (u64::from(self.combat_res_charges) + earned).min(5) as u8;
            self.combat_res_next_ms = self
                .combat_res_next_ms
                .saturating_add(earned.saturating_mul(600_000));
        }
        self.combat_res_charges
    }

    pub fn consume_combat_res(&mut self, now: u64) -> bool {
        if self.combat_res_charges(now) == 0 {
            return false;
        }
        self.combat_res_charges -= 1;
        true
    }

    pub fn kill(&mut self, spawn: u64, entry: u32) {
        if self.snapshot.phase != ChallengePhase::Running || !self.killed.insert(spawn) {
            return;
        }
        if let Some(weight) = self.definition.creature_weights.get(&entry) {
            self.snapshot.forces = self.snapshot.forces.saturating_add(*weight);
        }
    }
    pub fn emit_scenario_event(&mut self, event: u32) {
        let receipt = self.next_scenario_receipt;
        self.next_scenario_receipt = self
            .next_scenario_receipt
            .checked_add(1)
            .expect("scenario event receipts exhausted");
        self.scenario_event(receipt, event);
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
            .saturating_add(self.policy.death_penalty_ms(self.snapshot.key.level));
    }
    pub fn depleted_level(&self) -> u32 {
        self.snapshot.key.level.saturating_sub(1)
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
            .policy
            .upgrade_levels(self.snapshot.elapsed_ms, self.snapshot.time_limit_ms);
        let level = self.snapshot.key.level;
        self.snapshot.resulting_level = Some(if self.snapshot.upgrade_levels == 0 {
            self.depleted_level()
        } else {
            level.saturating_add(self.snapshot.upgrade_levels)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run() -> Run {
        let mut run = Run::new(
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
        );
        run.policy.countdown_ms.value = 0;
        run.policy.death_bands.value.clear();
        run
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
    fn mythic_runtime_combat_res_charges_regenerate_and_cap() {
        let mut r = run();
        r.start(1000).unwrap();
        assert!(r.consume_combat_res(1000));
        assert!(!r.consume_combat_res(600999));
        assert!(r.consume_combat_res(601000));
        assert_eq!(r.combat_res_charges(4_201_000), 5);
        for _ in 0..5 {
            assert!(r.consume_combat_res(4_201_000));
        }
        assert!(!r.consume_combat_res(4_201_000));
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
        r.policy.countdown_ms.value = 4000;
        r.policy.death_bands.value = vec![crate::mythic_plus_policy::DeathBand {
            min_level: 2,
            milliseconds: 15000,
        }];
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
    fn mythic_runtime_retail_upgrades_follow_timer_fractions() {
        for (elapsed, upgrades) in [(600, 3), (601, 2), (800, 2), (801, 1), (1000, 1), (1001, 0)] {
            let mut run = run();
            run.snapshot.time_limit_ms = 1000;
            run.start(0).unwrap();
            clear(&mut run);
            run.tick(elapsed);
            assert_eq!(run.snapshot.upgrade_levels, upgrades, "elapsed {elapsed}");
        }
    }

    #[test]
    fn mythic_runtime_retail_deaths_use_key_level_band() {
        for (level, penalty) in [(2, 0), (5, 0), (6, 5000), (11, 5000), (12, 15000)] {
            let mut run = run();
            run.snapshot.key.level = level;
            run.policy = ChallengePolicy::retail();
            run.start(1000).unwrap();
            run.death();
            assert_eq!(run.snapshot.deaths, 0);
            run.tick(11000);
            run.death();
            assert_eq!(run.snapshot.time_lost_ms, penalty);
        }
    }

    #[test]
    fn mythic_runtime_unsourced_minimum_floor_is_not_assumed() {
        let mut r = run();
        r.snapshot.key.level = 2;
        assert_eq!(r.depleted_level(), 1);
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
