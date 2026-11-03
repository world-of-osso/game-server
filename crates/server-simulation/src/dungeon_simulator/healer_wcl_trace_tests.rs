use super::healer_wcl_trace::RecordedHit;

#[test]
#[ignore = "requires native Priest and Altar spell data"]
fn wcl_intake_hostile_spell_triggers_prayer_of_mending() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::components::Health;
    use shared::protocol::CombatLogKind;

    let mut h = harness(Vec::new(), &[33076, 41635, 33110, 155793, 1306338], 10).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let holder = spawn_player(&mut h.app, 10, 3.0);
    *h.app.world_mut().get_mut::<Health>(holder).unwrap() = Health {
        current: 50_000.0,
        max: 100_000.0,
    };
    h.app
        .world_mut()
        .entity_mut(holder)
        .insert(crate::level_scaling::CreatureArmor(1_000_000.0));
    h.cast_and_deliver(33076, Some(holder));
    h.advance(3);
    let cursor = h.app.world().resource::<ObservedCombatEvents>().0.len();
    let hit: RecordedHit = serde_json::from_str(r#"{"time_ms":0,"slot":0,"source_actor":413,"target_actor":6,"spell_id":1306338,"school":1,"defense":2,"hit_type":1,"periodic":false,"max_hp":100000,"amount":1000,"absorbed":0}"#).unwrap();
    assert!(matches!(
        hit.deliver(h.app.world_mut(), h.npc, &[holder; 5]).unwrap(),
        super::healer_wcl_trace::Delivery::Applied(_)
    ));
    assert_eq!(
        h.app.world().get::<Health>(holder).unwrap().current,
        49_000.0
    );
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

#[test]
#[ignore = "requires native Altar spell data"]
fn wcl_intake_normalizes_health_fraction_before_native_absorb() {
    use crate::spell_cast::test_harness::{harness, spawn_player};
    use shared::components::Health;

    let mut h = harness(Vec::new(), &[1306338], 10).unwrap();
    let holder = spawn_player(&mut h.app, 10, 3.0);
    *h.app.world_mut().get_mut::<Health>(holder).unwrap() = Health {
        current: 200_000.0,
        max: 200_000.0,
    };
    crate::auras::source_absorb::grant(h.app.world_mut(), holder, h.npc, 17, 500.0, 30_000);
    let hit: RecordedHit = serde_json::from_str(r#"{"time_ms":0,"slot":0,"source_actor":413,"target_actor":6,"spell_id":1306338,"school":1,"defense":2,"hit_type":1,"periodic":false,"max_hp":100000,"amount":1000,"absorbed":0}"#).unwrap();
    assert!(matches!(
        hit.deliver(h.app.world_mut(), h.npc, &[holder; 5]).unwrap(),
        super::healer_wcl_trace::Delivery::Applied(_)
    ));
    assert_eq!(
        h.app.world().get::<Health>(holder).unwrap().current,
        198_500.0
    );
}
