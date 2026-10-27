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
