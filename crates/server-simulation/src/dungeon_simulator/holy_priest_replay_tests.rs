use std::path::Path;

#[test]
#[ignore = "requires native Holy Priest spell data"]
fn divine_hymn_pulses_emit_scaled_cosmic_ripple() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::components::Health;
    use shared::protocol::CombatLogKind;
    let mut h = harness(Vec::new(), &[64843, 64844, 1262760, 243241], 5).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let ally = spawn_player(&mut h.app, 5, 3.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 1.0,
        max: 1_000_000.0,
    };
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 0.0;
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 243241, Some(h.caster), 1);
    h.advance(2);
    let baseline = h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .find(|e| {
            e.kind == CombatLogKind::Heal
                && e.spell_id == Some(243241)
                && e.target == Some(ally.to_bits())
        })
        .unwrap()
        .amount;
    h.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1262760,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 64843, Some(h.caster), true)
        .unwrap();
    h.advance(100);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events.iter().any(|e| e.kind == CombatLogKind::Heal
            && e.spell_id == Some(64844)
            && e.target == Some(ally.to_bits())),
        "native hymn pulse must remain: {events:?}"
    );
    let ripple = events
        .iter()
        .find(|e| {
            e.kind == CombatLogKind::Heal
                && e.spell_id == Some(243241)
                && e.target == Some(ally.to_bits())
        })
        .expect("hymn must emit ripple");
    assert!(
        (ripple.amount as f32 - baseline as f32 * 0.75).abs() <= baseline as f32 * 0.08 + 1.0,
        "baseline={baseline}, ripple={ripple:?}"
    );
}

#[test]
#[ignore = "requires native Holy Priest spell data"]
fn serenity_upgrades_the_next_real_flash_heal_to_benediction() {
    use crate::spell_cast::{
        CastTargets, cast_for_player_with_targets,
        test_harness::{ObservedCombatEvents, harness, spawn_player},
    };
    use shared::components::{Health, Mana};
    use shared::protocol::CombatLogKind;
    let mut h = harness(
        Vec::new(),
        &[2050, 2061, 1262755, 1262760, 1262763, 1262766, 243241],
        5,
    )
    .unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let ally = spawn_player(&mut h.app, 5, 3.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 1.0,
        max: 1_000_000.0,
    };
    *h.app.world_mut().get_mut::<Mana>(h.caster).unwrap() = Mana {
        current: 250_000.0,
        max: 250_000.0,
    };
    for id in [1262755, 1262760] {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: h.caster,
                spell_id: id,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 2050, Some(ally), true).unwrap();
    h.advance(2);
    assert_eq!(
        crate::auras::overrides::resolve_spell(h.app.world(), h.caster, 2061),
        1262763
    );
    cast_for_player_with_targets(
        h.app.world_mut(),
        h.caster,
        2061,
        CastTargets {
            unit: ally,
            primary: Some(ally),
            destination: None,
        },
    )
    .unwrap();
    h.advance(100);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events.iter().any(|e| e.kind == CombatLogKind::Heal
            && e.spell_id == Some(1262763)
            && e.target == Some(ally.to_bits())
            && e.amount > 0),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| e.kind == CombatLogKind::Heal && e.spell_id == Some(243241)),
        "Benediction must emit Cosmic Ripple: {events:?}"
    );
    assert_eq!(
        crate::auras::overrides::resolve_spell(h.app.world(), h.caster, 2061),
        2061
    );
}

