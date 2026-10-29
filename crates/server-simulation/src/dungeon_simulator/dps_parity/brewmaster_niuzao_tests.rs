//! Invoke Niuzao must create the actual native summon.
use super::*;
use crate::spell_cast::summon::SummonedCreature;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn invoke_niuzao_creates_the_native_summon() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 132578, Some(target), false)
        .unwrap();
    let world = run.app.world_mut();
    let pets: Vec<_> = world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, summon)| summon.owner == actor && summon.spell_id == 132578)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(pets.len(), 1, "Invoke must create one real Niuzao");
    let pet = pets[0];
    for _ in 0..300 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 205523, Some(target), false)
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
        events
            .iter()
            .any(|event| event.source == Some(pet.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
                && event.amount > 0),
        "Niuzao performs a real melee swing"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(pet.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(227291)
                && event.amount > 0),
        "Blackout Kick causes the pet's native Stomp"
    );
}
