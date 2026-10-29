use std::path::Path;

#[test]
#[ignore = "requires native Restoration Druid spell data"]
fn master_shapeshifter_restores_mana_from_actual_wrath() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness};
    use shared::components::Mana;
    use shared::protocol::CombatLogKind;
    let mut h = harness(Vec::new(), &[5176, 289237, 411146], 11).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    *h.app.world_mut().get_mut::<Mana>(h.caster).unwrap() = Mana {
        current: 5000.0,
        max: 10_000.0,
    };
    h.cast(5176, Some(h.npc));
    h.advance(100);
    assert!(
        !h.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|e| e.spell_id == Some(411146) && e.kind == CombatLogKind::Energize)
    );
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 289237,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    h.cast(5176, Some(h.npc));
    h.advance(100);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events.iter().any(|e| e.spell_id == Some(411146)
            && e.kind == CombatLogKind::Energize
            && e.target == Some(h.caster.to_bits())
            && e.amount > 0),
        "actual Wrath must restore native scaled mana: {events:?}"
    );
}

#[test]
#[ignore = "requires top-character profiles, native DB and SimC party pulse"]
fn restoration_druid_uses_selected_germination_in_party_pulse() {
    let trace = std::env::var("HEALER_TANK_TIMELINE").unwrap();
    let party = std::env::var("HEALER_TANK_PARTY_TRACE").unwrap();
    let report = super::super::healer_replay::replay_simc_party(
        105,
        Path::new(&trace),
        Path::new(&party),
        300,
        70448,
    )
    .unwrap();
    assert!(
        report["metrics"]["spell_healing"]["155777"]["raw"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "selected Germination must heal: {report}"
    );
    println!("DRUID_POLICY_REPORT={report}");
}

#[test]
#[ignore = "requires native Restoration Druid spell data"]
fn swiftmend_triggers_three_blooms_on_owned_lifebloom() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::protocol::CombatLogKind;
    let mut h = harness(Vec::new(), &[774, 18562, 33763, 33778, 1244470], 11).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let lifebloom_target = spawn_player(&mut h.app, 11, 3.0);
    let swiftmend_target = spawn_player(&mut h.app, 11, 5.0);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1244470,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        h.caster,
        33763,
        Some(lifebloom_target),
        true,
    )
    .unwrap();
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        h.caster,
        774,
        Some(swiftmend_target),
        true,
    )
    .unwrap();
    h.advance(2);
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        h.caster,
        18562,
        Some(swiftmend_target),
        true,
    )
    .unwrap();
    h.advance(30);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    let blooms: Vec<_> = events
        .iter()
        .filter(|e| {
            e.kind == CombatLogKind::Heal
                && e.spell_id == Some(33778)
                && e.target == Some(lifebloom_target.to_bits())
        })
        .collect();
    assert_eq!(
        blooms.len(),
        3,
        "Swiftmend must trigger three Lifebloom blooms: {events:?}"
    );
    assert!(
        crate::auras::has_aura(h.app.world(), lifebloom_target, 33763),
        "frenzy must not consume Lifebloom"
    );
}

#[test]
#[ignore = "requires native Restoration Druid spell data"]
fn wild_growth_grows_one_selected_talent_bloom() {
    use crate::spell_cast::test_harness::{harness, spawn_player};
    use shared::components::Health;
    let mut h = harness(Vec::new(), &[48438, 440118, 439530], 11).unwrap();
    let first = spawn_player(&mut h.app, 11, 3.0);
    let second = spawn_player(&mut h.app, 11, 5.0);
    for unit in [first, second] {
        *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
            current: 1.0,
            max: 100_000.0,
        };
    }
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 440118,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 48438, Some(first), true)
        .unwrap();
    h.advance(2);
    let count = [h.caster, first, second]
        .into_iter()
        .filter(|unit| crate::auras::has_aura(h.app.world(), *unit, 439530))
        .count();
    assert_eq!(
        count, 1,
        "Wild Growth must grow exactly one selected-talent bloom"
    );
}

