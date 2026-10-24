//! Selected native193532/469880 returns, not tuned cast counts or cooldowns.
//! SimC76f8118 sc_hunter.cpp6741-6742/4963-4964; native70448 amounts remain authoritative.
use super::*;
use crate::auras::{ActiveAuras, RemoveReason};
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use server::simc_apl::Facts;
use shared::components::PowerType;
use shared::protocol::{CombatLogEvent, CombatLogKind};

const BARBED_SHOT: u32 = 217200;
const BESTIAL_WRATH: u32 = 19574;
const COBRA_SHOT: u32 = 193455;
const SCENT_OF_BLOOD: u32 = 193532;
const BARBED_SCALES: u32 = 469880;

fn barbed_fact(run: &Run, property: &str) -> f64 {
    let world = run.app.world();
    let bindings = Bindings {
        actions: BTreeMap::from([("barbed_shot".to_owned(), BARBED_SHOT)]),
        ..Default::default()
    };
    let encounter = Encounter::Fixture {
        started_at: world.resource::<Time>().elapsed_secs_f64(),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    NativeFacts::new(world, run.actor, &bindings, encounter, 1)
        .value(&format!("cooldown.barbed_shot.{property}"))
        .unwrap()
}

fn native_amount(run: &Run, spell: u32) -> f64 {
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell)
        .expect("native selected talent must be active");
    f64::from(
        aura.effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .amount,
    )
}

fn remove_native_talent(run: &mut Run, spell: u32) {
    let actor = run.actor;
    let removed = crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == spell,
    );
    assert_eq!(removed.len(), 1);
    assert!(!crate::auras::has_aura(run.app.world(), actor, spell));
}

fn wait_for_gcd(run: &mut Run, spell: u32) {
    for _ in 0..200 {
        let world = run.app.world();
        let info = world
            .resource::<crate::spell_info::SpellStore>()
            .get(spell)
            .unwrap();
        if !world
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .is_some_and(|history| history.on_gcd(info, crate::spell_cast::history_now(world)))
        {
            return;
        }
        run.app.update();
    }
    panic!("native GCD for{spell} never recovered");
}

fn cast_normal(run: &mut Run, spell: u32) {
    wait_for_gcd(run, spell);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, spell, Some(target), false)
        .unwrap_or_else(|error| panic!("native normal cast{spell}: {error:?}"));
}

fn barbed_casts(events: &[CombatLogEvent], actor: Entity) -> f64 {
    events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id == Some(BARBED_SHOT)
        })
        .count() as f64
}

fn wait_for_actual_kit_bw(run: &mut Run) {
    // engage clears the prior events; callers inspect this new complete event window.
    engage(run);
    for _ in 0..200 {
        run.app.update();
        if run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.kind == CombatLogKind::CastSuccess
                    && event.spell_id == Some(BESTIAL_WRATH)
            })
        {
            return;
        }
    }
    panic!("actual Austin kit did not admit Bestial Wrath");
}

fn assert_scent_return(run: &mut Run, expected_return: f64) {
    let full = barbed_fact(run, "charges");
    cast_normal(run, BARBED_SHOT);
    let spent = barbed_fact(run, "charges");
    assert_eq!(spent, full - 1.0, "real Barbed must spend its own charge");
    let recovery = barbed_fact(run, "full_recharge_time");
    let started = crate::spell_cast::history_now(run.app.world());
    wait_for_actual_kit_bw(run);
    let elapsed = crate::spell_cast::history_now(run.app.world()) - started;
    assert!(
        elapsed < recovery,
        "natural charge recovery must not explain the return"
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let bw = events
        .iter()
        .position(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id == Some(BESTIAL_WRATH)
        })
        .unwrap();
    let before_bw = barbed_casts(&events[..bw], run.actor);
    let after_bw = barbed_casts(&events[bw + 1..], run.actor);
    let expected = (spent - before_bw + expected_return).min(full) - after_bw;
    let after = barbed_fact(run, "charges");
    eprintln!(
        "NATIVE_SCENT full={full} spent={spent} returned={expected_return} before_bw_casts={before_bw} after_bw_casts={after_bw} after={after} elapsed={elapsed}"
    );
    assert_eq!(
        after, expected,
        "actual successful BW must apply only its active native charge return"
    );
}

