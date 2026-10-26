//! SimC76f8118 sc_priest_shadow.cpp281–288/542–547/2468.
//! Every actual Void apparition launch emits native Void Bolt, not Void Spike.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_void_bolt_real_slam_void_apparitions_emit_one_native_bolt_at_primary() {
    let mut run = prepare_literal(258, 3).unwrap();
    engage_fixture_opponents(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    for _ in 0..5 {
        literal_rotation::execution::execute(
            run.app.world_mut(),
            actor,
            literal_rotation::execution::Execution::Spell(1227280, primary, None),
        )
        .expect("ordinary native Slam");
        for _ in 0..900 {
            run.app.update();
        }
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let damage = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .collect::<Vec<_>>();
    let voids = damage
        .iter()
        .filter(|event| event.spell_id == Some(1264176))
        .count();
    assert!(voids > 0, "ordinary Slam actually earns Void apparitions");
    let bolts = damage
        .iter()
        .filter(|event| event.spell_id == Some(1264177))
        .collect::<Vec<_>>();
    assert_eq!(
        bolts.len(),
        voids,
        "one real native Void Bolt per settled Void apparition"
    );
    assert!(
        bolts
            .iter()
            .all(|event| event.target == Some(primary.to_bits())),
        "every Void Bolt retains the actual source primary"
    );
    assert!(
        !damage.iter().any(|event| event.spell_id == Some(373279)),
        "Void apparition launch cannot substitute unrelated area Void Spike"
    );
}
