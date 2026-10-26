//! Real Shuriken Storm per-hit CP gains collect earned Shadow Techniques.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

fn cast_storm(initial_points: f32) -> Run {
    let mut run = prepare_literal(261, 3).unwrap();
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
    {
        let mut pools = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        pools.get_mut(PowerType::Energy).unwrap().current = 100.0;
        pools.get_mut(PowerType::ComboPoints).unwrap().current = initial_points;
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(197835, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    run
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_storm_funding_collects_after_actual_three_target_combo_gains() {
    let run = cast_storm(0.0);
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(197835)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "Storm actually hits each target"
        );
    }
    let (points, max) =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::ComboPoints)
            .unwrap();
    assert_eq!(
        points, max,
        "Storm's three base points precede collection of four stored points"
    );
    let stacks = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 196911 && aura.caster == run.actor)
        .unwrap()
        .stacks;
    assert_eq!(stacks, 1, "unspent storage remains for a later builder");
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_storm_funding_logs_each_actual_combo_gain_once() {
    let run = cast_storm(0.0);
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let gains: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.actor.to_bits())
                && event.spell_id == Some(197835)
                && event.kind == CombatLogKind::Energize
        })
        .collect();
    assert_eq!(
        gains.len(),
        3,
        "each actual native Storm gain emits one combo-point receipt"
    );
    assert_eq!(
        gains
            .iter()
            .map(|event| event.amount - event.overflow)
            .sum::<i32>(),
        3
    );
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_storm_funding_at_cap_keeps_storage_and_arms_ancient_arts() {
    let run = cast_storm(7.0);
    let stacks = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 196911 && aura.caster == run.actor)
        .unwrap()
        .stacks;
    assert_eq!(stacks, 5, "capped gains cannot consume stored points");
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1269163),
        "an actual builder at cap still evaluates the native Ancient Arts threshold"
    );
}
