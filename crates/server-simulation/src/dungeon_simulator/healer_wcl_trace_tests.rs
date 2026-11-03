use super::healer_wcl_trace::RecordedHit;

#[test]
#[ignore = "requires native Priest and Altar spell data"]
fn wcl_hostile_spell_triggers_prayer_of_mending() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::components::Health;
    use shared::protocol::CombatLogKind;

    let mut h = harness(Vec::new(), &[33076, 41635, 33110, 155793, 1306890], 10).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let holder = spawn_player(&mut h.app, 10, 3.0);
    *h.app.world_mut().get_mut::<Health>(holder).unwrap() = Health {
        current: 50_000.0,
        max: 100_000.0,
    };
    h.cast_and_deliver(33076, Some(holder));
    h.advance(3);
    let cursor = h.app.world().resource::<ObservedCombatEvents>().0.len();
    let hit: RecordedHit = serde_json::from_str(r#"{"time_ms":0,"slot":0,"source_actor":-1,"target_actor":6,"spell_id":1306890,"school":8,"defense":1,"hit_type":1,"periodic":false,"unmitigated":1000,"amount":1000,"absorbed":0}"#).unwrap();
    assert!(matches!(
        hit.deliver(h.app.world_mut(), h.npc, &[holder; 5], 1)
            .unwrap(),
        super::healer_wcl_trace::Delivery::Applied
    ));
    h.advance(8);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0[cursor..];
    assert!(
        events.iter().any(|event| event.kind == CombatLogKind::Heal
            && event.spell_id == Some(33110)
            && event.target == Some(holder.to_bits())
            && event.amount > 0),
        "{events:?}"
    );
}
