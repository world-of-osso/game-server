use super::healer_wcl_trace::RecordedHit;

#[test]
#[ignore = "requires native Ghost Wolf spell data"]
fn opening_wcl_form_is_removed_at_recorded_time() {
    use crate::spell_cast::test_harness::harness;
    let mut h = harness(Vec::new(), &[2645], 7).unwrap();
    let actor = h.caster;
    let trace: super::healer_wcl_trace::Trace = serde_json::from_str(
        r#"{"provenance":"test","excluded_self":0,"party_specs":[264,264,264,264,264],
        "actor_slots":{"46":0},"bloodlust":[],"hits":[],
        "opening_auras":[{"source_actor":46,"target_actor":46,"spell_id":2645,
        "stacks":1,"evidence":"first_removebuff","remove_ms":2535}]}"#,
    )
    .unwrap();
    let removals = trace
        .apply_opening_auras(h.app.world_mut(), &[actor; 5])
        .unwrap();
    let mut next = 0;
    super::healer_wcl_trace::remove_due_opening_auras(
        h.app.world_mut(),
        &removals,
        &mut next,
        2534,
    );
    assert!(crate::auras::has_aura(h.app.world(), actor, 2645));
    super::healer_wcl_trace::remove_due_opening_auras(
        h.app.world_mut(),
        &removals,
        &mut next,
        2535,
    );
    assert!(!crate::auras::has_aura(h.app.world(), actor, 2645));
}

#[test]
#[ignore = "requires native Ghost Wolf spell data"]
fn recorded_removal_preserves_a_natively_refreshed_opening_aura() {
    use crate::auras::{AuraApplication, AuraSource};
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness};
    let mut h = harness(Vec::new(), &[2645], 7).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let actor = h.caster;
    let trace: super::healer_wcl_trace::Trace = serde_json::from_str(
        r#"{"provenance":"test","excluded_self":0,"party_specs":[264,264,264,264,264],
        "actor_slots":{"46":0},"bloodlust":[],"hits":[],
        "opening_auras":[{"source_actor":46,"target_actor":46,"spell_id":2645,
        "stacks":1,"evidence":"first_removebuff","remove_ms":2535}]}"#,
    )
    .unwrap();
    let mut removals = trace
        .apply_opening_auras(h.app.world_mut(), &[actor; 5])
        .unwrap();
    h.advance(1);
    let cursor = h.app.world().resource::<ObservedCombatEvents>().0.len();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 2645,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    h.advance(1);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0[cursor..];
    assert!(
        events
            .iter()
            .any(|event| event.kind == shared::protocol::CombatLogKind::AuraRefreshed)
    );
    super::healer_wcl_trace::preserve_refreshed_opening_auras(&mut removals, events);
    let mut next = 0;
    super::healer_wcl_trace::remove_due_opening_auras(
        h.app.world_mut(),
        &removals,
        &mut next,
        2535,
    );
    assert!(crate::auras::has_aura(h.app.world(), actor, 2645));
}

#[test]
#[ignore = "requires native Merithra spell data"]
fn opening_wcl_aura_resolves_reversion_to_merithra() {
    use super::dps_parity::literal_rotation::execution::resolve_action;
    use crate::spell_cast::test_harness::harness;
    let mut h = harness(Vec::new(), &[1256579, 1256581, 366155], 13).unwrap();
    let actor = h.caster;
    let trace: super::healer_wcl_trace::Trace = serde_json::from_str(
        r#"{
        "provenance":"test", "excluded_self":0, "party_specs":[1468,1468,1468,1468,1468],
        "actor_slots":{"2":0}, "bloodlust":[], "hits":[],
        "opening_auras":[{"source_actor":2,"target_actor":2,"spell_id":1256579,
        "stacks":1,"evidence":"first_removebuff"}]
    }"#,
    )
    .unwrap();
    trace
        .apply_opening_auras(h.app.world_mut(), &[actor; 5])
        .unwrap();
    assert!(crate::auras::has_aura(h.app.world(), actor, 1256579));
    assert_eq!(
        resolve_action(h.app.world(), actor, 366155).unwrap(),
        Some(1256581)
    );
}

