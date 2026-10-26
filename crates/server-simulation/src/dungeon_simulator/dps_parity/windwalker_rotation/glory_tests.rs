//! Selected native Glory uses real kick executes, haste, damage and Chi.
use super::super::*;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;
use std::collections::HashSet;

const TALENT: u32 = 392958;
const DAMAGE: u32 = 392959;
const RSK_DAMAGE: u32 = 185099;
const RWK_DAMAGE: u32 = 468179;

#[test]
#[ignore = "real selected Glory must proc from RSK and RWK and grant native Chi"]
fn literal_ww_native_glory_kick_executes_emit_damage_and_chi() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(269, 3).unwrap();
    let actor = run.actor;
    let primary = run.targets[0];
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    assert!(
        run.app
            .world()
            .get::<TraitLoadout>(actor)
            .unwrap()
            .active_talent_spells()
            .any(|spell| spell.spell_id == TALENT)
    );
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(DAMAGE)
        .unwrap();
    let energize = spell
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .unwrap();
    assert_eq!(
        energize.misc_values[0],
        shared::components::PowerType::Chi as i32
    );
    let expected_chi = energize.base_points as i32;
    assert!(expected_chi > 0);
    engage(&mut run);
    let mut last_kick = None;
    let mut parents = HashSet::new();
    let mut chi_earned = false;
    for _ in 0..TICKS {
        let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
        run.app.update();
        drive_player(&mut run).unwrap();
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        for event in &events[before..] {
            if event.source != Some(actor.to_bits()) {
                continue;
            }
            if matches!(event.spell_id, Some(RSK_DAMAGE | RWK_DAMAGE))
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
            {
                last_kick = event.spell_id;
            }
            if event.spell_id != Some(DAMAGE) {
                continue;
            }
            if event.kind == CombatLogKind::Damage && event.amount > 0 {
                assert_eq!(
                    event.target,
                    Some(primary.to_bits()),
                    "Glory retains the actual primary"
                );
                parents.insert(last_kick.expect("actual RSK/RWK damage precedes its Glory child"));
            }
            if event.kind == CombatLogKind::Energize {
                assert_eq!(
                    event.amount, expected_chi,
                    "native child owns the Chi amount"
                );
                chi_earned |= event.amount > event.overflow;
            }
        }
        if parents.contains(&RSK_DAMAGE) && parents.contains(&RWK_DAMAGE) && chi_earned {
            eprintln!("NATIVE_GLORY parents={parents:?} chi={expected_chi}");
            return;
        }
    }
    panic!(
        "selected native kicks did not produce Glory damage and actual Chi: parents={parents:?}, chi={chi_earned}"
    );
}
