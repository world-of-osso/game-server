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
fn grove_guardians_heal_the_class_followers_target() {
    use shared::protocol::CombatLogKind;

    for source in [18562, 48438] {
        let mut run = super::snapshot_party(105, 90, 1);
        let healer = run.healer;
        super::hold(&mut run, healer);
        for _ in 0..5 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), run.healer, 1226140));
        run.app
            .world_mut()
            .get_mut::<Health>(run.tank)
            .unwrap()
            .current *= 0.25;
        if source == 18562 {
            crate::spell_cast::cast_triggered(
                run.app.world_mut(),
                run.healer,
                774,
                Some(run.tank),
                1,
            );
        }
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.healer,
            source,
            Some(run.tank),
            false,
        )
        .expect("cast selected Restoration spell");
        let mut treant = None;
        let mut healed = false;
        for _ in 0..160 {
            run.app.update();
            let world = run.app.world_mut();
            if treant.is_none() {
                treant = world
                    .query::<(
                        bevy::prelude::Entity,
                        &crate::spell_cast::summon::SummonedCreature,
                    )>()
                    .iter(world)
                    .find_map(|(actor, summon)| {
                        (summon.owner == run.healer && summon.spell_id == 102693).then_some(actor)
                    });
            }
            if let Some(actor) = treant {
                healed = world
                    .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                    .0
                    .iter()
                    .any(|event| {
                        event.source == Some(actor.to_bits())
                            && event.target == Some(run.tank.to_bits())
                            && event.kind == CombatLogKind::Heal
                            && event.spell_id == Some(422090)
                            && event.amount > 0
                    });
                if healed {
                    break;
                }
            }
        }
        assert!(treant.is_some(), "spell {source} must summon a treant");
        assert!(
            healed,
            "spell {source}'s treant must heal the original target"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn grove_nourish_applies_the_owners_harmony() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::protocol::CombatLogKind;

    let mut run = super::snapshot_party(105, 90, 1);
    let healer = run.healer;
    super::hold(&mut run, healer);
    for _ in 0..5 {
        run.app.update();
    }
    let stats = run
        .app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(healer)
        .unwrap();
    let mastery = stats.mastery_pct;
    assert!(mastery > 0.0);
    run.app
        .world_mut()
        .get_mut::<Health>(run.tank)
        .unwrap()
        .current *= 0.1;
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 102693, Some(run.tank), 1);
    let world = run.app.world_mut();
    let treant = world
        .query::<(bevy::prelude::Entity, &SummonedCreature)>()
        .iter(world)
        .find_map(|(actor, summon)| {
            (summon.owner == healer && summon.spell_id == 102693).then_some(actor)
        })
        .expect("Grove Guardian summoned");
    crate::spell_cast::cast_triggered(world, treant, 422090, Some(run.tank), 1);
    crate::spell_cast::cast_triggered(world, healer, 774, Some(run.tank), 1);
    crate::spell_cast::cast_triggered(world, treant, 422090, Some(run.tank), 1);
    let heals: Vec<_> = world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(treant.to_bits())
                && event.target == Some(run.tank.to_bits())
                && event.kind == CombatLogKind::Heal
                && event.spell_id == Some(422090)
        })
        .collect();
    assert_eq!(heals.len(), 2, "both Nourish casts must heal the target");
    let normalized = |event: &shared::protocol::CombatLogEvent| {
        event.amount as f32 / if event.crit { 2.0 } else { 1.0 }
    };
    let ratio = normalized(heals[1]) / normalized(heals[0]);
    let expected = 1.0 + mastery / 100.0;
    assert!(
        (ratio - expected).abs() < 0.02,
        "one owned HoT: {ratio} vs {expected}"
    );
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
    super::settle_party_buffs(&mut run).unwrap();
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
        assert!(
            crate::auras::has_aura(world, member, 2825),
            "member {member:?} lacks Bloodlust"
        );
    }
    for spell in [1490, 113746] {
        assert!(
            crate::auras::has_aura(world, run.sources[0], spell),
            "enemy lacks {spell}"
        );
    }
}

