//! SIMC_RET_EMPYREAN_LEGACY_RELATIONSHIP: sourced dispatch, native amounts/targets.
use super::ret_purpose_tests::{current_holy_power, earn_finisher_power};
use super::ret_tier_tests::{has, native_ret};
use super::*;
use crate::spell_cast::{CastFailure, test_harness::ObservedCombatEvents};
use shared::protocol::CombatLogKind;

const AVENGING_WRATH: u32 = 31884;
const TEMPLARS_VERDICT: u32 = 85256;
const LEGACY_TALENT: u32 = 387170;
const LEGACY_BUFF: u32 = 387178;
const DIVINE_STORM: u32 = 53385;
const PURPOSE: u32 = 223819;
const VERDICT_ARBITER: u32 = 1306161;
const DIVINE_ARBITER: u32 = 1306923;

fn fixture(selected: bool) -> Run {
    let mut run = native_ret(4);
    if !selected {
        inputs::install_traits(run.app.world_mut(), run.actor, &[]).unwrap();
        for _ in 0..4 {
            run.app.update();
        }
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    run
}

fn resolved_verdict(run: &Run) -> u32 {
    crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, TEMPLARS_VERDICT)
        .unwrap()
}

fn cast_ready(run: &mut Run, spell: u32, target: Entity) {
    for _ in 0..200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => return,
            Err(CastFailure::OnGlobalCooldown | CastFailure::InProgress) => run.app.update(),
            Err(reason) => panic!("native Legacy action{spell} refused: {reason:?}"),
        }
    }
    panic!("native Legacy action{spell} never reached readiness");
}

fn applications(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::AuraApplied
        })
        .count()
}

fn damage(run: &Run, spell: u32) -> i64 {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(run.targets[0].to_bits())
        })
        .map(|event| i64::from(event.amount))
        .sum()
}

fn wait_for_damage(run: &mut Run, spell: u32) {
    for _ in 0..200 {
        if damage(run, spell) > 0 {
            return;
        }
        run.app.update();
    }
    panic!("actual native background{spell} must damage the dummy");
}

#[test]
#[ignore = "selected Legacy must not use the metadata-only every-cast grant"]
fn dps_parity_ret_empyrean_without_wrath_generators_do_not_grant_legacy() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture(true);
    let verdict = resolved_verdict(&run);
    earn_finisher_power(&mut run, verdict);
    assert_eq!(
        applications(&run, LEGACY_BUFF),
        0,
        "ordinary Judgment funding is not Avenging Wrath"
    );
}

#[test]
#[ignore = "native AW then selected TV alias must emit free background Storm at native percentage"]
fn dps_parity_ret_empyrean_wrath_verdict_emits_native_percent_background_storm() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture(true);
    let verdict = resolved_verdict(&run);
    earn_finisher_power(&mut run, verdict);
    let actor = run.actor;
    cast_ready(&mut run, AVENGING_WRATH, actor);
    for _ in 0..4 {
        run.app.update();
    }
    assert!(
        has(&run, LEGACY_BUFF),
        "actual AW grants native Legacy387178"
    );
    let percent =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, LEGACY_TALENT, 1);
    run.app.insert_resource(
        crate::spell_cast::effects::factor_capture::FactorCapture::new(
            actor,
            run.targets[0],
            DIVINE_STORM,
            "empyrean_legacy",
            8,
        ),
    );
    let storm = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(DIVINE_STORM)
        .unwrap();
    let storm_cost: f32 = crate::spell_cast::applicable_costs(run.app.world(), actor, storm)
        .unwrap()
        .into_iter()
        .filter(|(pool, _, _)| {
            *pool == crate::spell_cast::CostPool::Power(shared::components::PowerType::HolyPower)
        })
        .map(|(_, amount, _)| amount)
        .sum();
    let target = run.targets[0];
    cast_ready(&mut run, verdict, target);
    assert!(
        !has(&run, LEGACY_BUFF),
        "foreground Verdict consumes Legacy before its callback"
    );
    assert!(
        current_holy_power(&run) < storm_cost,
        "background is not funded as a second paid spender"
    );
    wait_for_damage(&mut run, DIVINE_STORM);
    let factors = run
        .app
        .world()
        .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
        .export();
    let result = factors["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| {
            event["event"] == "result"
                && event["spell_id"] == DIVINE_STORM
                && event["native_depth"]
                    .as_u64()
                    .is_some_and(|depth| depth > 0)
        })
        .expect("actual background damage has a native calculation receipt");
    let observed = &result["factors"];
    let measured = observed["total_amount"].as_f64().unwrap()
        / (observed["native_source_amount"].as_f64().unwrap()
            * observed["target_product"].as_f64().unwrap());
    let expected = 1.0 + f64::from(percent) / 100.0;
    assert!(
        (measured - expected).abs() <= 4.0 * f64::from(f32::EPSILON),
        "real damage stage multiplier{measured} must equal native Legacy percentage{expected}"
    );
}

#[test]
#[ignore = "Legacy background consumes opposite Arbiter, but not real Divine Purpose or its charge"]
fn dps_parity_ret_empyrean_background_preserves_purpose_and_consumes_opposite_arbiter() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture(true);
    let verdict = resolved_verdict(&run);
    earn_finisher_power(&mut run, verdict);
    let actor = run.actor;
    cast_ready(&mut run, AVENGING_WRATH, actor);
    for _ in 0..4 {
        run.app.update();
    }
    assert!(has(&run, LEGACY_BUFF));
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, VERDICT_ARBITER, Some(actor), 0);
    let target = run.targets[0];
    cast_ready(&mut run, verdict, target);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, PURPOSE, Some(actor), 0);
    let purpose_state = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == PURPOSE)
            .map(|aura| (aura.instance_id, aura.charges))
    };
    let before = purpose_state(&run).unwrap();
    wait_for_damage(&mut run, DIVINE_STORM);
    wait_for_damage(&mut run, DIVINE_ARBITER);
    assert_eq!(
        purpose_state(&run),
        Some(before),
        "background keeps the actual Purpose instance/charge"
    );
    assert!(!has(&run, VERDICT_ARBITER));
}

#[test]
#[ignore = "AW cannot grant Legacy or a background spender without selected387170"]
fn dps_parity_ret_empyrean_unselected_owner_cannot_receive_the_policy_grant() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture(false);
    assert!(
        !run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(LEGACY_TALENT)
    );
    let verdict = resolved_verdict(&run);
    earn_finisher_power(&mut run, verdict);
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        AVENGING_WRATH,
        Some(run.actor),
        0,
    );
    let target = run.targets[0];
    cast_ready(&mut run, verdict, target);
    for _ in 0..10 {
        run.app.update();
    }
    assert_eq!(applications(&run, LEGACY_BUFF), 0);
    assert_eq!(damage(&run, DIVINE_STORM), 0);
}
