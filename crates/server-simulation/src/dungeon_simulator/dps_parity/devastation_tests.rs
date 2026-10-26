//! Actual pinned-profile controller regression, not a replacement priority list.
use super::*;

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_literal_empower_rows_preserve_rank_and_primary() {
    let duration = |rank: u8| {
        let mut run = prepare_literal(1467, 1).unwrap();
        engage(&mut run);
        let row = format!("fire_breath,empower_to={rank},target_if=max:target.health.pct");
        let mut bindings = crate::simc_apl_native::Bindings::default();
        bindings.actions.insert("fire_breath".into(), 357208);
        let equipment =
            literal_rotation::equipment::Equipment::load(run.app.world(), run.actor, &mut bindings)
                .unwrap();
        let prepared = literal_rotation::execution::Prepared::read(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            &equipment,
            [literal_rotation::execution::NativeAction {
                name: "fire_breath",
                spell: 357208,
                talent: None,
                self_target: true,
            }],
            false,
        )
        .unwrap();
        let facts = crate::simc_apl_native::NativeFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            crate::simc_apl_native::Encounter::Fixture {
                started_at: 0.0,
                duration: 300.0,
                style: crate::simc_apl_native::FightStyle::Patchwerk,
            },
            1,
        )
        .with_targets(run.targets.first().copied(), &run.targets);
        let mut driver = literal_rotation::Driver::new(
            server::simc_apl::Program::parse(&[("default", row.as_str())]).unwrap(),
        );
        let (action, execution) = driver
            .select("default", &facts, &prepared, &equipment)
            .unwrap()
            .unwrap();
        driver
            .execute(run.app.world_mut(), run.actor, action, execution)
            .unwrap();
        for _ in 0..200 {
            run.app.update();
            if let Some(aura) = run
                .app
                .world()
                .get::<crate::auras::ActiveAuras>(run.targets[0])
                .and_then(|auras| {
                    auras
                        .iter()
                        .find(|a| a.spell_id == 357209 && a.caster == run.actor)
                })
            {
                return aura.duration_ms;
            }
        }
        panic!("rank{rank} release did not reach its selected enemy");
    };
    assert!(
        duration(1) > duration(3),
        "rank1 must retain more periodic duration than rank3"
    );
}

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_deep_breath_cast_damages_the_native_three_target_pack() {
    let mut run = prepare_literal(1467, 3).unwrap();
    engage(&mut run);
    let primary = run.targets[0];
    let destination = crate::spell_cast::targets::position(run.app.world(), primary)
        .unwrap()
        .to_array();
    crate::spell_cast::creature_cast_with_targets(
        run.app.world_mut(),
        run.actor,
        357210,
        crate::spell_cast::CastTargets {
            unit: primary,
            primary: Some(primary),
            destination: Some(destination),
        },
    )
    .unwrap();
    for _ in 0..200 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(357210)
                && event.kind == shared::protocol::CombatLogKind::CastSuccess
        }),
        "ordinary native Deep Breath must really complete its cast"
    );
    // SimC76f8118 sc_evoker.cpp6241–6245: every main impact executes353759.
    for target in &run.targets {
        assert!(
            events.iter().any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(353759)
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && event.amount > 0
            }),
            "native Deep Breath cast must damage pack recipient {target:?}"
        );
    }
}

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_strafing_run_earns_one_additional_deep_breath_without_restarting_recovery() {
    let mut run = prepare_literal(1467, 3).unwrap();
    engage(&mut run);
    let primary = run.targets[0];
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1266151),
        "the exact native profile selects Strafing Run"
    );
    let window = |run: &Run| crate::auras::has_aura(run.app.world(), run.actor, 1266165);
    let cast = |run: &mut Run| {
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            run.actor,
            433874,
            crate::spell_cast::CastTargets {
                unit: run.actor,
                primary: Some(primary),
                destination: None,
            },
        )
        .unwrap();
        preparation::finish_pending_setup(run).unwrap();
    };
    cast(&mut run);
    // SimC76f8118 sc_evoker.cpp6384–6398 grants only when no old window was present.
    assert!(
        window(&run),
        "first real Deep Breath must earn native Strafing Run"
    );
    let before = read_native_recharge_ms(&run, 433874);
    assert!(
        before > 10_000.0,
        "ordinary Deep Breath must start native recovery"
    );
    let started_at = crate::spell_cast::now(run.app.world());
    cast(&mut run);
    assert!(
        !window(&run),
        "the second Deep Breath must consume, not renew, the earned window"
    );
    let elapsed_ms = (crate::spell_cast::now(run.app.world()) - started_at) * 1000.0;
    let after = read_native_recharge_ms(&run, 433874);
    assert!(
        (before - after - elapsed_ms).abs() <= 3.0,
        "the earned extra cast must preserve original recovery: before={before}, after={after}, elapsed={elapsed_ms}"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert_eq!(
        events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(433874)
                    && event.kind == shared::protocol::CombatLogKind::CastSuccess
            })
            .count(),
        2,
        "two ordinary native casts, not synthesized child successes"
    );
}

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_scintillation_native_channel_ticks_launch_damage_children() {
    for selected in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(1467, 3).unwrap();
        engage(&mut run);
        let primary = run.targets[1];
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 370821));
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                run.actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 370821,
            );
        }
        crate::spell_cast::creature_cast_empowered_with_targets(
            run.app.world_mut(),
            run.actor,
            357208,
            crate::spell_cast::CastTargets {
                unit: run.actor,
                primary: Some(primary),
                destination: None,
            },
            1,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            run.actor,
            356995,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: Some(primary),
                destination: None,
            },
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..20 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events.iter().any(|event| event.spell_id == Some(356995)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0),
            "Scintillation must be tested after an actual native Disintegrate tick"
        );
        // SimC76f8118 sc_evoker.cpp6669–6677: each tick rolls native chance
        // and launches a rank-one Surge child at its actual tick recipient.
        let hits = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(359077)
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        assert_eq!(
            hits > 0,
            selected,
            "only selected native Scintillation may produce actual Surge damage: selected={selected}, hits={hits}"
        );
    }
}

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_shattering_stars_actual_surge_hits_only_its_selected_primary() {
    for selected in [false, true] {
        let mut run = prepare_literal(1467, 3).unwrap();
        engage(&mut run);
        let primary = run.targets[1];
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 1265802));
        assert!(
            !crate::auras::has_aura(run.app.world(), run.actor, 1265826),
            "this exact native profile has no Star Salvo secondary-target policy"
        );
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                run.actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 1265802,
            );
        }
        crate::spell_cast::creature_cast_empowered_with_targets(
            run.app.world_mut(),
            run.actor,
            359073,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: Some(primary),
                destination: None,
            },
            2,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events.iter().any(|event| event.spell_id == Some(359077)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0),
            "the real native rank-two Surge release must precede its star"
        );
        // SimC76f8118 sc_evoker.cpp5550–5573: without Salvo, only chain_target0.
        for target in &run.targets {
            let star = events.iter().any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1265804)
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && event.amount > 0
            });
            assert_eq!(
                star,
                selected && *target == primary,
                "native Shattering Stars selected={selected} recipient{target:?} primary{primary:?}"
            );
        }
    }
}

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_mass_disintegrate_redistributes_only_missing_native_recipients() {
    for count in [1, 2, 3, 4] {
        let hit = |empowered: bool| {
            let _seed = crate::random::seed_scope(70448);
            let mut run = prepare_literal(1467, count).unwrap();
            engage(&mut run);
            let primary = run.targets[0];
            crate::spell_cast::creature_cast_empowered_with_targets(
                run.app.world_mut(),
                run.actor,
                357208,
                crate::spell_cast::CastTargets {
                    unit: run.actor,
                    primary: Some(primary),
                    destination: None,
                },
                1,
            )
            .unwrap();
            preparation::finish_pending_setup(&mut run).unwrap();
            assert!(crate::auras::has_aura(run.app.world(), run.actor, 436336));
            let talent = run
                .app
                .world()
                .get::<crate::auras::ActiveAuras>(run.actor)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 436335)
                .unwrap();
            let native_targets = talent
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .unwrap()
                .amount;
            let native_percent = talent
                .effects
                .iter()
                .find(|effect| effect.index == 1)
                .unwrap()
                .amount;
            assert!(native_targets >= count as f32 && native_percent > 0.0);
            if !empowered {
                crate::auras::remove_auras(
                    run.app.world_mut(),
                    run.actor,
                    crate::auras::RemoveReason::Unlearned,
                    |aura| aura.spell_id == 436336,
                );
            }
            crate::spell_cast::creature_cast_with_targets(
                run.app.world_mut(),
                run.actor,
                356995,
                crate::spell_cast::CastTargets {
                    unit: primary,
                    primary: Some(primary),
                    destination: None,
                },
            )
            .unwrap();
            preparation::finish_pending_setup(&mut run).unwrap();
            let events = &run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0;
            let amount = events
                .iter()
                .find(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.target == Some(primary.to_bits())
                        && event.spell_id == Some(356995)
                        && event.kind == shared::protocol::CombatLogKind::Damage
                        && event.periodic
                        && !event.crit
                        && event.amount > 0
                })
                .expect("the actual native channel must supply a noncritical primary tick")
                .amount;
            (amount, native_targets, native_percent)
        };
        let (baseline, _, _) = hit(false);
        let (empowered, native_targets, native_percent) = hit(true);
        // SimC76f8118 sc_evoker.cpp6560–6567: use the effective native talent,
        // including selected Concentrated Power, not the raw unmodified DB2 points.
        let multiplier =
            1.0 + f64::from(native_targets - count as f32) * f64::from(native_percent) / 100.0;
        let expected = f64::from(baseline) * multiplier;
        assert!(
            (f64::from(empowered) - expected).abs() <= 3.0,
            "Mass Disintegrate count{count}: native targets={native_targets}, native percent={native_percent}, baseline={baseline}, actual={empowered}, expected={expected}"
        );
    }
}