#[test]
#[ignore = "requires native spell data"]
fn wcl_critical_hit_replays_recorded_damage_without_critical_multiplier() {
    use crate::spell_cast::test_harness::harness;
    use shared::components::Health;

    let mut h = harness(Vec::new(), &[], 10).unwrap();
    let victim = h.caster;
    let initial = h.app.world().get::<Health>(victim).unwrap().current;
    let hit: RecordedHit = serde_json::from_str(r#"{"time_ms":0,"slot":0,"source_actor":-1,"target_actor":6,"spell_id":3,"school":1,"defense":null,"hit_type":2,"periodic":false,"max_hp":1000,"amount":100,"absorbed":0}"#).unwrap();
    assert!(matches!(
        hit.deliver(h.app.world_mut(), h.npc, &[victim; 5]).unwrap(),
        super::healer_wcl_trace::Delivery::Applied(_)
    ));
    let health = h.app.world().get::<Health>(victim).unwrap();
    assert!((health.current - (initial - health.max * 0.1)).abs() < 0.01);
}

#[test]
#[ignore = "requires native spell data"]
fn wcl_falling_replays_as_physical_pressure_without_db2_spell_three() {
    use crate::spell_cast::test_harness::harness;
    use shared::components::Health;

    let mut h = harness(Vec::new(), &[], 10).unwrap();
    let victim = h.caster;
    let initial = h.app.world().get::<Health>(victim).unwrap().current;
    let hit: RecordedHit = serde_json::from_str(r#"{"time_ms":0,"slot":0,"source_actor":-1,"target_actor":6,"spell_id":3,"school":1,"defense":null,"hit_type":1,"periodic":false,"max_hp":1000,"amount":100,"absorbed":0}"#).unwrap();
    assert!(matches!(
        hit.deliver(h.app.world_mut(), h.npc, &[victim; 5]).unwrap(),
        super::healer_wcl_trace::Delivery::Applied(_)
    ));
    let health = h.app.world().get::<Health>(victim).unwrap();
    assert!((health.current - (initial - health.max * 0.1)).abs() < 0.01);
}

#[test]
#[ignore = "requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn wcl_intake_triggers_selected_aessinas_renewal_from_recorded_large_hit() {
    use shared::components::Health;
    use shared::protocol::CombatLogKind;
    let mut party =
        super::healer_simc_replay::snapshot_party_with_specs(105, 90, [66, 105, 70, 1480, 262]);
    let healer = party.healer;
    super::healer_simc_replay::hold(&mut party, healer);
    let enemy = party.sources[0];
    assert!(
        crate::auras::has_aura(party.app.world(), healer, 474678),
        "selected Aessina aura must be present"
    );
    let hp = party.app.world().get::<Health>(healer).unwrap().max;
    party
        .app
        .world_mut()
        .get_mut::<Health>(healer)
        .unwrap()
        .current = hp * 0.7;
    party
        .app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    let hit: RecordedHit = serde_json::from_str(r#"{"time_ms":9771,"slot":1,"source_actor":1034,"target_actor":4,"spell_id":1306381,"school":8,"defense":1,"hit_type":1,"periodic":false,"max_hp":877300,"amount":239678,"absorbed":0}"#).unwrap();
    assert!(matches!(
        hit.deliver(party.app.world_mut(), enemy, &party.slots)
            .unwrap(),
        super::healer_wcl_trace::Delivery::Applied(_)
    ));
    for _ in 0..5 {
        party.app.update();
    }
    let events = &party
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| event.kind == CombatLogKind::Heal
            && event.spell_id == Some(474683)
            && event.target == Some(healer.to_bits())
            && event.amount > 0),
        "Aessina self-heal missing: {events:?}"
    );
}

#[test]
#[ignore = "requires native Priest and Altar spell data"]
fn wcl_intake_counts_guardian_spirit_absorb_separately_from_rescue_heal() {
    use crate::spell_cast::test_harness::harness;
    use shared::components::Health;

    let mut h = harness(Vec::new(), &[47788, 48153, 1306890], 10).unwrap();
    h.cast_and_deliver(47788, Some(h.caster));
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 47788));
    *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
        current: 10000.0,
        max: 100000.0,
    };
    let hit: RecordedHit = serde_json::from_str(r#"{"time_ms":0,"slot":0,"source_actor":-1,"target_actor":4,"spell_id":1306890,"school":8,"defense":1,"hit_type":1,"periodic":false,"max_hp":100000,"amount":20000,"absorbed":0}"#).unwrap();
    let super::healer_wcl_trace::Delivery::Applied(sample) = hit
        .deliver(h.app.world_mut(), h.npc, &[h.caster; 5])
        .unwrap()
    else {
        panic!("hit not applied")
    };
    assert_eq!(sample["native_loss_pct"], 0.0);
    assert!((sample["native_absorbed_pct"].as_f64().unwrap() - 20.0).abs() < 0.001);
    assert!(sample["native_hp_pct"].as_f64().unwrap() > 10.0);
}

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

#[test]
#[ignore = "requires native Vampiric Blood spell data"]
fn replayed_vampiric_blood_is_cast_only_at_its_logged_time() {
    use super::healer_wcl_trace::{cast_due_replayed_casts, hold_replayed_casts};
    use crate::spell_cast::test_harness::harness;
    use server_follower_apl::literal_rotation::execution::{Execution, execute};
    let mut h = harness(Vec::new(), &[55233], 6).unwrap();
    let actor = h.caster;
    let slots = [actor; 5];
    let trace: super::healer_wcl_trace::Trace = serde_json::from_str(
        r#"{"provenance":"test","excluded_self":0,"party_specs":[250,250,250,250,250],
        "actor_slots":{"4":0},"bloodlust":[],"hits":[],"opening_auras":[],
        "replayed_casts":[{"time_ms":4323,"slot":0,"spell_id":55233,"evidence":"applybuff"}]}"#,
    )
    .unwrap();
    hold_replayed_casts(h.app.world_mut(), &slots, &trace.replayed_casts);
    assert!(
        execute(
            h.app.world_mut(),
            actor,
            Execution::Spell(55233, actor, None)
        )
        .is_err()
    );
    let mut next = 0;
    cast_due_replayed_casts(
        h.app.world_mut(),
        &slots,
        &trace.replayed_casts,
        &mut next,
        4250,
    )
    .unwrap();
    assert!(!crate::auras::has_aura(h.app.world(), actor, 55233));
    cast_due_replayed_casts(
        h.app.world_mut(),
        &slots,
        &trace.replayed_casts,
        &mut next,
        4300,
    )
    .unwrap();
    assert!(crate::auras::has_aura(h.app.world(), actor, 55233));
    assert!(
        execute(
            h.app.world_mut(),
            actor,
            Execution::Spell(55233, actor, None)
        )
        .is_err()
    );
}
