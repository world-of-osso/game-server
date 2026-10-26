//! Selected native WDP/SCK earned-buff relationships; no injected buffs or resources.
use super::super::*;
use crate::auras::ActiveAuras;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const SEQUENCED: u32 = 451515;
const REVOLVING: u32 = 451524;
const DANCE: u32 = 325202;
const COMBO_BREAKER: u32 = 116768;
const WDP: u32 = 152175;
const SCK: u32 = 101546;

fn prepare(talent: u32) -> Run {
    let mut run = prepare_measurement(269, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|spell| spell.spell_id == talent)
    );
    let native_chance = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(talent)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    assert_eq!(
        native_chance, 100.0,
        "this native selection guarantees the earned buff"
    );
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    run
}

fn stacks(run: &Run, spell: u32) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

fn cap(run: &Run, spell: u32) -> u8 {
    run.app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .unwrap()
        .aura_options
        .as_ref()
        .unwrap()
        .cumulative_aura as u8
}

fn cast_since(run: &Run, spell: u32, before: usize) -> bool {
    run.app.world().resource::<ObservedCombatEvents>().0[before..]
        .iter()
        .any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::CastSuccess
        })
}

#[test]
#[ignore = "actual selected WDP must grant native Dance independently of its ordinary RPPM"]
fn literal_ww_native_revolving_whirl_wdp_grants_dance() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(REVOLVING);
    let maximum = cap(&run, DANCE);
    let mut observed = 0;
    for _ in 0..TICKS {
        let before = stacks(&run, DANCE);
        let events = run.app.world().resource::<ObservedCombatEvents>().0.len();
        run.app.update();
        drive_player(&mut run).unwrap();
        if !cast_since(&run, WDP, events) || before >= maximum {
            continue;
        }
        let after = stacks(&run, DANCE);
        assert!(
            after > before,
            "selected actual WDP must earn native Dance: before={before}, after={after}"
        );
        observed += 1;
    }
    assert!(
        observed > 0,
        "fixture prerequisite: actual WDP with Dance capacity"
    );
    eprintln!("NATIVE_REVOLVING native_wdp={observed}");
}

#[test]
#[ignore = "actual empowered SCK must earn native Combo Breaker from selected Sequenced Strikes"]
fn literal_ww_native_sequenced_empowered_sck_grants_combo_breaker() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(SEQUENCED);
    let maximum = cap(&run, COMBO_BREAKER);
    let mut observed = 0;
    for _ in 0..TICKS {
        let dance_before = stacks(&run, DANCE);
        let before = stacks(&run, COMBO_BREAKER);
        let events = run.app.world().resource::<ObservedCombatEvents>().0.len();
        run.app.update();
        drive_player(&mut run).unwrap();
        let empowered = dance_before > stacks(&run, DANCE);
        let admitted_empowered = empowered && cast_since(&run, SCK, events);
        if !admitted_empowered || before >= maximum {
            continue;
        }
        let after = stacks(&run, COMBO_BREAKER);
        assert!(
            after > before,
            "actual SCK consuming earned Dance must grant native Combo Breaker: before={before}, after={after}"
        );
        observed += 1;
    }
    assert!(
        observed > 0,
        "fixture prerequisite: actual empowered SCK with Combo Breaker capacity"
    );
    eprintln!("NATIVE_SEQUENCED empowered_sck={observed}");
}