#[test]
#[ignore = "requires native Holy Priest spell data"]
fn piety_redistributes_flash_heal_overhealing() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::components::Health;
    use shared::protocol::CombatLogKind;
    let mut h = harness(Vec::new(), &[2061, 1246802, 443126], 10).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let full = spawn_player(&mut h.app, 10, 3.0);
    let first = spawn_player(&mut h.app, 10, 4.0);
    let second = spawn_player(&mut h.app, 10, 5.0);
    for unit in [first, second] {
        *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
            current: 1.0,
            max: 100_000.0,
        };
    }
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1246802,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 2061, Some(full), true).unwrap();
    h.advance(2);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    let source = events
        .iter()
        .find(|e| {
            e.kind == CombatLogKind::Heal
                && e.spell_id == Some(2061)
                && e.target == Some(full.to_bits())
        })
        .expect("Flash Heal must land");
    assert!(source.overflow > 0);
    for ally in [first, second] {
        let heal = events
            .iter()
            .find(|e| {
                e.kind == CombatLogKind::Heal
                    && e.spell_id == Some(443126)
                    && e.target == Some(ally.to_bits())
            })
            .expect("Piety must heal injured ally");
        assert!((heal.amount as f32 - source.overflow as f32 * 0.1).abs() <= 1.0);
    }
}

#[test]
#[ignore = "requires native Holy Priest spell data"]
fn ultimate_serenity_heals_nearby_injured_allies() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::components::Health;
    use shared::protocol::CombatLogKind;
    let mut h = harness(Vec::new(), &[2050, 1246517, 1250548, 1250581], 10).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let primary = spawn_player(&mut h.app, 10, 3.0);
    let nearby = spawn_player(&mut h.app, 10, 5.0);
    let far = spawn_player(&mut h.app, 10, 100.0);
    for unit in [primary, nearby, far] {
        *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
            current: 1.0,
            max: 100_000.0,
        };
    }
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1246517,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 2050, Some(primary), true)
        .unwrap();
    h.advance(2);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    assert!(events.iter().any(|event| event.kind == CombatLogKind::Heal
        && event.spell_id == Some(2050)
        && event.target == Some(primary.to_bits())));
    assert!(
        events.iter().any(|event| event.kind == CombatLogKind::Heal
            && event.spell_id == Some(1250581)
            && event.target == Some(nearby.to_bits())
            && event.amount > 0),
        "{events:?}"
    );
    assert!(!events.iter().any(|event| event.spell_id == Some(1250581)
        && matches!(event.target, Some(t) if t == primary.to_bits() || t == far.to_bits())));
}

#[test]
#[ignore = "requires native Holy Priest spell data"]
fn prayer_of_mending_heals_its_target_only_with_prompt_prognosis() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::components::Health;
    use shared::protocol::CombatLogKind;

    let mut h = harness(Vec::new(), &[33076, 41635, 1246799, 1246798], 10).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let ally = spawn_player(&mut h.app, 10, 3.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 1.0,
        max: 100_000.0,
    };
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 33076, Some(ally), true).unwrap();
    h.advance(2);
    assert!(
        !h.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.spell_id == Some(1246798))
    );
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1246799,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 33076, Some(ally), true).unwrap();
    h.advance(2);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    assert!(events.iter().any(|event| event.kind == CombatLogKind::Heal
        && event.spell_id == Some(1246798)
        && event.target == Some(ally.to_bits())
        && event.amount > 0));
}

