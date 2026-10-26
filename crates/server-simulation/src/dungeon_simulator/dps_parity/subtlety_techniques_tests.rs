//! Real white swings store points; a native builder spends only the missing capacity.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

fn stored(run: &Run) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 196911 && aura.caster == run.actor)
        })
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_techniques_actual_white_hits_store_points_without_spending_them() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::combat::AutoAttack::new(run.targets[0]));
    for _ in 0..200 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let whites = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
        })
        .count();
    assert!(whites >= 6, "real native white swings must land: {whites}");
    assert!(
        stored(&run) >= 2,
        "landed swings must store Shadow Techniques points, not lose the producer"
    );
    let (points, _) =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::ComboPoints)
            .unwrap();
    assert_eq!(
        points, 0.0,
        "white hits store points; only builders collect them"
    );
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_techniques_native_builder_collects_only_points_missing_after_base_gain() {
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.actor,
            target: run.actor,
            spell_id: 196911,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::set_self_cast_stacks(
        run.app.world_mut(),
        run.actor,
        196911,
        5
    ));
    let (_, maximum) =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::ComboPoints)
            .unwrap();
    {
        let mut pools = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        pools.get_mut(PowerType::ComboPoints).unwrap().current = maximum - 2.0;
        let energy = pools.get_mut(PowerType::Energy).unwrap();
        energy.current = energy.max;
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(53, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let (points, _) =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::ComboPoints)
            .unwrap();
    assert_eq!(
        points, maximum,
        "native Backstab base point must precede stored-point collection"
    );
    assert_eq!(
        stored(&run),
        4,
        "collect only one missing point, preserving four for later builders"
    );
}