/// Party 66/256/262/63/253 provides Power Word: Fortitude (Discipline), Skyfury
/// (Elemental) and Arcane Intellect (Fire): every slot holds exactly those three.
#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn replay_party_holds_exactly_the_buffs_its_composition_casts() {
    use server_follower_apl::party_buffs::{PARTY_BUFFS, holds_buff};
    let mut run = super::snapshot_party(256, 90, 1);
    super::settle_party_buffs(&mut run).unwrap();
    let world = run.app.world();
    for unit in run.slots {
        let held: Vec<u32> = PARTY_BUFFS
            .into_iter()
            .map(|(spell, _)| spell)
            .filter(|&spell| holds_buff(world, unit, spell).unwrap())
            .collect();
        assert_eq!(held, [1459, 21562, 462854], "slot {unit:?}");
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
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn simc_party_pulse_replays_two_engaged_damageable_dummies() {
    let report = super::replay_simc_party_with_targets(
        256,
        90,
        &fixture("healer_simc_fixture.log"),
        &fixture("healer_simc_party_fixture.log"),
        12,
        70448,
        2,
    )
    .unwrap();
    assert_eq!(report["target_count"], 2);
    assert_eq!(report["party_incoming_events"], 5);
    let targets = report["damage_by_dummy"].as_object().unwrap();
    assert_eq!(
        targets.len(),
        2,
        "both engaged dummies must take damage: {report}"
    );
    assert!(targets.values().all(|value| value.as_u64().unwrap() > 0));
    assert!(report["pain_target_seconds"].as_f64().unwrap() > 0.0);
    assert!(report["atonement_target_seconds"].as_f64().unwrap() > 0.0);
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn simc_enemy_profile_changes_engaged_enemy_count() {
    let report = super::replay_simc_party_with_profile(
        270,
        90,
        &fixture("healer_simc_fixture.log"),
        &fixture("healer_simc_party_fixture.log"),
        5,
        70448,
        &fixture("healer_enemy_profile_test.json"),
    )
    .unwrap();
    let samples = report["enemy_profile"]["samples"].as_array().unwrap();
    let counts: Vec<_> = samples
        .iter()
        .map(|s| {
            (
                s["time_ms"].as_u64().unwrap(),
                s["observed"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(counts, [(0, 1), (1000, 2), (2000, 3), (3000, 2), (4000, 1)]);
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn simc_enemy_profile_retires_targets_during_active_casts() {
    let report = super::replay_simc_party_with_profile(
        270,
        90,
        &fixture("healer_simc_fixture.log"),
        &fixture("healer_simc_party_fixture.log"),
        35,
        1,
        &fixture("altar_fangs_wcl_pull1.json"),
    )
    .unwrap();
    let samples = report["enemy_profile"]["samples"].as_array().unwrap();
    assert!(
        samples
            .iter()
            .any(|sample| sample["time_ms"].as_u64().unwrap() >= 24_050)
    );
    assert!(
        samples
            .iter()
            .all(|sample| sample["expected"] == sample["observed"])
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn wcl_intake_replays_hostile_pressure_and_excludes_self_damage() {
    let report = super::replay_simc_party_with_wcl(
        1468,
        90,
        Path::new("not-used-for-wcl"),
        10,
        1,
        &fixture("altar_fangs_wcl_pull1.json"),
        &fixture("altar_fangs_wcl_pull1_damage.json"),
    )
    .unwrap();
    assert_eq!(report["timeline"], "wcl_damage_taken");
    assert_eq!(report["incoming_events"], 0);
    assert_eq!(report["party_incoming_events"], 0);
    assert_eq!(report["wcl_intake"]["records"], 40);
    let applied = report["wcl_intake"]["delivered"].as_u64().unwrap();
    let dead = report["wcl_intake"]["hits_on_dead_targets_not_replayed"]
        .as_u64()
        .unwrap();
    assert!(applied > 0);
    assert_eq!(applied + dead, 28);
    assert_eq!(
        report["wcl_intake"]["zero_damage_outcomes_not_replayed"],
        12
    );
    assert!(
        report["metrics"]["party_damage"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| v.as_u64().unwrap_or(0) > 0)
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
