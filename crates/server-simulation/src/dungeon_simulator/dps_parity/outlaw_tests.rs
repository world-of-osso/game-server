use super::*;
use server::simc_apl::Facts;

#[test]
#[ignore = "requires private native260 profile/traits and player actor"]
fn outlaw_literal_uses_the_known_roll_the_bones_action() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(260, 1).unwrap();
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..200 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id == Some(1214909)),
        "literal Roll the Bones must execute the native specialization action"
    );
}

#[test]
#[ignore = "requires private native260 profile/traits and player actor"]
fn outlaw_earned_fang_strike_finisher_deals_damage_and_consumes_the_use() {
    use shared::components::PowerType;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    for _ in 0..32 {
        if crate::auras::has_aura(run.app.world(), run.actor, 1301405) {
            break;
        }
        {
            let mut powers = run
                .app
                .world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
                .unwrap();
            let energy = powers.get_mut(PowerType::Energy).unwrap();
            energy.current = energy.max;
        }
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                193315,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    let instance = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1301405)
        .expect("ordinary attacks must earn native Fang Strike")
        .instance_id;
    {
        let mut powers = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        powers.get_mut(PowerType::Energy).unwrap().current = 0.0;
        powers.get_mut(PowerType::ComboPoints).unwrap().current = 0.0;
    }
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(2098, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(2098)
                && event.amount > 0),
        "a free Fang Strike finisher uses effective combo points, not zero paid cost"
    );
    assert!(
        !run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .any(|aura| aura.instance_id == instance),
        "the earned Fang Strike use must be consumed"
    );
}
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native260 profile and traits"]
fn rogue_literal_spend_cap_and_base_energy_ignore_temporary_capacity() {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle};
    use shared::components::PowerType;
    let mut run = prepare_literal(260, 1).unwrap();
    let mut bindings = Bindings {
        resources: [
            ("energy".into(), PowerType::Energy),
            ("combo_points".into(), PowerType::ComboPoints),
        ]
        .into(),
        ..default()
    };
    let equipment =
        literal_rotation::equipment::Equipment::load(run.app.world_mut(), run.actor, &mut bindings)
            .unwrap();
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let read = |run: &Run, name| {
        assassination_rotation::RogueFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            &equipment,
            encounter,
            &run.targets,
        )
        .value(name)
        .unwrap()
    };
    assert_eq!(
        read(&run, "cp_max_spend"),
        7.0,
        "selected Deeper and Devious Stratagem"
    );
    let maximum = read(&run, "energy.max");
    {
        let mut pools = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        pools.get_mut(PowerType::Energy).unwrap().current = 100.0;
    }
    let deficit = maximum - 100.0;
    assert_eq!(read(&run, "energy.base_deficit"), deficit);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.actor,
            target: run.actor,
            spell_id: 13750,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    run.app.update();
    run.app
        .world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
        .unwrap()
        .get_mut(PowerType::Energy)
        .unwrap()
        .current = 100.0;
    assert_eq!(
        read(&run, "energy.max"),
        maximum + 50.0,
        "native Adrenaline Rush max Energy"
    );
    assert_eq!(read(&run, "energy.base_deficit"), deficit);
    let expected = deficit / read(&run, "energy.regen");
    assert!((read(&run, "energy.base_time_to_max") - expected).abs() < 0.001);
}

#[test]
#[ignore = "requires private native260 profile/traits and DPS_PARITY_ACTOR=player"]
fn outlaw_literal_driver_coats_and_executes_profile_opener() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(260, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_some(),
        "Outlaw must use the literal source instead of AssistedCombat"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 315584),
        "literal Instant Poison"
    );
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..200 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && matches!(event.spell_id, Some(193315 | 8676 | 185763 | 315341))),
        "literal Outlaw source must execute an offensive action"
    );
}
