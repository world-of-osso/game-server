//! Real native Ret casts prove the missing damage-child boundaries.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn native_ret(targets: usize) -> Run {
    let (profile, traits) = read_reference(70).unwrap();
    let mut run = prepare_literal_reference(
        profile,
        &traits,
        targets,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    engage(&mut run);
    run
}

fn damage(run: &Run, spell: u32) -> i64 {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|e| {
            e.source == Some(run.actor.to_bits())
                && e.spell_id == Some(spell)
                && e.kind == CombatLogKind::Damage
        })
        .map(|e| i64::from(e.amount))
        .sum()
}
fn ticks(run: &mut Run, count: usize) {
    for _ in 0..count {
        run.app.update();
    }
}
fn triggered(run: &mut Run, spell: u32) {
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        spell,
        Some(run.targets[0]),
        1,
    );
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_execution_sentence_paid_cast_deals_native_initial_hit() {
    let mut run = native_ret(1);
    ret_purpose_tests::earn_finisher_power(&mut run, 343527);
    ret_tier_tests::cast_finisher(&mut run, 343527);
    ticks(&mut run, 30);
    assert!(
        damage(&run, 1260251) > 0,
        "paid Sentence must execute its native initial damage child"
    );
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_judgment_produces_mastery_highlords_judgment() {
    let mut run = native_ret(1);
    for _ in 0..80 {
        triggered(&mut run, 20271);
        ticks(&mut run, 2);
    }
    assert!(
        damage(&run, 383921) > 0,
        "real Judgment impacts must roll Highlord's Judgment with native mastery"
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.actor.to_bits())
                && event.spell_id == Some(20271)
                && matches!(
                    event.kind,
                    CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                )),
        "native Judgment must also resolve its real caster-aura recipient"
    );
    let self_hits = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.actor.to_bits())
                && event.spell_id == Some(383921)
                && event.kind == CombatLogKind::Damage
        })
        .count();
    assert_eq!(
        self_hits, 0,
        "caster-aura recipients are not enemy damage impacts"
    );
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_art_of_war_blade_produces_delayed_light_within() {
    let mut run = native_ret(1);
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 406086, Some(run.actor), 1);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 406086));
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        184575,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    assert_eq!(
        damage(&run, 1261160),
        0,
        "Light Within has an authored350ms delay"
    );
    ticks(&mut run, 30);
    assert!(
        damage(&run, 1261160) > 0,
        "Art of War Blade must produce native Light Within damage"
    );
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_paid_storm_produces_tempest_damage() {
    let mut run = native_ret(1);
    ret_purpose_tests::earn_finisher_power(&mut run, 53385);
    ret_tier_tests::cast_finisher(&mut run, 53385);
    ticks(&mut run, 30);
    assert!(
        damage(&run, 224239) > 0,
        "native selected Tempest must deal its separate child damage"
    );
}

