//! Special Delivery launches from a real defensive Brew and lands after its native flight.
use super::*;
use shared::protocol::CombatLogKind;

fn damage_events(run: &Run, target: Entity) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.spell_id == Some(196733)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(target.to_bits())
                && event.amount > 0
        })
        .count()
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn defensive_brew_launches_special_delivery_after_missile_delay() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 115203, Some(actor), false)
        .unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    assert_eq!(
        damage_events(&run, target),
        0,
        "the projectile cannot land early"
    );
    for _ in 0..300 {
        run.app.update();
    }
    assert_eq!(
        damage_events(&run, target),
        1,
        "the real Brew projectile reaches its enemy"
    );
}
