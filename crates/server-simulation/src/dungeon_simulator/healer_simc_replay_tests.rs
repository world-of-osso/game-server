//! Class-party SimC replay receipts and native healer mechanics on the class party.
use shared::components::Health;
use std::path::Path;

#[test]
#[ignore = "class-party death-removal audit over the measurement snapshot"]
fn class_party_death_audit() {
    let spec = std::env::var("HEALER_TANK_SPEC").unwrap().parse().unwrap();
    let level = std::env::var("HEALER_TANK_LEVEL").unwrap().parse().unwrap();
    let seed = std::env::var("HEALER_TANK_SEED").unwrap().parse().unwrap();
    let trace = std::env::var("HEALER_TANK_TIMELINE").unwrap();
    let report = super::replay_simc(spec, level, Path::new(&trace), 120, seed).unwrap();
    println!("HEALER_DEATH_AUDIT_REPORT={report}");
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn restoration_grove_guardian_admission_receipt() {
    let mut run = super::snapshot_party(105, 90, 1);
    let world = run.app.world();
    let known = world
        .get::<crate::class_progression::KnownSpells>(run.healer)
        .unwrap();
    assert!(known.contains(1226140), "selected Grove Guardians passive");
    assert!(
        !known.contains(102693),
        "check whether the active summon is learned"
    );
    run.app
        .world_mut()
        .get_mut::<Health>(run.tank)
        .unwrap()
        .current *= 0.5;
    let result = crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.healer,
        102693,
        Some(run.tank),
        false,
    );
    eprintln!("GROVE_CAST_RESULT {result:?}");
    assert!(result.is_err(), "check native cast admission");
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn dummy_pressure_uses_unscaled_same_level_units() {
    for level in [70, 90] {
        let run = super::snapshot_party(105, level, 1);
        let world = run.app.world();
        let source = run.sources[0];
        assert_eq!(
            world
                .get::<shared::components::UnitLevel>(source)
                .unwrap()
                .0,
            level
        );
        assert_eq!(
            world
                .get::<shared::components::UnitLevel>(run.tank)
                .unwrap()
                .0,
            level
        );
        assert!(
            world
                .get::<shared::level_scaling::LevelScaling>(source)
                .is_none()
        );
        assert!(
            world
                .get::<shared::level_scaling::LevelScaling>(run.tank)
                .is_none()
        );
        assert_eq!(
            crate::level_scaling::damage_to_hp_scale(world, source, run.tank),
            1.0
        );
        eprintln!(
            "REPLAY_SCALE level={level} source_damage_to_hp=1 tank_health={}",
            world.get::<Health>(run.tank).unwrap().max
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn death_audit_records_real_aura_removals() {
    let mut run = super::snapshot_party(105, 70, 1);
    let actors = run.slots.to_vec();
    let enemy = run.sources[0];
    super::apply_optimal_raid_environment(&mut run.app, enemy, &actors).unwrap();
    run.app.init_resource::<crate::auras::DeathAuraAudit>();
    run.app
        .world_mut()
        .get_mut::<Health>(run.tank)
        .unwrap()
        .current = 0.0;
    run.app.update();
    let audit = run.app.world().resource::<crate::auras::DeathAuraAudit>();
    assert!(
        audit
            .0
            .iter()
            .any(|(unit, spells)| *unit == run.tank && spells.contains(&1126))
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn optimal_raid_environment_reaches_all_replay_actors() {
    let mut run = super::snapshot_party(256, 90, 1);
    let members = run.slots.to_vec();
    let enemy = run.sources[0];
    super::apply_optimal_raid_environment(&mut run.app, enemy, &members).unwrap();
    let world = run.app.world();
    for member in members {
        for spell in [1459, 1126, 2825] {
            assert!(
                crate::auras::has_aura(world, member, spell),
                "member {member:?} lacks {spell}"
            );
        }
    }
    for spell in [1490, 113746] {
        assert!(
            crate::auras::has_aura(world, run.sources[0], spell),
            "enemy lacks {spell}"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn beacon_transfers_healing_between_party_members() {
    let mut run = super::snapshot_party(65, 90, 1);
    let (tank, healer) = (run.tank, run.healer);
    // A DPS party member.
    let follower = run.slots[2];
    for (beacon, healed) in [(tank, follower), (follower, tank)] {
        for target in [beacon, healed] {
            let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
            health.current = health.max * 0.25;
        }
        crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 53563, Some(beacon), 1);
        let before = run.app.world().get::<Health>(beacon).unwrap().current;
        let healed_before = run.app.world().get::<Health>(healed).unwrap().current;
        crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 19750, Some(healed), 1);
        let after = run.app.world().get::<Health>(beacon).unwrap().current;
        let effective = run.app.world().get::<Health>(healed).unwrap().current - healed_before;
        let transferred = after - before;
        assert!(effective > 0.0);
        assert!(
            (transferred - effective * 0.15).abs() <= 2.0,
            "Beacon {beacon:?} received {transferred} from {effective} healing on {healed:?}"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn preservation_lifebind_aggregates_raw_healing_on_native_party() {
    let mut run = super::snapshot_party(1468, 90, 1);
    let (tank, healer) = (run.tank, run.healer);
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 360995, Some(tank), 1);
    assert!(crate::auras::has_aura(run.app.world(), healer, 373267));
    assert!(crate::auras::has_aura(run.app.world(), tank, 373267));
    for unit in [tank, healer] {
        let mut health = run.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.current = if unit == tank {
            health.max
        } else {
            health.max * 0.25
        };
    }
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..2 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 361509, Some(tank), 1);
    }
    for _ in 0..3 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    assert!(!events.iter().any(|event| event.spell_id == Some(373268)));
    for _ in 0..2 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let raw: i32 = events
        .iter()
        .filter(|event| {
            event.spell_id == Some(361509)
                && event.kind == shared::protocol::CombatLogKind::Heal
                && event.target == Some(tank.to_bits())
        })
        .map(|event| {
            assert_eq!(
                event.amount, event.overflow,
                "source heal must be entirely overheal"
            );
            event.amount
        })
        .sum();
    assert!(raw > 0);
    let transfers: Vec<_> = events
        .iter()
        .filter(|event| {
            event.spell_id == Some(373268)
                && event.kind == shared::protocol::CombatLogKind::Heal
                && event.target == Some(healer.to_bits())
        })
        .collect();
    assert_eq!(
        transfers.len(),
        1,
        "two heals must aggregate into one 200 ms transfer"
    );
    assert!((transfers[0].amount as f32 - raw as f32 * 0.6).abs() <= 2.0);

    let start = events.len() + start;
    for unit in [tank, healer] {
        let mut health = run.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.current = if unit == healer {
            health.max
        } else {
            health.max * 0.25
        };
    }
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 361509, Some(healer), 1);
    for _ in 0..5 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let heal = events
        .iter()
        .find(|event| {
            event.kind == shared::protocol::CombatLogKind::Heal
                && event.spell_id == Some(361509)
                && event.target == Some(healer.to_bits())
        })
        .expect("self-heal must land");
    assert_eq!(heal.amount, heal.overflow);
    let transfer = events
        .iter()
        .find(|event| {
            event.kind == shared::protocol::CombatLogKind::Heal
                && event.spell_id == Some(373268)
                && event.target == Some(tank.to_bits())
        })
        .expect("Lifebind must heal the bonded ally");
    assert!((transfer.amount as f32 - heal.amount as f32 * 0.6).abs() <= 2.0);
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn preservation_lifebind_transfers_periodic_overhealing() {
    let mut run = super::snapshot_party(1468, 90, 1);
    let (tank, healer) = (run.tank, run.healer);
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 360995, Some(tank), 1);
    for unit in [tank, healer] {
        let mut health = run.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.current = if unit == tank {
            health.max
        } else {
            health.max * 0.25
        };
    }
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 366155, Some(tank), 1);
    for _ in 0..45 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let tick = events
        .iter()
        .find(|event| {
            event.kind == shared::protocol::CombatLogKind::Heal
                && event.spell_id == Some(366155)
                && event.periodic
                && event.target == Some(tank.to_bits())
        })
        .expect("Reversion must tick on the bonded ally");
    assert_eq!(
        tick.amount, tick.overflow,
        "source tick must be fully overhealed"
    );
    let transfer = events
        .iter()
        .find(|event| {
            event.kind == shared::protocol::CombatLogKind::Heal
                && event.spell_id == Some(373268)
                && event.target == Some(healer.to_bits())
        })
        .expect("Lifebind must transfer the periodic heal");
    assert!((transfer.amount as f32 - tick.amount as f32 * 0.6).abs() <= 2.0);
}

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/dungeon_simulator/fixtures")
        .join(name)
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn simc_party_pulse_hits_all_five_class_party_members() {
    let report = super::replay_simc_party(
        65,
        90,
        &fixture("healer_simc_fixture.log"),
        &fixture("healer_simc_party_fixture.log"),
        12,
        70448,
    )
    .unwrap();
    assert_eq!(report["party_incoming_events"], 5);
    let party_damage = report["metrics"]["party_damage"].as_object().unwrap();
    assert_eq!(
        party_damage.len(),
        5,
        "every party member must take native damage: {report}"
    );
    // The tank can absorb its hit; the other four must take real health damage.
    assert!(
        party_damage
            .values()
            .filter(|v| v.as_u64().unwrap() > 0)
            .count()
            >= 4,
        "{report}"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot and SimC traces"]
fn simc_class_healer_replays_damage_and_reports_receipts() {
    let trace = fixture("healer_simc_fixture.log");
    assert_eq!(
        super::replay_simc(66, 90, &trace, 12, 70448).unwrap_err(),
        "Simulator party slot2 requiresHealer, spec66 isTank",
        "only a healer spec takes the healer slot"
    );
    let report = super::replay_simc(65, 90, &trace, 12, 70448).unwrap();
    assert_eq!(report["incoming_events"], 3);
    assert_eq!(report["level"], 90);
    assert_eq!(report["actors"], "class_followers");
    let snapshot = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let digest = {
        use sha2::Digest;
        format!(
            "{:x}",
            sha2::Sha256::digest(std::fs::read(snapshot).unwrap())
        )
    };
    assert_eq!(report["world_db_sha256"], digest.as_str());
    let metrics = &report["metrics"];
    let idle = metrics["idle_gcd_pct"].as_f64().unwrap();
    assert!(
        (0.0..100.0).contains(&idle),
        "the class healer must cast during the fixture: {report}"
    );
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(effective > 0.0 && raw >= effective, "{report}");
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 13);
}

#[test]
fn class_replay_refuses_the_canonical_world_database() {
    assert_eq!(
        super::hash_snapshot(Path::new(super::CANONICAL_WORLD_DB)).unwrap_err(),
        "HEALER_REPLAY_REQUIRES_SNAPSHOT_WORLD_DB"
    );
}
