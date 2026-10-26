//! Native displacement must not strand a literal melee actor outside combat.
use super::*;
use shared::components::Position;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn shared_fixture_displacement_returns_to_melee_after_real_retreat() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 1).unwrap();
    havoc_tests::start_havoc_swings(&mut run);
    let start = *run.app.world().get::<Position>(run.actor).unwrap();
    literal_rotation::execution::execute(run.app.world_mut(), run.actor,
        literal_rotation::execution::Execution::Spell(198793, run.actor, Some(run.targets[0]))).unwrap();
    let mut displaced = false;
    for frame in 0..240 {
        let boundary = run.app.world().resource::<Time>().elapsed() + Duration::from_millis(50);
        advance_fixture_step(&mut run, boundary).unwrap();
        let position = run.app.world().get::<Position>(run.actor).unwrap();
        displaced |= (position.x - start.x).hypot(position.z - start.z) > 5.0;
        if frame == 140 {
            run.app.world_mut().resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.clear();
        }
    }
    assert!(displaced, "the real native Retreat must still displace the actor");
    let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    assert!(events.iter().any(|event| event.kind == CombatLogKind::Damage
        && event.source == Some(run.actor.to_bits()) && event.spell_id.is_none()
        && event.amount > 0), "native melee swings must resume after the actual flight and finite return walk");
    assert!(events.iter().any(|event| event.kind == CombatLogKind::Energize
        && event.spell_id == Some(203796) && event.amount > 0),
        "real Demon Blades must again fund Fury after returning");
}