#[test]
#[ignore = "requires native Restoration Druid spell data"]
fn swiftmend_grows_the_selected_talent_bloom() {
    use crate::spell_cast::test_harness::{harness, spawn_player};
    let mut h = harness(Vec::new(), &[774, 18562, 440118, 439530], 11).unwrap();
    let ally = spawn_player(&mut h.app, 11, 3.0);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 440118,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 774, Some(ally), true).unwrap();
    h.advance(2);
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 18562, Some(ally), true).unwrap();
    h.advance(2);
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(ally)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 439530 && aura.caster == h.caster)
        .expect("Swiftmend must grow Symbiotic Blooms");
    assert_eq!(aura.duration_ms, 6000);
}

#[test]
#[ignore = "requires native Restoration Druid spell data"]
fn regrowth_can_grow_symbiotic_blooms_and_heal() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::components::Health;
    use shared::protocol::CombatLogKind;
    let mut h = harness(Vec::new(), &[8936, 439528, 439530], 11).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let ally = spawn_player(&mut h.app, 11, 3.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 1.0,
        max: 1_000_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 439528,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for _ in 0..500 {
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 8936, Some(ally), true)
            .unwrap();
        h.advance(2);
        if crate::auras::has_aura(h.app.world(), ally, 439530) {
            h.advance(100);
            assert!(
                h.app
                    .world()
                    .resource::<ObservedCombatEvents>()
                    .0
                    .iter()
                    .any(|e| e.kind == CombatLogKind::Heal
                        && e.spell_id == Some(439530)
                        && e.target == Some(ally.to_bits())
                        && e.amount > 0)
            );
            return;
        }
    }
    panic!("Symbiotic Blooms did not proc in 500 Regrowth casts");
}

#[test]
#[ignore = "requires native Restoration Druid spell data"]
fn everbloom_copies_an_expiring_lifebloom_to_nearby_allies() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::components::Health;
    use shared::protocol::CombatLogKind;
    let mut h = harness(Vec::new(), &[33763, 33778, 1244331, 1244341], 11).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let primary = spawn_player(&mut h.app, 11, 3.0);
    let ally = spawn_player(&mut h.app, 11, 5.0);
    let far = spawn_player(&mut h.app, 11, 100.0);
    for unit in [primary, ally, far] {
        *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
            current: 1.0,
            max: 100_000.0,
        };
    }
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1244331,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 33763, Some(primary), true)
        .unwrap();
    h.advance(2);
    crate::auras::remove_auras(
        h.app.world_mut(),
        primary,
        crate::auras::RemoveReason::Expired,
        |a| a.spell_id == 33763,
    );
    h.advance(2);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    let bloom = events
        .iter()
        .find(|e| {
            e.kind == CombatLogKind::Heal
                && e.spell_id == Some(33778)
                && e.target == Some(primary.to_bits())
        })
        .expect("Lifebloom must bloom on expiry");
    for unit in [primary, ally] {
        let copy = events
            .iter()
            .find(|e| {
                e.kind == CombatLogKind::Heal
                    && e.spell_id == Some(1244341)
                    && e.target == Some(unit.to_bits())
            })
            .expect("Everbloom must heal nearby ally");
        assert!((copy.amount as f32 - bloom.amount as f32 * 0.24).abs() <= 1.0);
    }
    assert!(
        !events
            .iter()
            .any(|e| e.spell_id == Some(1244341) && e.target == Some(far.to_bits()))
    );
}

#[test]
#[ignore = "replays the MID2 Restoration Druid against the SimC dummy trace"]
fn restoration_druid_heals_the_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::super::healer_replay::replay_simc(105, Path::new(&trace), 300, 70448).unwrap();
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(effective > 0.0 && raw >= effective, "{report}");
    assert!(!metrics["spell_healing"].as_object().unwrap().is_empty());
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    let casts = report["healer_casts"].as_object().unwrap();
    assert!(
        casts.get("18562").and_then(|v| v.as_u64()).unwrap_or(0) > 0,
        "Swiftmend must be used: {report}"
    );
    assert!(
        casts.get("8921").and_then(|v| v.as_u64()).unwrap_or(0) > 0,
        "Moonfire must be used: {report}"
    );
    assert!(
        casts.get("5176").and_then(|v| v.as_u64()).unwrap_or(0) > 0,
        "Wrath must be used: {report}"
    );
    println!("RESTORATION_DRUID_REPORT={report}");
}
