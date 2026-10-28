use std::path::Path;

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
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(effective > 0.0 && raw >= effective, "{report}");
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    println!("NATIVE_HOLY_PRIEST={report}");
}