fn assert_scales_reduction(run: &mut Run, seconds: f64, triggered: bool) {
    cast_normal(run, BARBED_SHOT);
    if !triggered {
        wait_for_gcd(run, COBRA_SHOT);
    }
    let before = barbed_fact(run, "full_recharge_time");
    assert!(
        before > 2.0,
        "native pending charge must outlive this sourced reduction"
    );
    let at = crate::spell_cast::history_now(run.app.world());
    let actor = run.actor;
    let target = run.targets[0];
    let focus =
        crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Focus).unwrap();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        actor,
        COBRA_SHOT,
        Some(target),
        triggered,
    )
    .unwrap();
    if triggered {
        assert_eq!(
            crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Focus)
                .unwrap(),
            focus,
            "real FULL_MASK background Cobra does not pay Focus"
        );
    }
    assert_eq!(
        crate::spell_cast::history_now(run.app.world()),
        at,
        "Cobra cast-time return must precede the next frame/missile hit"
    );
    let after = barbed_fact(run, "full_recharge_time");
    eprintln!("NATIVE_BARBED_SCALES before={before} after={after} reduction={seconds}");
    assert!(
        (after - (before - seconds).max(0.0)).abs() < 1e-6,
        "native Cobra cast must apply the active Barbed Scales reduction once"
    );
}

#[test]
#[ignore = "native owner Scent return; requires prepared253 gear/traits/current canonical"]
fn dps_parity_owner_cooldown_native_scent_actual_bestial_wrath_returns_barbed() {
    let (mut run, _) = super::power_tests::prepare_hunter_and_pet();
    let count = native_amount(&run, SCENT_OF_BLOOD);
    assert_eq!(
        count, 1.0,
        "native70448 selected Scent is one charge, not assumed old two"
    );
    assert_scent_return(&mut run, count);
}

#[test]
#[ignore = "native inactive Scent control; requires prepared253 gear/traits/current canonical"]
fn dps_parity_owner_cooldown_native_scent_removed_aura_does_not_return_barbed() {
    let (mut run, _) = super::power_tests::prepare_hunter_and_pet();
    native_amount(&run, SCENT_OF_BLOOD);
    remove_native_talent(&mut run, SCENT_OF_BLOOD);
    assert_scent_return(&mut run, 0.0);
}

#[test]
#[ignore = "native Cobra cast-time Scales return; requires prepared253 gear/traits/current canonical"]
fn dps_parity_owner_cooldown_native_scales_cobra_cast_reduces_pending_barbed() {
    let (mut run, _) = super::power_tests::prepare_hunter_and_pet();
    let milliseconds = native_amount(&run, BARBED_SCALES);
    assert_eq!(
        milliseconds, 2000.0,
        "actual selected native Scales milliseconds"
    );
    assert_scales_reduction(&mut run, milliseconds / 1000.0, false);
}

#[test]
#[ignore = "native inactive Scales control; requires prepared253 gear/traits/current canonical"]
fn dps_parity_owner_cooldown_native_scales_removed_aura_keeps_pending_barbed() {
    let (mut run, _) = super::power_tests::prepare_hunter_and_pet();
    native_amount(&run, BARBED_SCALES);
    remove_native_talent(&mut run, BARBED_SCALES);
    assert_scales_reduction(&mut run, 0.0, false);
}

#[test]
#[ignore = "native free Cobra Scales return; requires prepared253 gear/traits/current canonical"]
fn dps_parity_owner_cooldown_native_scales_free_cobra_reduces_pending_barbed() {
    let (mut run, _) = super::power_tests::prepare_hunter_and_pet();
    let milliseconds = native_amount(&run, BARBED_SCALES);
    assert_eq!(milliseconds, 2000.0);
    assert_scales_reduction(&mut run, milliseconds / 1000.0, true);
}

#[test]
#[ignore = "native inactive free Scales control; requires prepared253 gear/traits/current canonical"]
fn dps_parity_owner_cooldown_native_scales_removed_aura_keeps_free_cobra_pending_barbed() {
    let (mut run, _) = super::power_tests::prepare_hunter_and_pet();
    native_amount(&run, BARBED_SCALES);
    remove_native_talent(&mut run, BARBED_SCALES);
    assert_scales_reduction(&mut run, 0.0, true);
}