#[test]
#[ignore = "requires the canonical MID2 Holy Priest profile and world database"]
fn holy_priest_spells_target_the_tank_and_pay_native_mana() {
    use crate::spell_cast::{
        CastTargets, cast_for_player_with_targets,
        test_harness::{ObservedCombatEvents, ObservedPowerChanges},
    };
    use shared::components::{Health, Mana, PowerType};
    use shared::protocol::CombatLogKind;
    let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    let healer = super::super::dps_parity::spawn_second_reference_player(&mut run, 257).unwrap();
    let faction = *run
        .app
        .world()
        .get::<shared::components::UnitFactionTemplate>(tank)
        .unwrap();
    run.app.world_mut().entity_mut(healer).insert(faction);
    crate::followers::simulator::spawn_party_for_reference_healer(
        run.app.world_mut(),
        tank,
        healer,
    )
    .unwrap();
    run.app.insert_resource(ObservedPowerChanges::default());
    // Renew is not selected in this MID2 profile. Grant the requested optional
    // spells in this fixture so their native casts can still be exercised.
    let mut known: Vec<_> = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(healer)
        .unwrap()
        .iter()
        .collect();
    assert!(
        !known.contains(&139),
        "MID2 profile unexpectedly selects Renew"
    );
    known.extend([139, 204883]);
    run.app
        .world_mut()
        .entity_mut(healer)
        .insert(crate::class_progression::KnownSpells::fixture(known));
    for id in [2050, 2061, 139, 204883] {
        for _ in 0..100 {
            run.app.update();
        }
        {
            let mut hp = run.app.world_mut().get_mut::<Health>(tank).unwrap();
            hp.current = hp.max * 0.25;
        }
        {
            let mut mana = run.app.world_mut().get_mut::<Mana>(healer).unwrap();
            mana.current = mana.max;
        }
        run.app
            .world_mut()
            .resource_mut::<ObservedCombatEvents>()
            .0
            .clear();
        run.app
            .world_mut()
            .resource_mut::<ObservedPowerChanges>()
            .0
            .clear();
        let expected_cost: f32 = {
            let world = run.app.world();
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(id)
                .unwrap();
            crate::spell_cast::applicable_costs(world, healer, spell)
                .unwrap()
                .into_iter()
                .filter(|(pool, _, _)| *pool == crate::spell_cast::CostPool::Power(PowerType::Mana))
                .map(|(_, amount, _)| amount)
                .sum()
        };
        cast_for_player_with_targets(
            run.app.world_mut(),
            healer,
            id,
            CastTargets {
                unit: tank,
                primary: Some(tank),
                destination: None,
            },
        )
        .unwrap_or_else(|e| panic!("spell {id} rejected: {e:?}"));
        for _ in 0..100 {
            run.app.update();
        }
        let world = run.app.world();
        let costs = &world.resource::<ObservedPowerChanges>().0;
        let paid: f32 = costs
            .iter()
            .filter(|&&(actor, power, amount)| {
                actor == healer && power == PowerType::Mana && amount < 0.0
            })
            .map(|&(_, _, amount)| -amount)
            .sum();
        assert!(
            expected_cost > 0.0 && (paid - expected_cost).abs() < 1.0,
            "spell {id}: paid {paid}, native cost {expected_cost}"
        );
        println!("HOLY_MANA spell={id} paid={paid} native={expected_cost}");
        if id == 139 {
            assert!(
                world
                    .get::<crate::auras::ActiveAuras>(tank)
                    .is_some_and(|auras| auras
                        .iter()
                        .any(|a| a.spell_id == 139 && a.caster == healer)),
                "Renew did not reach tank"
            );
        } else {
            let events = &world.resource::<ObservedCombatEvents>().0;
            let healer_events: Vec<_> = events
                .iter()
                .filter(|e| e.source == Some(healer.to_bits()))
                .collect();
            assert!(
                events.iter().any(|e| e.kind == CombatLogKind::Heal
                    && e.target == Some(tank.to_bits())
                    && e.source == Some(healer.to_bits())),
                "spell {id} did not heal tank, healer={healer:?}, tank={tank:?}, events={healer_events:?}"
            );
        }
    }
}

#[test]
#[ignore = "requires MID2 profiles, canonical world database and SimC trace"]
fn holy_priest_heals_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::super::healer_replay::replay_simc(257, Path::new(&trace), 300, 70448).unwrap();
    println!("NATIVE_HOLY_PRIEST={report}");
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(effective > 0.0 && raw >= effective, "{report}");
    let casts = report["healer_casts"].as_object().unwrap();
    let total: u64 = casts.values().map(|value| value.as_u64().unwrap()).sum();
    assert!(
        total >= 60,
        "Holy policy spent too much time idle: {report}"
    );
    assert!(
        casts
            .get("2061")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0,
        "Holy did not cast Flash Heal: {report}"
    );
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
}
