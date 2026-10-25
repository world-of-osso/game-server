//! Native Rogue cast relationships; no injected auto-attack or combat payloads.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_garrote_starts_real_native_weapon_swings() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(259, 1).unwrap();
    engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<crate::combat::AutoAttack>(run.actor)
            .is_none()
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(703, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..120 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id == Some(703)
        }),
        "ordinary Garrote must really complete before the missing melee boundary"
    );
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
                && event.amount > 0
        }),
        "Garrote's native melee-combat-start flag must produce actual weapon damage"
    );
}
