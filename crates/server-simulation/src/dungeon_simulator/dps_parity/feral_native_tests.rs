//! Native Feral mechanics, independent of the literal Feral controller.
use super::*;

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_hunt_beneath_applies_cat_form_damage_modifiers() {
    use crate::auras::spell_mods::{SpellModOp, modify};
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    assert!(crate::auras::has_aura(run.app.world(), actor, 439868));
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1079)
        .unwrap()
        .clone();
    let values = |run: &Run| {
        [
            SpellModOp::HealingAndDamage,
            SpellModOp::PeriodicHealingAndDamage,
        ]
        .map(|op| modify(run.app.world(), actor, &spell, op, 100.0))
    };
    let before = values(&run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let in_form = values(&run);
    for i in 0..2 {
        assert!(
            (in_form[i] / before[i] - 1.03).abs() < 0.0001,
            "cat damage modifier {i}: {before:?} -> {in_form:?}"
        );
    }
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 768,
    );
    let after = values(&run);
    for i in 0..2 {
        assert!(
            (after[i] - before[i]).abs() < 0.001,
            "cat modifier must end on leaving form"
        );
    }
}

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_lycaras_teachings_follows_cat_form() {
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    let crit = |run: &Run| {
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .crit_pct
    };
    let before = crit(&run);
    assert!(crate::auras::has_aura(run.app.world(), actor, 378988));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), actor, 378990));
    assert!(
        (crit(&run) - before - 6.0).abs() < 0.01,
        "rank two Cat Form grants six crit points"
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 768,
    );
    assert!(!crate::auras::has_aura(run.app.world(), actor, 378990));
    run.app.update();
    assert!(
        (crit(&run) - before).abs() < 0.01,
        "leaving Cat Form removes the crit gain: before={before} after={}",
        crit(&run)
    );
}

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_tigers_tenacity_only_rewards_foreground_finishers() {
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(103, 3).unwrap();
    let actor = run.actor;
    let charges = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == 391873 && aura.caster == actor)
            })
            .map_or(0, |aura| aura.charges)
    };
    let grants = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(391874)
                    && event.kind == CombatLogKind::Energize
            })
            .count()
    };
    assert_eq!(
        charges(&run),
        0,
        "precombat actions must not grant Tiger's Tenacity"
    );
    assert_eq!(
        grants(&run),
        0,
        "precombat must not generate its combo point"
    );
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(5217, actor, None),
    )
    .unwrap();
    run.app.update();
    assert_eq!(charges(&run), 3, "Tiger's Fury earns three native charges");
    assert_eq!(grants(&run), 0, "applying the buff is not a finisher");
    feral_growth_tests::cast_native_spell_after_regeneration(&mut run, 1822);
    assert_eq!(charges(&run), 3, "Rake must not consume a finisher charge");
    assert_eq!(grants(&run), 0, "Rake must not trigger the finisher reward");
    feral_growth_tests::cast_native_spell_after_regeneration(&mut run, 1079);
    run.app.update();
    assert_eq!(
        charges(&run),
        2,
        "one actual Rip consumes exactly one charge"
    );
    assert_eq!(grants(&run), 1, "one actual Rip grants exactly one reward");
}

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_exact_magisters_token_applies_native_mastery_flask() {
    let run = prepare_literal(103, 1).expect("exact selected Feral profile");
    let stats = report::stats(run.app.world(), run.actor);
    let mastery = stats["ratings"]["mastery"].as_f64().unwrap();
    assert!(
        (mastery - 1424.0).abs() < 1.0,
        "native gear1259 plus quality2 Magisters165 must produce the exact-profile mastery rating, got{mastery}"
    );
    assert!(
        !run.gaps.iter().any(|gap| gap.contains("flask=magisters_2")),
        "the applied native flask must not remain reported as unsupported"
    );
}

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_prev_gcd_tracks_successful_foreground_gcd_casts_only() {
    use server::simc_apl::Facts;
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    let program = server::simc_apl::Program::parse(&[(
        "default",
        "shred,if=prev_gcd.1.shred|prev_gcd.2.shred",
    )])
    .unwrap();
    server::simc_apl::gcd_history::initialize(run.app.world_mut(), actor, &program);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("shred".into(), 5221), ("rake".into(), 1822)].into(),
        ..Default::default()
    };
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let read = |run: &Run, name: &str| {
        crate::simc_apl_native::NativeFacts::new(run.app.world(), actor, &bindings, encounter, 1)
            .with_targets(Some(target), &run.targets)
            .value(name)
            .unwrap()
    };
    assert_eq!(read(&run, "prev_gcd.1.shred"), 0.0);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5221, Some(target), false)
        .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(read(&run, "prev_gcd.1.shred"), 1.0);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5217, Some(actor), false).unwrap();
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1822, Some(target), 1);
    assert!(
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5221, Some(actor), false)
            .is_err()
    );
    assert_eq!(
        read(&run, "prev_gcd.1.shred"),
        1.0,
        "off-GCD, triggered and failed requests preserve history"
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1822, Some(target), false)
        .unwrap();
    assert_eq!(read(&run, "prev_gcd.1.rake"), 1.0);
    assert_eq!(read(&run, "prev_gcd.2.shred"), 1.0);
}

