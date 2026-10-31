//! Class-party SimC replay receipts and native healer mechanics on reference actors.
use bevy::prelude::{Entity, World};
use shared::components::Health;
use std::path::Path;

#[test]
#[ignore = "requires canonical world database and healer-tank profiles"]
fn optimal_raid_environment_reaches_all_replay_actors() {
    let mut run = crate::dungeon_simulator::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    let healer =
        crate::dungeon_simulator::dps_parity::spawn_second_reference_player(&mut run, 256).unwrap();
    let party = super::spawn_reference_party(run.app.world_mut(), tank, healer).unwrap();
    let members: Vec<_> = std::iter::once(tank)
        .chain(party.members.iter().copied())
        .collect();
    let enemy = run.targets[0];
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
            crate::auras::has_aura(world, run.targets[0], spell),
            "enemy lacks {spell}"
        );
    }
}

#[test]
#[ignore = "requires canonical world database and healer-tank profiles"]
fn reference_healer_remains_alive_during_single_faction_setup() {
    let mut run = crate::dungeon_simulator::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    crate::spell_cast::cast_triggered(run.app.world_mut(), tank, 1308399, Some(tank), 1);
    let world = run.app.world_mut();
    assert!(
        world
            .query::<&crate::spell_cast::summon::SummonedCreature>()
            .iter(world)
            .any(|summon| summon.owner == tank && summon.spell_id == 1308399),
        "real tank summon must be present before healer setup"
    );
    let healer =
        crate::dungeon_simulator::dps_parity::spawn_second_reference_player(&mut run, 65).unwrap();
    let world = run.app.world();
    assert_eq!(
        world.get::<shared::components::UnitFactionTemplate>(healer),
        world.get::<shared::components::UnitFactionTemplate>(tank)
    );
    assert!(
        world.get::<Health>(healer).unwrap().current > 0.0,
        "a same-party healer must survive all setup updates"
    );
    let events = &world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        !events
            .iter()
            .any(|event| event.target == Some(healer.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage),
        "a tank summon must not attack the reference healer during setup"
    );
}

#[test]
#[ignore = "requires canonical world database and healer-tank profiles"]
fn reference_replay_follower_companions_wear_replay_profile_gear() {
    let mut run = crate::dungeon_simulator::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    let healer =
        crate::dungeon_simulator::dps_parity::spawn_second_reference_player(&mut run, 65).unwrap();
    let party = super::spawn_reference_party(run.app.world_mut(), tank, healer).unwrap();
    let directory = std::path::PathBuf::from(std::env::var_os("DPS_PARITY_OUTPUT").unwrap());
    let world = run.app.world();
    let companions: Vec<_> = party
        .members
        .iter()
        .filter_map(|&unit| {
            world
                .get::<crate::followers::Follower>(unit)
                .map(|follower| (unit, follower.profile.spec_id))
        })
        .collect();
    assert_eq!(companions.len(), 3, "three follower companions");
    for (unit, spec) in companions {
        let path = directory.join(format!("{spec}.profile.json"));
        let profile: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let expected: Vec<u32> = profile["equipment"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["item_id"].as_u64().unwrap() as u32)
            .collect();
        let equipped: Vec<u32> = world
            .get::<crate::stats::EquippedItems>(unit)
            .expect("replay companion equipment")
            .items
            .iter()
            .map(|(_, _, item)| *item)
            .collect();
        assert!(!expected.is_empty());
        assert_eq!(
            equipped, expected,
            "spec{spec} wears its replay profile gear"
        );
    }
}

#[test]
#[ignore = "requires canonical world database and healer-tank profiles"]
fn beacon_transfers_healing_between_player_and_follower_party_members() {
    let mut run = crate::dungeon_simulator::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    let healer =
        crate::dungeon_simulator::dps_parity::spawn_second_reference_player(&mut run, 65).unwrap();
    let party = crate::dungeon_simulator::healer_simc_replay::spawn_reference_party(
        run.app.world_mut(),
        tank,
        healer,
    )
    .unwrap();
    let follower = party.members[0];
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
#[ignore = "requires canonical world database and Preservation reference profile"]
fn preservation_lifebind_aggregates_raw_healing_on_native_party() {
    let mut run = crate::dungeon_simulator::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    let healer =
        crate::dungeon_simulator::dps_parity::spawn_second_reference_player(&mut run, 1468)
            .unwrap();
    crate::dungeon_simulator::healer_simc_replay::spawn_reference_party(
        run.app.world_mut(),
        tank,
        healer,
    )
    .unwrap();
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
#[ignore = "requires canonical world database and Preservation reference profile"]
fn preservation_lifebind_transfers_periodic_overhealing() {
    let mut run = crate::dungeon_simulator::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    let healer =
        crate::dungeon_simulator::dps_parity::spawn_second_reference_player(&mut run, 1468)
            .unwrap();
    crate::dungeon_simulator::healer_simc_replay::spawn_reference_party(
        run.app.world_mut(),
        tank,
        healer,
    )
    .unwrap();
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
#[ignore = "class party replay requires the FOLLOWER_DPS_WORLD_DB snapshot"]
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
#[ignore = "class party replay requires the FOLLOWER_DPS_WORLD_DB snapshot"]
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
