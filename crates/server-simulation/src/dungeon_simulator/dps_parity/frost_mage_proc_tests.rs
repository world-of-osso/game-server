use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_orb_earns_procs_and_lance_retains_spent_fingers_in_flight() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(84714, run.actor, Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 44544),
        "selected Everlasting Frost must earn native Fingers from the real Orb cast"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 190446),
        "selected Wintertide must earn native Brain Freeze from the real Orb cast"
    );
    let freezing = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.targets[0])
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1221389 && aura.caster == run.actor)
            .map_or(0, |aura| aura.stacks)
    };
    let before = freezing(&run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(30455, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 44544),
        "Lance must spend its earned Fingers at execution, before impact"
    );
    for _ in 0..40 {
        run.app.update();
    }
    let shatters = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1246949)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(
        shatters, 1,
        "the missile must retain Fingers after the live buff was spent"
    );
    assert!(
        freezing(&run) >= before,
        "Fingers must preserve recipient stacks; independent Infused impacts may add more"
    );
}
