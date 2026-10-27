//! Native Entropy combat-entry fragment production.
use super::*;
use shared::{components::CombatStatus, protocol::CombatLogKind};

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn selected_entropy_spawns_lesser_fragment_on_combat_entry() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .is_some_and(|known| known.contains(1261684)),
        "selected Entropy talent"
    );
    assert!(
        !run.app
            .world()
            .get::<CombatStatus>(actor)
            .is_some_and(|status| status.0),
        "actor starts outside combat"
    );
    assert_eq!(
        server::soul_fragments::active_count(
            run.app.world(),
            actor,
            server::soul_fragments::FragmentFilter::Lesser
        ),
        0,
        "no prior active fragments"
    );
    engage(&mut run);
    // A real Voidblade child damages the dummy and starts combat; it is not an Entropy fragment producer.
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1245414, Some(target), 1);
    for _ in 0..30 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<CombatStatus>(actor)
            .is_some_and(|status| status.0),
        "real damage entered combat"
    );
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(1245414)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "real damage landed"
    );
    // SimC76f8118 cpp9873–9881,9959–9967 triggers Entropy's first combat tick immediately.
    let initial = server::soul_fragments::active_count(
        run.app.world(),
        actor,
        server::soul_fragments::FragmentFilter::Lesser,
    );
    assert!(
        initial > 0,
        "Entropy creates an active lesser fragment on entering combat"
    );
    for _ in 0..180 {
        run.app.update();
    }
    assert_eq!(
        server::soul_fragments::active_count(
            run.app.world(),
            actor,
            server::soul_fragments::FragmentFilter::Lesser
        ),
        initial,
        "no second Entropy fragment before its native 12-second period"
    );
    for _ in 0..60 {
        run.app.update();
    }
    assert!(
        server::soul_fragments::active_count(
            run.app.world(),
            actor,
            server::soul_fragments::FragmentFilter::Lesser
        ) > initial,
        "Entropy creates another fragment at its native 12-second period"
    );
}
