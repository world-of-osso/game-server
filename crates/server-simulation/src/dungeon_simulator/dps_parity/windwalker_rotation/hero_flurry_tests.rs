//! Real selected hero producers; no aura, stack, stat or resource grants.
use super::super::*;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::{CombatLogEvent, CombatLogKind};

const ZENITH: u32 = 1249625;
const WISDOM: u32 = 1272821;
const STAND_READY: u32 = 1262603;
const STAND_READY_BUFF: u32 = 1237196;
const STRIKE: u32 = 450617;
const AREA: u32 = 451250;

fn prepare(talent: u32) -> Run {
    let mut run = prepare_measurement(269, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|spell| spell.spell_id == talent),
        "the real profile must select the native hero talent"
    );
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    run
}

#[test]
#[ignore = "actual Zenith must produce the selected native Stand Ready buff"]
fn literal_ww_native_zenith_applies_stand_ready() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(STAND_READY);
    for _ in 0..TICKS {
        let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
        run.app.update();
        drive_player(&mut run).unwrap();
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        let actor_events: Vec<_> = events[before..]
            .iter()
            .filter(|event| event.source == Some(run.actor.to_bits()))
            .collect();
        if actor_events
            .iter()
            .any(|event| event.spell_id == Some(ZENITH) && event.kind == CombatLogKind::CastSuccess)
        {
            assert!(
                actor_events
                    .iter()
                    .any(|event| event.spell_id == Some(STAND_READY_BUFF)
                        && event.kind == CombatLogKind::AuraApplied),
                "the actual selected Zenith execute must apply native Stand Ready1237196"
            );
            return;
        }
    }
    panic!("fixture prerequisite: no native Zenith execute occurred");
}

#[test]
#[ignore = "an isolated real RSK during Zenith must emit native Wisdom Flurry children"]
fn literal_ww_native_zenith_rsk_emits_wisdom_flurry() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(WISDOM);
    let count = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(WISDOM)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points as usize;
    assert!(count > 0);
    let mut history = CastHistory::default();
    let mut sample: Option<(f64, Option<u64>)> = None;
    let mut strikes = 0;
    let mut areas = 0;
    for _ in 0..TICKS {
        let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
        run.app.update();
        drive_player(&mut run).unwrap();
        let now = run.app.world().resource::<Time>().elapsed_secs_f64();
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        for event in &events[before..] {
            if event.source != Some(run.actor.to_bits()) {
                continue;
            }
            let zenith_active = crate::auras::has_aura(run.app.world(), run.actor, ZENITH);
            let isolated_kick = history.observe_cast(event, now, zenith_active);
            if sample.is_none() && isolated_kick {
                sample = Some((now, event.target));
            }
            if let Some((_, primary)) = sample {
                if event.kind == CombatLogKind::Damage && event.amount > 0 {
                    match event.spell_id {
                        Some(STRIKE) => {
                            assert_eq!(event.target, primary, "Wisdom uses the real kick primary");
                            strikes += 1;
                        }
                        Some(AREA) => areas += 1,
                        _ => {}
                    }
                }
            }
        }
        if sample.is_some_and(|(t, _)| now - t > 0.6) {
            assert_eq!(
                (strikes, areas),
                (count, count),
                "isolated actual Zenith RSK must emit both native Wisdom children"
            );
            eprintln!("NATIVE_WISDOM strikes={strikes} areas={areas}");
            return;
        }
    }
    panic!("fixture prerequisite: no isolated native RSK during Zenith occurred");
}

#[derive(Default)]
struct CastHistory {
    last_fof: Option<f64>,
    last_zenith: Option<f64>,
    last_parent: Option<f64>,
}

impl CastHistory {
    fn observe_cast(&mut self, event: &CombatLogEvent, now: f64, zenith_active: bool) -> bool {
        if event.kind != CombatLogKind::CastSuccess {
            return false;
        }
        match event.spell_id {
            Some(113656) => self.last_fof = Some(now),
            Some(ZENITH) => self.last_zenith = Some(now),
            Some(107428 | 101546) => {
                // Exclude the full 30-charge FoF tail (29*150ms), initial
                // Stand Ready tail, and another Wisdom ICD respectively.
                let ordinary_tail_finished = self.last_fof.is_none_or(|t| now - t > 5.0);
                let initial_tail_finished = self.last_zenith.is_some_and(|t| now - t > 2.0);
                let icd_finished = self.last_parent.is_none_or(|t| now - t >= 1.0);
                self.last_parent = Some(now);
                let isolated = ordinary_tail_finished && initial_tail_finished && icd_finished;
                let active_rsk = event.spell_id == Some(107428) && zenith_active;
                return isolated && active_rsk;
            }
            _ => {}
        }
        false
    }
}
