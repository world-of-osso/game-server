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
#[ignore = "requires native spell data"]
fn wcl_intake_avoided_outcomes_publish_distinct_miss_events() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness};
    use shared::components::Health;
    use shared::protocol::{CombatLogKind, MissKind};
    let mut h = harness(Vec::new(), &[], 10).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let victim = h.caster;
    let initial = h.app.world().get::<Health>(victim).unwrap().current;
    for (hit_type, expected) in [
        (0, MissKind::Miss),
        (7, MissKind::Dodge),
        (8, MissKind::Parry),
        (9, MissKind::Deflect),
        (10, MissKind::Immune),
    ] {
        let text = format!(
            r#"{{"time_ms":0,"slot":0,"source_actor":413,"target_actor":6,"spell_id":1,"school":1,"defense":2,"hit_type":{hit_type},"periodic":false,"max_hp":100000,"amount":0,"absorbed":0}}"#
        );
        let hit: RecordedHit = serde_json::from_str(&text).unwrap();
        assert!(matches!(
            hit.deliver(h.app.world_mut(), h.npc, &[victim; 5]).unwrap(),
            super::healer_wcl_trace::Delivery::Avoided
        ));
        assert!(
            h.app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| event.kind == CombatLogKind::Miss(expected)
                    && event.target == Some(victim.to_bits()))
        );
    }
    assert_eq!(
        h.app.world().get::<Health>(victim).unwrap().current,
        initial
    );
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn wcl_intake_does_not_stagger_already_mitigated_damage() {
    use shared::components::Health;
    let mut party =
        super::healer_simc_replay::snapshot_party_with_specs(1468, 90, [268, 1468, 62, 262, 262]);
    let tank = party.tank;
    let enemy = party.sources[0];
    let before = *party.app.world().get::<Health>(tank).unwrap();
    let pool = crate::auras::stagger::pool_size(party.app.world(), tank);
    let hit: RecordedHit = serde_json::from_str(r#"{"time_ms":0,"slot":0,"source_actor":413,"target_actor":6,"spell_id":1306338,"school":1,"defense":2,"hit_type":1,"periodic":false,"max_hp":100000,"amount":1000,"absorbed":0}"#).unwrap();
    assert!(matches!(
        hit.deliver(party.app.world_mut(), enemy, &[tank; 5])
            .unwrap(),
        super::healer_wcl_trace::Delivery::Applied(_)
    ));
    assert_eq!(
        crate::auras::stagger::pool_size(party.app.world(), tank),
        pool
    );
    let after = party.app.world().get::<Health>(tank).unwrap().current;
    assert!((before.current - after - before.max * 0.01).abs() <= 2.0);
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn wcl_intake_dodge_consumes_elusive_brawler_but_miss_does_not() {
    use crate::auras::{self, AuraApplication, AuraSource};
    let _seed = crate::random::seed_scope(70448);
    let mut run =
        super::healer_simc_replay::snapshot_party_with_specs(1468, 90, [268, 1468, 62, 262, 262]);
    let tank = run.tank;
    let source = run.sources[0];
    for _ in 0..3 {
        auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: tank,
                target: tank,
                spell_id: 195630,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    for (hit_type, expected) in [(0, true), (7, false)] {
        let text = format!(
            r#"{{"time_ms":0,"slot":0,"source_actor":413,"target_actor":6,"spell_id":1,"school":1,"defense":2,"hit_type":{hit_type},"periodic":false,"max_hp":100000,"amount":0,"absorbed":0}}"#
        );
        let hit: RecordedHit = serde_json::from_str(&text).unwrap();
        assert!(matches!(
            hit.deliver(run.app.world_mut(), source, &[tank; 5])
                .unwrap(),
            super::healer_wcl_trace::Delivery::Avoided
        ));
        run.app.update();
        assert_eq!(auras::has_aura(run.app.world(), tank, 195630), expected);
    }
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
