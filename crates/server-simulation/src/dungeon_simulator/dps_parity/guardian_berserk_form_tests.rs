//! Incarnation must enter Bear Form from the real Cat Form opener.
use super::*;
use crate::auras::ActiveAuras;

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn guardian_star_cascade_comes_from_foreground_landed_thrash() {
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 393427,
    );
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    for _ in 0..15 {
        crate::spell_cast::scripts::reduce_cooldown(run.app.world_mut(), actor, 77758, f64::MAX);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
    }
    let count = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(197626)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count()
    };
    let foreground = count(&run);
    assert!(
        foreground > 0 && foreground <= 15,
        "15 landed foreground Thrashes earned {foreground} cascades"
    );
    for _ in 0..15 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 77758, Some(target), 1);
    }
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(
        count(&run),
        foreground,
        "background Thrash cannot trigger Star Cascade"
    );
}

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn guardian_heart_in_cat_executes_empowered_feral_frenzy() {
    use shared::protocol::CombatLogKind;
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 274838, Some(target), 1);
    let control = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(274838)
                && event.kind == CombatLogKind::Damage
                && !event.periodic
        })
        .expect("unempowered Frenzy hit");
    let control_damage = control.amount as f32 / if control.crit { 2.0 } else { 1.0 };
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        shared::components::PowerType::ComboPoints,
        -5.0,
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 274838 && aura.caster == actor,
    );
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1261868, Some(target), false)
        .unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let impacts: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(274838)
                && event.kind == CombatLogKind::Damage
                && !event.periodic
        })
        .collect();
    let ticks = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(274838)
                && event.kind == CombatLogKind::Damage
                && event.periodic
        })
        .count();
    let normalized = impacts
        .iter()
        .map(|event| event.amount as f32 / if event.crit { 2.0 } else { 1.0 })
        .sum::<f32>()
        / impacts.len() as f32;
    assert!(
        normalized > control_damage * 8.0 && normalized < control_damage * 12.0,
        "Heart damage {normalized} vs unempowered {control_damage}"
    );
    assert_eq!(
        impacts.len(),
        5,
        "Cat Heart executes five Feral Frenzy hits"
    );
    assert!(impacts.iter().all(|event| event.amount > 0));
    assert!(ticks > 0, "the empowered frenzy applies its native bleed");
    let combo = crate::spell_cast::powers::power_value(
        run.app.world(),
        actor,
        shared::components::PowerType::ComboPoints,
    )
    .unwrap()
    .0;
    assert_eq!(
        combo, 5.0,
        "the five native children earn five combo points"
    );
}

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn guardian_mangle_earns_rank_scaled_vicious_brambles() {
    use shared::protocol::CombatLogKind;
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let percent = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1269617 && aura.caster == actor)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    assert_eq!(percent, 40.0, "the selected rank supplies 40% residual");
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 33917, Some(target), false)
        .unwrap();
    for _ in 0..260 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let mangle = events
        .iter()
        .find(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(33917)
                && event.kind == CombatLogKind::Damage
        })
        .expect("real Mangle damage")
        .amount;
    let ticks: i32 = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(1270065)
                && event.kind == CombatLogKind::Damage
                && event.periodic
        })
        .map(|event| event.amount)
        .sum();
    assert!(
        mangle > 0 && ticks > 0,
        "Mangle {mangle} must earn native rolling damage, got {ticks}"
    );
    let expected = mangle as f32 * percent / 100.0;
    assert!(
        (ticks as f32 - expected).abs() <= 8.0,
        "Mangle {mangle}, talent {percent}%, residual {ticks}"
    );
}

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn guardian_bear_entry_grants_rage_and_leaving_discards_it() {
    use crate::spell_cast::powers::power_value;
    use shared::components::PowerType;
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let rage = |run: &Run| {
        power_value(run.app.world(), actor, PowerType::Rage)
            .unwrap()
            .0
    };
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    assert_eq!(rage(&run), 250.0, "Bear entry earns native 25 Rage");
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    assert_eq!(rage(&run), 0.0, "leaving Bear consumes the stored Rage");
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    assert_eq!(rage(&run), 250.0, "re-entering Bear earns 25 Rage again");
}

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn guardian_absorbed_melee_special_earns_bear_rage_with_native_cooldown() {
    use crate::spell_cast::powers::{change_power, power_value};
    use shared::components::{Health, PowerType};
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let enemy = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 22812, Some(actor), false)
        .unwrap();
    change_power(run.app.world_mut(), actor, PowerType::Rage, -1000.0);
    run.app
        .insert_resource(super::super::healer_intake::Capture {
            members: vec![actor],
            ..Default::default()
        });
    let health = run.app.world().get::<Health>(actor).unwrap().current;
    let strike = |run: &mut Run| {
        crate::spell_cast::effects::replay_simc_dummy_strike(
            run.app.world_mut(),
            enemy,
            actor,
            100.0,
            false,
            false,
        );
        run.app.update();
    };
    let rage = |run: &Run| {
        power_value(run.app.world(), actor, PowerType::Rage)
            .unwrap()
            .0
    };
    strike(&mut run);
    assert_eq!(
        run.app.world().get::<Health>(actor).unwrap().current,
        health,
        "the hit must be absorbed"
    );
    assert_eq!(rage(&run), 30.0);
    let hit = super::super::healer_intake::Hit {
        tick: 1,
        source_guid: 0,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 100.0,
        level_scaling_factor: 1.0,
        kind: super::super::healer_intake::HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    crate::combat::replay_healer_hit(run.app.world_mut(), enemy, actor, &hit).unwrap();
    run.app.update();
    assert_eq!(
        rage(&run),
        30.0,
        "swing and special share the 750ms recovery"
    );
    for _ in 0..16 {
        run.app.update();
    }
    crate::combat::replay_healer_hit(run.app.world_mut(), enemy, actor, &hit).unwrap();
    run.app.update();
    assert_eq!(
        rage(&run),
        60.0,
        "a later absorbed swing grants Rage exactly once"
    );
    let capture = run
        .app
        .world()
        .resource::<super::super::healer_intake::Capture>();
    assert_eq!(capture.melee_rage.get(&(actor, "taken")), Some(&6.0));
    change_power(run.app.world_mut(), actor, PowerType::Rage, 930.0);
    for _ in 0..16 {
        run.app.update();
    }
    strike(&mut run);
    assert_eq!(rage(&run), 1000.0);
    let capture = run
        .app
        .world()
        .resource::<super::super::healer_intake::Capture>();
    assert_eq!(
        capture.melee_rage.get(&(actor, "taken")),
        Some(&7.0),
        "capture records the capped gain, not the attempted gain"
    );
}

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn druid_foreground_spell_resets_swing_but_mangle_and_form_do_not() {
    use shared::protocol::CombatLogKind;
    for (spell, should_reset) in [(8921, true), (33917, false), (768, false)] {
        let mut run = prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        run.app
            .insert_resource(super::super::healer_intake::Capture {
                members: vec![actor],
                ..Default::default()
            });
        run.app
            .world_mut()
            .entity_mut(actor)
            .insert(crate::combat::AutoAttack::new(target));
        for _ in 0..21 {
            run.app.update();
        }
        let captured = run
            .app
            .world()
            .resource::<super::super::healer_intake::Capture>()
            .melee_rage
            .get(&(actor, "swing"))
            .copied()
            .unwrap_or_default();
        assert!(captured > 0.0, "the real swing must report its earned Rage");
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            actor,
            spell,
            Some(if spell == 768 { actor } else { target }),
            false,
        )
        .unwrap();
        run.app
            .world_mut()
            .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .clear();
        for _ in 0..35 {
            run.app.update();
        }
        let swings = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
            })
            .count();
        assert_eq!(swings == 0, should_reset, "spell {spell} swings={swings}");
    }
}

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn guardian_bear_passive_changes_stamina_and_swing_rage_only_in_bear() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let stats = |run: &Run| {
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .primary
            .stamina
    };
    let rage_mod = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::AuraStatMods>(actor)
            .unwrap()
            .rage_from_damage_dealt_pct
    };
    let baseline_stamina = stats(&run);
    let baseline_rage_mod = rage_mod(&run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        stats(&run) > baseline_stamina,
        "Bear Form must increase stamina"
    );
    assert_eq!(
        rage_mod(&run),
        baseline_rage_mod - 8.0,
        "Bear passive must reduce swing Rage"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(
        rage_mod(&run),
        baseline_rage_mod,
        "leaving Bear Form must remove its Rage modifier"
    );
    assert!(
        stats(&run) < baseline_stamina * 1.01,
        "leaving Bear Form must remove its stamina bonus"
    );
}

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn guardian_incarnation_resets_real_mangle_and_thrash_cooldowns() {
    for spell in [33917, 77758] {
        let mut run = prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, spell, Some(target), false)
            .unwrap();
        for _ in 0..25 {
            run.app.update();
        }
        assert!(
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                spell,
                Some(target),
                false
            )
            .is_err(),
            "{spell} must be on cooldown before Incarnation"
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 102558, Some(actor), false)
            .unwrap();
        for _ in 0..25 {
            run.app.update();
        }
        assert!(
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                spell,
                Some(target),
                false
            )
            .is_ok(),
            "Incarnation must reset {spell}"
        );
    }
}

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn guardian_incarnation_shifts_cat_to_bear_before_thrash() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .has_spell(768)
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 102558, Some(actor), false)
        .unwrap();
    let auras = run.app.world().get::<ActiveAuras>(actor).unwrap();
    assert!(auras.has_spell(5487));
    assert!(!auras.has_spell(768));
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
        .unwrap();
}
