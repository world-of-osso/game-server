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
const PURPOSE: u32 = 408458;
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
#[ignore = "native Legacy Storms must exercise the selected Divine Purpose chance"]
fn dps_parity_ret_empyrean_background_storm_can_generate_divine_purpose() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture(true);
    let verdict = resolved_verdict(&run);
    let mut background_grants = 0;
    for _ in 0..32 {
        earn_finisher_power(&mut run, verdict);
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            AVENGING_WRATH,
            Some(run.actor),
            1,
        );
        let target = run.targets[0];
        cast_ready(&mut run, verdict, target);
        // The foreground roll is finished, but Legacy is still queued. Only
        // the real background consumer may grant Purpose during these ticks.
        let before = applications(&run, PURPOSE);
        for _ in 0..30 {
            run.app.update();
        }
        background_grants += applications(&run, PURPOSE) - before;
    }
    assert!(
        background_grants > 0,
        "32 real Legacy Storms must roll native Divine Purpose after foreground completion; got{background_grants} grants"
    );
}

#[test]
#[ignore = "unrelated native Judgment must not spend Second Sunrise's ICD"]
fn dps_parity_ret_sunrise_unrelated_judgment_preserves_icd() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture(true);
    let sunrise_timer = |run: &Run| {
        let auras = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap();
        let aura = auras.iter().find(|aura| aura.spell_id == 431474).unwrap();
        (aura.proc.last_success_at, aura.proc.next_ready_at)
    };
    let before = sunrise_timer(&run);
    let target = run.targets[0];
    cast_ready(&mut run, 20271, target);
    for _ in 0..30 {
        run.app.update();
    }
    assert!(damage(&run, 20271) > 0, "actual Judgment must land");
    // SimC76f8118 Ret612–619/sc_paladin.cpp1774–1781: only Storm
    // and Hammer producers roll this ICD, never Judgment or white swings.
    assert_eq!(sunrise_timer(&run), before);
}

#[test]
#[ignore = "native Legacy Storms must exercise the selected Second Sunrise chance"]
fn dps_parity_ret_empyrean_background_storm_can_echo_second_sunrise() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture(true);
    let verdict = resolved_verdict(&run);
    assert!(has(&run, 431474), "selected native Second Sunrise passive");
    const LEGACY_CASTS: usize = 32;
    for _ in 0..LEGACY_CASTS {
        earn_finisher_power(&mut run, verdict);
        // Exercise the already-qualified Wrath→Legacy relationship without
        // changing proc chance or injecting any Storm/Second Sunrise outcome.
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            AVENGING_WRATH,
            Some(run.actor),
            1,
        );
        let target = run.targets[0];
        cast_ready(&mut run, verdict, target);
        for _ in 0..30 {
            run.app.update();
        }
    }
    let storms = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(DIVINE_STORM)
                && event.kind == CombatLogKind::Damage
        })
        .count();
    assert!(
        storms > LEGACY_CASTS,
        "{LEGACY_CASTS} actual Legacy Storms must earn Second Sunrise repeats; got{storms} hits"
    );
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
    assert_eq!(
        damage(&run, DIVINE_STORM),
        0,
        "Legacy callback is scheduled, not inline"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, PURPOSE, Some(actor), 0);
    assert_eq!(
        damage(&run, DIVINE_STORM),
        0,
        "Purpose is present before background execution"
    );
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
#[ignore = "SimC background Wrath454351 retains the same selected Legacy relationship"]
fn dps_parity_ret_empyrean_background_wrath_grants_only_to_selected_owner() {
    let _seed = crate::random::seed_scope(70448);
    for selected in [true, false] {
        let mut run = fixture(selected);
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            454351,
            Some(run.actor),
            0,
        );
        for _ in 0..4 {
            run.app.update();
        }
        assert!(
            has(&run, 454351),
            "actual background Wrath applies its native aura"
        );
        assert_eq!(has(&run, LEGACY_BUFF), selected);
    }
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
