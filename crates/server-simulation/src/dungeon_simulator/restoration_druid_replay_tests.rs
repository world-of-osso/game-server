use std::path::Path;

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
