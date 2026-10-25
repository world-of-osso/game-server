//! Native Venomous Wounds must return Energy after earned poisoned bleed ticks.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_poisoned_bleeds_return_native_venomous_energy() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 79134));
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..800 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let damage = |spell| {
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(spell)
                && event.amount > 0
        })
    };
    assert!(
        damage(2818),
        "an earned native poison must really tick before the Energy boundary"
    );
    assert!(
        damage(703) && damage(1943),
        "ordinary Garrote and Rupture must really tick"
    );
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Energize
                && event.spell_id == Some(79134)
                && event.amount > event.overflow
        }),
        "Venomous Wounds must restore uncapped native Energy from the poisoned bleed ticks"
    );
}