#[test]
#[ignore = "native70448 Tempest must hit the complete ordinary Storm pack"]
fn ret_damage_tempest_hits_every_storm_recipient() {
    let mut run = native_ret(3);
    ret_purpose_tests::earn_finisher_power(&mut run, 53385);
    ret_tier_tests::cast_finisher(&mut run, 53385);
    ticks(&mut run, 30);
    let recipients = |spell| {
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .filter_map(|event| event.target)
            .collect::<std::collections::BTreeSet<_>>()
    };
    let storm = recipients(53385);
    assert_eq!(
        storm.len(),
        3,
        "ordinary native Storm hits the actual three-target pack"
    );
    assert_eq!(
        recipients(224239),
        storm,
        "Tempest must hit the same native area, not only the explicit primary"
    );
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_critical_storm_applies_sun_sear() {
    let mut run = native_ret(1);
    for _ in 0..20 {
        triggered(&mut run, 53385);
        ticks(&mut run, 30);
    }
    assert!(
        damage(&run, 431414) > 0,
        "selected Sun Sear must tick after a critical Storm"
    );
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_dawnlight_tick_produces_suns_avatar() {
    let mut run = native_ret(1);
    triggered(&mut run, 431380);
    ticks(&mut run, 80);
    assert!(damage(&run, 431380) > 0, "native Dawnlight must tick first");
    assert!(
        damage(&run, 431911) > 0,
        "selected Sun's Avatar must damage on each Dawnlight tick"
    );
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_blade_produces_consecration_ticks() {
    let mut run = native_ret(1);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        184575,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    ticks(&mut run, 100);
    assert!(
        damage(&run, 81297) > 0,
        "Consecrated Blade must spawn real native Consecration ticks"
    );
}

fn source_product(run: &mut Run, spell: u32) -> f64 {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    run.app.world_mut().insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        spell,
        "ret_damage",
        1,
    ));
    for _ in 0..20 {
        triggered(run, spell);
        ticks(run, 20);
        let capture = run.app.world().resource::<FactorCapture>().export();
        if let Some(result) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["event"] == "result" && e["spell_id"] == spell)
        {
            return result["factors"]["native_source_amount"].as_f64().unwrap()
                / result["factors"]["native_input_amount"].as_f64().unwrap();
        }
    }
    panic!("native spell{spell} never hit");
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_highlords_wrath_enhances_hammer_mastery_without_double_applying_base() {
    let mut run = native_ret(1);
    let mastery = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .mastery_pct as f64
        / 100.0;
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 404512));
    let enhanced = source_product(&mut run, 1279408);
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 404512,
    );
    let ordinary = source_product(&mut run, 1279408);
    let expected = (1.0 + mastery * 1.5) / (1.0 + mastery);
    assert!(
        (enhanced / ordinary - expected).abs() < 0.01,
        "native Highlord's Wrath must enhance only the whitelisted mastery: {enhanced}/{ordinary}, expected{expected}"
    );
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_truths_wake_ticks_before_unhasted_three_second_deadline() {
    let mut run = native_ret(1);
    triggered(&mut run, 255937);
    ticks(&mut run, 55);
    assert!(
        damage(&run, 403695) > 0,
        "native Truth's Wake must use hasted ticks before3s"
    );
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_divine_arbiter_has_one_primary_hit_not_primary_plus_cleave() {
    let mut run = native_ret(1);
    triggered(&mut run, 1306923);
    ticks(&mut run, 40);
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|e| {
            e.source == Some(run.actor.to_bits())
                && e.spell_id == Some(1306923)
                && e.target == Some(run.targets[0].to_bits())
                && e.kind == CombatLogKind::Damage
        })
        .count();
    assert_eq!(
        hits, 1,
        "SimC Ret301-310: primary uses effect1AP, other enemies use effect2AP, never both on primary"
    );
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_sentence_gathers_marked_secondary_target_into_primary_explosion() {
    let mut run = native_ret(3);
    ret_purpose_tests::earn_finisher_power(&mut run, 343527);
    ret_tier_tests::cast_finisher(&mut run, 343527);
    ticks(&mut run, 10);
    let counter = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.targets[0])
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 343527 && a.caster == run.actor)
            .unwrap()
            .effects
            .iter()
            .find(|e| e.index == 0)
            .unwrap()
            .amount
    };
    let before = counter(&run);
    assert!(
        crate::auras::has_aura(run.app.world(), run.targets[1], 1260251),
        "native init must mark secondary recipients"
    );
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        383921,
        Some(run.targets[1]),
        1,
    );
    ticks(&mut run, 10);
    assert!(
        counter(&run) > before,
        "nonphysical damage to marked secondary must fund the primary Sentence"
    );
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_native_verdict_applies_mastery_exactly_once() {
    let mut run = native_ret(1);
    let mastery = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .mastery_pct as f64
        / 100.0;
    let with_mastery = source_product(&mut run, 383328);
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 267316,
    );
    let without_mastery = source_product(&mut run, 383328);
    assert!(
        (with_mastery / without_mastery - (1.0 + mastery)).abs() < 0.001,
        "FV native mastery must apply once: {with_mastery}/{without_mastery}, mastery{mastery}"
    );
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_burn_to_ash_critical_tick_extends_owned_truths_wake() {
    let mut run = native_ret(1);
    let expiry = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.targets[0])
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 403695 && aura.caster == run.actor)
            .unwrap()
            .expires_at
            .unwrap()
    };
    for _ in 0..8 {
        triggered(&mut run, 403695);
        let initial = expiry(&run);
        let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
        for _ in 0..100 {
            run.app.update();
            let critical_tick = run.app.world().resource::<ObservedCombatEvents>().0[before..]
                .iter()
                .any(|event| {
                    event.spell_id == Some(403695)
                        && event.periodic
                        && event.crit
                        && event.kind == CombatLogKind::Damage
                });
            if critical_tick && crate::spell_cast::now(run.app.world()) < initial {
                assert!(
                    expiry(&run) >= initial + 2.0,
                    "Burn to Ash's native2s extension must preserve a ticking owned aura"
                );
                return;
            }
        }
    }
    panic!("no actual native periodic crit exercised Burn to Ash");
}

#[test]
#[ignore = "opt-in native parity inputs"]
fn ret_damage_sentence_explosion_applies_normal_outgoing_modifiers_to_stored_damage() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let mut run = native_ret(1);
    ret_purpose_tests::earn_finisher_power(&mut run, 343527);
    ret_tier_tests::cast_finisher(&mut run, 343527);
    ticks(&mut run, 30);
    run.app.world_mut().insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        387113,
        "execution_sentence",
        1,
    ));
    for _ in 0..240 {
        run.app.update();
        let capture = run.app.world().resource::<FactorCapture>().export();
        if let Some(result) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["event"] == "result" && e["spell_id"] == 387113)
        {
            assert_eq!(
                result["factors"]["result"], "hit",
                "Sentence's native no-crit flag must survive"
            );
            assert!(
                result["factors"]["native_done_product"].as_f64().unwrap() > 1.01,
                "SimC Ret61–75 multiplies stored damage normally, including native versatility: {result}"
            );
            return;
        }
    }
    panic!("native funded Sentence never discharged stored damage");
}
