//! Native Devourer Burning Blades relationship.

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_voidblade_damage_applies_native_burning_blades() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    // SimC76f8118 sc_demon_hunter.cpp5830–5847: the native Voidblade child
    // is also used by Pierce the Veil and triggers Burning Blades on impact.
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1245414, Some(target), 1);
    for _ in 0..80 {
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
            .any(|event| event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1245414)
                && event.amount > 0),
        "native Voidblade child damage must land before checking Burning Blades"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1245654)
                && event.amount > 0
                && event.periodic
                && event.target == Some(target.to_bits())),
        "Voidblade damage must produce Devourer Burning Blades damage"
    );
}

use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native1480 player profile and three real targets"]
fn devourer_predators_wake_applies_hunt_dot_to_nearby_enemies() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(1480, 3).expect("native Devourer profile");
    let actor = run.actor;
    let primary = run.targets[0];
    engage(&mut run);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1259431, Some(primary), 1);
    for _ in 0..160 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1246168)
                    && event.kind == CombatLogKind::Damage
                    && event.periodic
                    && event.amount > 0),
            "Predator's Wake must deliver real Hunt periodic damage to {target:?}"
        );
    }
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_weapon_hits_apply_native_burning_blades() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::AutoAttack(target),
    )
    .expect("native weapon swings");
    for _ in 0..300 {
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
            .any(|event| event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
                && event.amount > 0),
        "native weapon hit must land before checking Burning Blades"
    );
    // SimC76f8118 sc_demon_hunter.cpp3088–3111,6875–6878,11390–11394:
    // landed auto attacks apply the selected talent's Devourer residual1245654.
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1245654)
                && event.amount > 0
                && event.periodic
                && event.target == Some(target.to_bits())),
        "native weapon hits must produce Devourer Burning Blades damage"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_real_voidblade_executes_damage_and_fury_child() {
    use shared::components::PowerType;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, -1000.0)
        .expect("native Fury pool");
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        1245412,
        Some(target),
    )
    .expect("real selected Voidblade admitted");
    for _ in 0..100 {
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
            .any(|event| event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id == Some(1245412)),
        "native Voidblade cast succeeded"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1245414)
                && event.target == Some(target.to_bits())
                && event.amount > 0),
        "Voidblade must deliver its native damage child"
    );
    let fury = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    assert!(
        fury > 0.0,
        "native Voidblade damage child must energize Fury"
    );
}
