//! Party-pulse damage reaches ordinary hostile-hit proc consumers.

#[test]
#[ignore = "requires native Priest spell data"]
fn party_pulse_triggers_prayer_of_mending_on_its_actual_holder() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::components::Health;
    use shared::protocol::CombatLogKind;

    let mut h = harness(Vec::new(), &[33076, 41635, 33110, 155793], 10).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let holder = spawn_player(&mut h.app, 10, 3.0);
    *h.app.world_mut().get_mut::<Health>(holder).unwrap() = Health {
        current: 50_000.0,
        max: 100_000.0,
    };
    h.cast_and_deliver(33076, Some(holder));
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), holder, 41635));
    let cursor = h.app.world().resource::<ObservedCombatEvents>().0.len();
    super::healer_simc_replay::deliver_party_hit(h.app.world_mut(), h.npc, holder, 1_000.0);
    h.advance(8);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0[cursor..];
    assert!(
        events
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.target == Some(holder.to_bits())
                && event.amount > 0),
        "the pulse must deal real damage"
    );
    assert!(
        events.iter().any(|event| event.kind == CombatLogKind::Heal
            && event.spell_id == Some(33110)
            && event.source == Some(h.caster.to_bits())
            && event.target == Some(holder.to_bits())
            && event.amount > 0),
        "the damage must trigger the actual ward heal: {events:?}"
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), holder, 41635),
        "the proc must consume the holder's ward"
    );
}