fn cast_reference_empowers(run: &mut Run, primary: Entity) {
    for (spell, unit) in [(357208, run.actor), (359073, primary)] {
        crate::spell_cast::creature_cast_empowered_with_targets(
            run.app.world_mut(),
            run.actor,
            spell,
            crate::spell_cast::CastTargets {
                unit,
                primary: Some(primary),
                destination: None,
            },
            1,
        )
        .unwrap();
        preparation::finish_pending_setup(run).unwrap();
    }
}

fn read_native_recharge_ms(run: &Run, id: u32) -> f64 {
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("measured".into(), id)].into(),
        ..Default::default()
    };
    let facts = crate::simc_apl_native::NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        crate::simc_apl_native::Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        },
        run.targets.len(),
    );
    server::simc_apl::Facts::value(&facts, "cooldown.measured.full_recharge_time").unwrap() * 1000.0
}

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_causality_reduces_empowers_once_per_primary_tick() {
    for (active, with_primary) in [(false, true), (true, false), (true, true)] {
        let mut run = prepare_literal(1467, 3).unwrap();
        engage(&mut run);
        // Choose an add, not the actor's default boss victim.
        let primary = run.targets[1];
        cast_reference_empowers(&mut run, primary);
        let reduction_ms = -run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 375777)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .amount;
        assert!(
            reduction_ms > 0.0,
            "native Causality supplies a negative cooldown delta"
        );
        if !active {
            crate::auras::remove_auras(
                run.app.world_mut(),
                run.actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 375777,
            );
        }
        let before = [
            read_native_recharge_ms(&run, 357208),
            read_native_recharge_ms(&run, 359073),
        ];
        assert!(
            before.iter().all(|remaining| *remaining > 5000.0),
            "measure real active empower charge recovery, not the expired one-second spell throttle: {before:?}"
        );
        let started_at = crate::spell_cast::now(run.app.world());
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            run.actor,
            356995,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: with_primary.then_some(primary),
                destination: None,
            },
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        let elapsed_ms = (crate::spell_cast::now(run.app.world()) - started_at) * 1000.0;
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let ticks: Vec<_> = run
            .targets
            .iter()
            .map(|target| {
                events
                    .iter()
                    .filter(|event| {
                        event.source == Some(run.actor.to_bits())
                            && event.target == Some(target.to_bits())
                            && event.spell_id == Some(356995)
                            && event.kind == shared::protocol::CombatLogKind::Damage
                            && event.periodic
                            && event.amount > 0
                    })
                    .count()
            })
            .collect();
        assert!(
            ticks.iter().all(|count| *count > 1),
            "the real native channel must tick on all three recipients: {ticks:?}"
        );
        // SimC76f8118 sc_evoker.cpp6694–6699: only main ticks reduce both timers.
        let expected = if active && with_primary {
            f64::from(reduction_ms) * ticks[1] as f64
        } else {
            0.0
        };
        for (id, before) in [357208, 359073].into_iter().zip(before) {
            let actual = before - read_native_recharge_ms(&run, id) - elapsed_ms;
            assert!(
                (actual - expected).abs() <= 3.0,
                "native Causality spell{id}: active={active}, primary={with_primary}, ticks={ticks:?}, reduction={actual}, expected={expected}"
            );
        }
    }
}

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_mass_disintegrate_native_chain_and_consumption() {
    let mut run = prepare_literal(1467, 3).unwrap();
    engage(&mut run);
    let primary = run.targets[0];
    cast_reference_empowers(&mut run, primary);
    let stacks = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == 436336 && aura.caster == run.actor)
            })
            .map_or(0, |aura| aura.stacks)
    };
    assert_eq!(stacks(&run), 2, "two actual empowers earn two uses");
    crate::spell_cast::creature_cast_with_targets(
        run.app.world_mut(),
        run.actor,
        356995,
        crate::spell_cast::CastTargets {
            unit: primary,
            primary: Some(primary),
            destination: None,
        },
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    // SimC76f8118 sc_evoker.cpp6584–6595: only the primary impact spends one use.
    assert_eq!(
        stacks(&run),
        1,
        "one Disintegrate must spend exactly one earned use"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        assert!(
            events.iter().any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(356995)
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && event.amount > 0
            }),
            "earned Mass Disintegrate must really damage recipient {target:?}"
        );
    }
}

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_mid1_two_piece_sweep_bonus_is_primary_only() {
    let damage = |with_tier: bool| {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(1467, 3).unwrap();
        engage(&mut run);
        let primary = run.targets[0];
        crate::spell_cast::creature_cast_empowered_with_targets(
            run.app.world_mut(),
            run.actor,
            359073,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: Some(primary),
                destination: None,
            },
            1,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 1264821));
        if !with_tier {
            crate::auras::remove_auras(
                run.app.world_mut(),
                run.actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 1264821,
            );
        }
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            run.actor,
            1265872,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: Some(primary),
                destination: None,
            },
        )
        .unwrap();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        run.targets
            .iter()
            .map(|target| {
                events
                    .iter()
                    .find(|event| {
                        event.source == Some(run.actor.to_bits())
                            && event.target == Some(target.to_bits())
                            && event.spell_id == Some(1265872)
                            && event.kind == shared::protocol::CombatLogKind::Damage
                            && event.amount > 0
                    })
                    .expect("ordinary earned Sweep must really damage each native recipient")
                    .amount
            })
            .collect::<Vec<_>>()
    };
    let baseline = damage(false);
    let tier = damage(true);
    for index in 0..3 {
        // Native1264821 effect0 supplies50% to all; sc_evoker.cpp5691–5698
        // applies native effect1's extra100% only to the selected primary.
        let multiplier = if index == 0 { 3.0 } else { 1.5 };
        let expected = f64::from(baseline[index]) * multiplier;
        assert!(
            (f64::from(tier[index]) - expected).abs() <= 3.0,
            "native tier Sweep recipient{index}: actual{}, baseline{}, expected{expected}",
            tier[index],
            baseline[index]
        );
    }
}

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_eternity_surge_earns_native_azure_sweep_action() {
    let mut run = prepare_literal(1467, 3).unwrap();
    engage(&mut run);
    let primary = run.targets[0];
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1264822),
        "the exact profile must earn native MID1 four-piece"
    );
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 1265871));
    crate::spell_cast::creature_cast_empowered_with_targets(
        run.app.world_mut(),
        run.actor,
        359073,
        crate::spell_cast::CastTargets {
            unit: primary,
            primary: Some(primary),
            destination: None,
        },
        1,
    )
    .unwrap();
    // SimC76f8118 sc_evoker.cpp5592–5599: Surge execute grants Azure Sweep.
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1265871),
        "actual Eternity Surge must earn the native Azure Sweep action override"
    );
    let sweep_stacks = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == 1265871 && aura.caster == run.actor)
            })
            .map_or(0, |aura| aura.stacks)
    };
    // SimC76f8118 sc_evoker.cpp10853–10856: actual MID1 four-piece adds one use.
    assert_eq!(
        sweep_stacks(&run),
        2,
        "native MID1 four-piece must earn two Sweep uses"
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, 362969).unwrap(),
        Some(1265872),
        "native aura332 must expose Azure Sweep through known Azure Strike"
    );
    for remaining_uses in [1, 0] {
        let before = read_native_recharge_ms(&run, 359073);
        assert!(
            before > 2000.0,
            "native Surge recovery must actually be active"
        );
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            run.actor,
            1265872,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: Some(primary),
                destination: None,
            },
        )
        .unwrap();
        // sc_evoker.cpp5704–5710: actual tier effect1 reduces the running Surge cooldown.
        assert!(
            (read_native_recharge_ms(&run, 359073) - (before - 2000.0)).abs() <= 2.0,
            "each native Sweep must earn the native two-second Surge charge-recovery reduction"
        );
        preparation::finish_pending_setup(&mut run).unwrap();
        assert_eq!(
            sweep_stacks(&run),
            remaining_uses,
            "each Sweep must spend one earned use, not the entire tier grant"
        );
    }
    assert_eq!(
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, 362969).unwrap(),
        Some(362969),
        "spending the last earned Sweep must restore native Azure Strike"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        assert!(
            events.iter().any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1265872)
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && event.amount > 0
            }),
            "earned Azure Sweep must really damage recipient {target:?}"
        );
    }
}

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_empower_release_grants_selected_mass_disintegrate() {
    let mut run = prepare_literal(1467, 1).unwrap();
    engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 436335),
        "the exact profile must select native Mass Disintegrate"
    );
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 436336));
    crate::spell_cast::creature_cast_empowered_with_targets(
        run.app.world_mut(),
        run.actor,
        357208,
        crate::spell_cast::CastTargets {
            unit: run.actor,
            primary: Some(run.targets[0]),
            destination: None,
        },
        1,
    )
    .unwrap();
    for _ in 0..200 {
        run.app.update();
        let released = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(run.targets[0])
            .is_some_and(|auras| {
                auras
                    .iter()
                    .any(|aura| aura.spell_id == 357209 && aura.caster == run.actor)
            });
        if released {
            // SimC76f8118 sc_evoker.cpp4703–4704: empower release grants one stack.
            let stacks = run
                .app
                .world()
                .get::<crate::auras::ActiveAuras>(run.actor)
                .and_then(|auras| {
                    auras
                        .iter()
                        .find(|aura| aura.spell_id == 436336 && aura.caster == run.actor)
                })
                .map_or(0, |aura| aura.stacks);
            assert_eq!(
                stacks, 1,
                "successful Fire Breath release must earn Mass Disintegrate"
            );
            return;
        }
    }
    panic!("native Fire Breath release never reached its selected target");
}

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_literal_measurement_executes_reference_deep_breath() {
    let mut run = prepare_measurement(1467, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 433871),
        "the exact reference selects native Maneuverability"
    );
    let deep_breath =
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, 357210)
            .unwrap()
            .expect("selected Devastation actor knows Deep Breath");
    // SimC76f8118 sc_evoker.cpp6323–6327 selects the actual Maneuverability endpoint.
    assert_eq!(
        deep_breath, 433874,
        "selected Maneuverability must replace the base Deep Breath action"
    );
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    engage(&mut run);
    for _ in 0..240 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(deep_breath)
                && event.target == Some(run.actor.to_bits())
                && event.kind == shared::protocol::CombatLogKind::CastSuccess
        }),
        "Maneuverability's native caster-only endpoint must keep its self cast unit; cast targets={:?}",
        events
            .iter()
            .filter(|event| event.spell_id == Some(deep_breath)
                && event.kind == shared::protocol::CombatLogKind::CastSuccess)
            .map(|event| event.target)
            .collect::<Vec<_>>()
    );
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(353759)
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
        }),
        "the self cast must retain the actual hostile primary for native Deep Breath damage"
    );
}