#[test]
#[ignore = "requires spec103 profile and native world data"]
fn tigers_fury_grants_savage_fury_energy_regeneration() {
    use shared::components::PowerType;
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), actor, 449645));
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Energy, -140.0);
    for _ in 0..20 {
        run.app.update();
    }
    let baseline =
        crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Energy)
            .unwrap()
            .0;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5217, Some(actor), false).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), actor, 449646));
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Energy, -140.0);
    for _ in 0..20 {
        run.app.update();
    }
    let empowered =
        crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Energy)
            .unwrap()
            .0;
    assert!(
        empowered > baseline * 1.25,
        "Savage Fury regen {empowered} vs {baseline}"
    );
}

#[test]
#[ignore = "requires spec103 profile and native world data"]
fn berserk_stores_overflow_until_real_finisher() {
    use shared::components::PowerType;
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 106951, Some(actor), 1);
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        PowerType::ComboPoints,
        8.0,
    );
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 405189)
        .expect("overflow stored");
    assert_eq!(aura.stacks, 3);
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Energy, 140.0);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 22568, Some(target), false)
        .unwrap();
    assert!(!crate::auras::has_aura(run.app.world(), actor, 405189));
    assert_eq!(
        crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::ComboPoints)
            .unwrap()
            .0,
        3.0
    );
}

#[test]
#[ignore = "requires spec103 profile and native world data"]
fn panthers_guile_fills_combo_points_after_real_shred() {
    use shared::components::PowerType;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), actor, 1280316));
    let mut filled = 0;
    for _ in 0..40 {
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            PowerType::ComboPoints,
            -5.0,
        );
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 5221, Some(target), 1);
        let combo =
            crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::ComboPoints)
                .unwrap()
                .0;
        if combo == 5.0 {
            filled += 1;
        }
        for _ in 0..4 {
            run.app.update();
        }
    }
    assert!(
        filled > 0,
        "selected Panther's Guile must fill combo points on Shred"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec103 exact profile/traits"]
fn feral_frenzy_rolls_five_hits_into_one_bleed() {
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(103, 2).expect("selected Feral profile");
    let actor = run.actor;
    let single = run.targets[0];
    let rolling = run.targets[1];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for target in [single, rolling] {
        crate::spell_cast::cast_triggered_with_overrides(
            run.app.world_mut(),
            actor,
            274838,
            Some(target),
            1,
            crate::spell_cast::TriggeredSpellOverrides {
                effect_mask: Some(0b1011),
                ..Default::default()
            },
        );
    }
    for _ in 0..4 {
        for _ in 0..4 {
            run.app.update();
        }
        crate::spell_cast::cast_triggered_with_overrides(
            run.app.world_mut(),
            actor,
            274838,
            Some(rolling),
            1,
            crate::spell_cast::TriggeredSpellOverrides {
                effect_mask: Some(0b1011),
                ..Default::default()
            },
        );
    }
    for _ in 0..120 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let first_tick = |target: bevy::prelude::Entity| {
        let e = events
            .iter()
            .find(|e| {
                e.target == Some(target.to_bits())
                    && e.source == Some(actor.to_bits())
                    && e.spell_id == Some(274838)
                    && e.kind == CombatLogKind::Damage
                    && e.periodic
            })
            .unwrap();
        e.amount as f32 / if e.crit { 2.0 } else { 1.0 }
    };
    let ratio = first_tick(rolling) / first_tick(single);
    assert!(
        ratio > 3.0 && ratio < 5.5,
        "five child hits must accumulate their bleed budget, first tick ratio {ratio}"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec103 exact profile/traits"]
fn feral_frenzy_selected_profile_executes_native_children_and_earns_combo_points() {
    let mut run = prepare_literal(103, 1).expect("selected Feral profile");
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false)
        .expect("ordinary native Cat Form");
    preparation::finish_pending_setup(&mut run).unwrap();
    let action = crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), actor, 274837)
        .expect("selected Frenzy action");
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, action, Some(target), false)
        .expect("ordinary Frenzy cast");
    for _ in 0..100 {
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.targets, 5.0);
    let child = if action == 1243807 { 1244079 } else { 274838 };
    let damage: u64 = combat["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["spell_id"] == child)
        .map(|row| row["damage"].as_u64().unwrap())
        .sum();
    assert!(
        damage > 0,
        "native Frenzy must execute its damaging child, not only the dummy parent"
    );
    let combo = crate::spell_cast::powers::power_value(
        run.app.world(),
        actor,
        shared::components::PowerType::ComboPoints,
    )
    .unwrap()
    .0;
    assert_eq!(combo, 5.0, "Frenzy earns the native five combo points");
}
