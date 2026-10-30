//! Replay the pinned SimC tank dummy's pre-outcome pressure against a follower.
use bevy::prelude::*;
use shared::components::Health;

use super::super::healer_intake::{Capture, Hit as RecordedHit, HitKind};
use super::super::{healer_replay, tank_replay};
use crate::simc_tank_timeline::{self, Hit, Kind, Scenario};

pub(super) struct TankScenario {
    scenario: Scenario,
    hits: Vec<Hit>,
    next_hit: usize,
    next_heal: usize,
    source: Entity,
    tank: Entity,
}

impl TankScenario {
    pub(super) fn read(
        path: &std::path::Path,
        world: &mut World,
        source: Entity,
        tank: Entity,
        level: u8,
    ) -> Result<Self, String> {
        let scenario = simc_tank_timeline::read_scenario(path)?;
        let hits = simc_tank_timeline::read_trace(path, &scenario.dummy)?;
        tank_replay::configure_trace_attacker(world, source, level, &scenario)?;
        healer_replay::freeze_sources(world, &[(0, source)].into());
        world.insert_resource(Capture {
            members: vec![tank],
            sources: [(source, (0, 0))].into(),
            ..Default::default()
        });
        world
            .entity_mut(tank)
            .insert(crate::test_support::SimcInfiniteHealth);
        crate::combat_refs::set_in_combat_with(world, tank, source);
        Ok(Self {
            scenario,
            hits,
            next_hit: 0,
            next_heal: 0,
            source,
            tank,
        })
    }

    /// SimC76f8118 sc_enemy.cpp:1478–1531: default tank dummy schedules swings,
    /// nuke, bleed and a five-second healer. The trace supplies its rolls and clock.
    pub(super) fn advance(&mut self, world: &mut World, elapsed_ms: u64) -> Result<(), String> {
        while let Some(&(time, amount)) = self
            .scenario
            .heals
            .get(self.next_heal)
            .filter(|(time, _)| *time <= elapsed_ms)
        {
            self.next_heal += 1;
            if time == 0 {
                continue;
            }
            let effective = {
                let mut health = world
                    .get_mut::<Health>(self.tank)
                    .ok_or("TANK_HEALTH_ABSENT")?;
                let effective = amount.min((health.max - health.current).max(0.0));
                health.current += effective;
                effective
            };
            crate::spell_cast::script_registry::heal_received(
                world, self.tank, 0, amount, effective, false,
            );
        }
        while let Some(hit) = self
            .hits
            .get(self.next_hit)
            .filter(|hit| hit.time_ms <= elapsed_ms)
        {
            let _attack = crate::random::attack_scope(self.next_hit as u64);
            let scale = crate::level_scaling::damage_to_hp_scale(world, self.source, self.tank);
            world.resource_mut::<Capture>().active_hit_scale = Some((self.tank, scale));
            match hit.kind {
                Kind::BleedApplication => {
                    crate::spell_cast::effects::replay_simc_dummy_bleed_application(
                        world,
                        self.source,
                        self.tank,
                    )
                }
                Kind::Bleed => crate::spell_cast::effects::replay_simc_dummy_bleed(
                    world,
                    self.source,
                    self.tank,
                    hit.amount as f32,
                ),
                Kind::Swing | Kind::Strike => {
                    let table = hit.melee_table.ok_or("SIMC_DUMMY_MELEE_TABLE_ABSENT")?;
                    let (outcome, level_difference) = crate::combat::roll_simc_dummy_outcome(
                        world,
                        self.source,
                        self.tank,
                        hit.kind == Kind::Strike,
                        table.miss,
                        table.crit,
                    )?;
                    let recorded = RecordedHit {
                        tick: (elapsed_ms / 50) as u32,
                        source_guid: 0,
                        pack: 0,
                        target_slot: 0,
                        school: 1,
                        raw: hit.amount as f32,
                        level_scaling_factor: scale,
                        kind: HitKind::Melee {
                            hand: 0,
                            outcome: format!("{outcome:?}"),
                            level_difference,
                            critical_multiplier: 1.0,
                            weapon_speed: 2.0,
                        },
                    };
                    if hit.kind == Kind::Strike {
                        let amount = tank_replay::source_amount(&recorded)? as f32;
                        crate::spell_cast::effects::replay_simc_dummy_strike(
                            world,
                            self.source,
                            self.tank,
                            amount,
                            matches!(
                                outcome,
                                crate::melee::MeleeOutcome::Crit
                                    | crate::melee::MeleeOutcome::CritBlock
                            ),
                            matches!(
                                outcome,
                                crate::melee::MeleeOutcome::Block
                                    | crate::melee::MeleeOutcome::CritBlock
                            ),
                        );
                    } else {
                        crate::combat::replay_healer_hit(world, self.source, self.tank, &recorded)?;
                    }
                }
            }
            world.resource_mut::<Capture>().active_hit_scale = None;
            self.next_hit += 1;
        }
        Ok(())
    }
}
