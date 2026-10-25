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
fn devastation_mass_disintegrate_native_chain_and_consumption() {
    let mut run = prepare_literal(1467, 3).unwrap();
    engage(&mut run);
    let primary = run.targets[0];
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
        preparation::finish_pending_setup(&mut run).unwrap();
    }
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
    let surge_cooldown = |run: &Run| {
        let world = run.app.world();
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(359073)
            .unwrap();
        world
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .unwrap()
            .remaining_cooldown_ms(spell, crate::spell_cast::now(world))
    };
    for remaining_uses in [1, 0] {
        let before = surge_cooldown(&run);
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
        assert_eq!(
            surge_cooldown(&run),
            before.saturating_sub(2000),
            "each native Sweep must earn the native two-second Surge reduction"
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
    let deep_breath =
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, 357210)
            .unwrap()
            .expect("selected Devastation actor knows Deep Breath");
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
                && event.kind == shared::protocol::CombatLogKind::CastSuccess
        }),
        "the literal Scalecommander APL must actually cast Deep Breath, absent from the follower kit"
    );
}
