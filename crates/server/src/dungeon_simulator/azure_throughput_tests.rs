//! Actual native campaign throughput boundaries, not replacement combat formulas.
use super::super::catalogs::Catalogs;
use super::spawn_run;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "explicit native Azure70 BM leader Call Pet and damage across cached worlds"]
fn native_bm_leader_summons_and_uses_its_pet_without_transport_identity() {
    let _rng_scope = crate::random::seed_scope(1);
    // The frozen cohort predates the duck import. Use the existing authenticated
    // class-summon fixture, not an invented template or a canonical DB write.
    let prepared = crate::spell_cast::test_harness::prepare_class_world();
    let mut catalogs = None;
    for iteration in 0..2 {
        let mut run = spawn_run(1, &mut catalogs).unwrap();
        assert_eq!(run.graph.map_id, 2515);
        let owner = run.party.owner;
        if iteration == 0 {
            let data = prepared.world_data();
            assert!(data.templates.contains_key(&174884));
            run.app.insert_resource(data);
        }
        let mut damaged = false;
        for _ in 0..1200 {
            run.control().unwrap();
            run.app.update();
            if let Some(pet) = crate::hunter_pet::summoned_pet(run.app.world(), owner) {
                damaged = run
                    .app
                    .world()
                    .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                    .0
                    .iter()
                    .any(|event| {
                        event.kind == CombatLogKind::Damage
                            && event.source == Some(pet.to_bits())
                            && event.amount > 0
                    });
                if damaged {
                    break;
                }
            }
        }
        assert!(
            damaged,
            "native BM leader must summon through Call Pet and deal real pet damage in cached world{iteration}; preparation={:?}",
            crate::hunter_pet::provision_follower_duck(run.app.world_mut(), owner)
        );
        // Observe the existing pet-spell path after ownership is real. Missing
        // Wild Thrash damage is reported to its mechanics owner, not fixed here.
        for _ in 0..600 {
            run.control().unwrap();
            run.app.update();
        }
        let pet = crate::hunter_pet::summoned_pet(run.app.world(), owner).unwrap();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let wild_thrash_casts = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(owner.to_bits())
                    && event.spell_id == Some(1264359)
            })
            .count();
        let wild_thrash_damage: i64 = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.source == Some(pet.to_bits())
                    && event.spell_id == Some(1264355)
            })
            .map(|event| i64::from(event.amount))
            .sum();
        let sheets: Vec<_> = run
            .members()
            .map(|unit| {
                let stats = run
                    .app
                    .world()
                    .get::<crate::stats::CharacterStats>(unit)
                    .unwrap();
                serde_json::json!({"entity":unit.to_bits(), "primary":stats.primary,
                "spell_power":stats.spell_power,"attack_power":stats.attack_power,
                "ranged_attack_power":stats.ranged_attack_power,"crit_pct":stats.crit_pct,
                "haste_pct":stats.haste_pct,"mastery_points":stats.mastery_points,
                "mastery_pct":stats.mastery_pct,"versatility_pct":stats.versatility_pct,
                "max_health":stats.max_health,"max_mana":stats.max_mana})
            })
            .collect();
        println!(
            "NATIVE_PET_WIRING {}",
            serde_json::json!({"iteration":iteration,
            "sim_s":run.now(),"owner":owner.to_bits(),"pet":pet.to_bits(),
            "wild_thrash_casts":wild_thrash_casts,"wild_thrash_pet_damage":wild_thrash_damage,
            "party_stat_sheets":sheets})
        );
        let stable = run
            .app
            .world()
            .get::<crate::hunter_pet::PetStable>(owner)
            .unwrap();
        assert_eq!(
            stable.character_id, None,
            "no fabricated persistent identity"
        );
        assert!(
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(owner.to_bits())
                    && event.spell_id == Some(883))
        );
        catalogs = Some(Catalogs::take(run.app.world_mut()));
    }
}

#[test]
#[ignore = "explicit native Azure70 seed7 early selected-Fury Death and next-group progression"]
fn seed_seven_advances_after_the_native_selected_fury_dies() {
    let _rng_scope = crate::random::seed_scope(7);
    let mut run = spawn_run(7, &mut None).unwrap();
    assert_eq!(run.graph.map_id, 2515);
    assert!(
        run.app
            .world()
            .resource::<crate::world_data::WorldData>()
            .templates
            .contains_key(&174884)
    );
    let owner = run.party.owner;
    let tank = run.party.tank;
    let selected = run.actors[&29001807];
    let pending = run.actors[&29001792];
    let mut pet = None;
    let mut pet_died = false;
    let mut selected_died = false;
    let mut cursor = 0;
    for tick in 0..super::MAX_TICKS {
        if tick % 10 == 0 {
            run.sample();
        }
        run.control().unwrap();
        run.app.update();
        run.healer_metrics.observe_frame(run.app.world());
        if tick % 1000 == 0 {
            let _ = crate::followers::simulator::inspect_tank_movement(run.app.world_mut(), tank);
        }
        pet = pet.or_else(|| crate::hunter_pet::summoned_pet(run.app.world(), owner));
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[cursor..] {
            if event.kind == CombatLogKind::Death {
                let pet_death = pet.is_some_and(|pet| event.target == Some(pet.to_bits()));
                if pet_death {
                    println!(
                        "NATIVE_PET_DEATH {}",
                        serde_json::json!({
                            "at":run.now(),
                            "pet":pet.map(|unit| super::super::diagnostics::inspect_unit(run.app.world(),None,unit)),
                            "attacker":event.source.and_then(bevy::prelude::Entity::try_from_bits)
                                .map(|unit| super::super::diagnostics::inspect_unit(run.app.world(),None,unit)),
                            "party":run.members().map(|unit| super::super::diagnostics::inspect_unit(run.app.world(),None,unit)).collect::<Vec<_>>(),
                        })
                    );
                }
                pet_died |= pet_death;
                selected_died |= event.target == Some(selected.to_bits());
            }
            if selected_died
                && event.kind == CombatLogKind::Damage
                && event.source == Some(tank.to_bits())
                && event.target == Some(pending.to_bits())
                && event.amount > event.overflow.max(0)
            {
                return;
            }
        }
        cursor = events.len();
        if run.stalled() {
            println!(
                "PENDING_FURY_HOLD {}",
                serde_json::json!({
                    "at":run.now(), "pet_died":pet_died, "selected_died":selected_died,
                    "has_uncleared_fight":crate::followers::simulator::has_uncleared_fight(run.app.world(),owner),
                    "next_pack":run.next_pack().map(|(pack,unit)| (pack,unit.to_bits())),
                    "pending":super::super::diagnostics::inspect_unit(run.app.world(),Some(29001792),pending),
                    "selected":super::super::diagnostics::inspect_unit(run.app.world(),Some(29001807),selected),
                    "party":run.members().map(|unit| super::super::diagnostics::inspect_unit(run.app.world(),None,unit)).collect::<Vec<_>>(),
                })
            );
            assert!(
                selected_died,
                "native failure ordering must include actual selected Fury Death"
            );
            panic!(
                "native pending Fury must be resumed, not strand a fully ready party: {}",
                run.report.detail
            );
        }
    }
    panic!("native pending-fight recovery must occur within the unchanged campaign bound");
}

#[test]
#[ignore = "explicit native pet-only damage, actual pet Death and evade/home history boundary"]
fn native_pet_only_enemy_reset_does_not_strand_the_next_authored_pull() {
    let _rng_scope = crate::random::seed_scope(7);
    let mut run = spawn_run(7, &mut None).unwrap();
    let owner = run.party.owner;
    let tank = run.party.tank;
    for _ in 0..200 {
        run.app.update();
        if crate::hunter_pet::summoned_pet(run.app.world(), owner).is_some() {
            break;
        }
    }
    let pet =
        crate::hunter_pet::summoned_pet(run.app.world(), owner).expect("native CallPet input");
    let pending = run.actors[&29001792];
    // Initialize real history through the same admission boundary, before damage.
    let _ =
        crate::followers::simulator::pull(run.app.world_mut(), tank, &[29001799].into()).unwrap();
    crate::combat::deal_direct_damage(run.app.world_mut(), pet, pending, 1.0);
    assert!(
        crate::followers::simulator::has_uncleared_fight(run.app.world(), owner),
        "living pet-attacked enemy is a real pending fight before evade"
    );
    crate::combat::kill_unit(run.app.world_mut(), pending, pet);
    crate::creature_aggro::enter_evade_mode(run.app.world_mut(), pending);
    for _ in 0..200 {
        run.app.update();
        if run
            .app
            .world()
            .get::<crate::creature_ai::CreatureState>(pending)
            == Some(&crate::creature_ai::CreatureState::Idle)
        {
            break;
        }
    }
    assert_eq!(
        run.app
            .world()
            .get::<crate::creature_ai::CreatureState>(pending),
        Some(&crate::creature_ai::CreatureState::Idle)
    );
    assert!(
        run.app
            .world()
            .get::<shared::components::Health>(pending)
            .unwrap()
            .current
            > 0.0,
        "evade is not Death or cleared-source credit"
    );
    let expected = run.next_pack().expect("authored native opponents remain").0;
    run.current_pack = None;
    run.control().unwrap();
    assert_eq!(
        run.current_pack,
        Some(expected),
        "real enemy evade/home after pet Death must release the stale pending-fight hold"
    );
    assert!(run.report.bosses_killed.is_empty());
}

#[test]
#[ignore = "explicit native gameplay schedule-unwind and durable abort report/export boundary"]
fn native_schedule_abort_preserves_report_and_ordered_combat_events() {
    let _rng_scope = crate::random::seed_scope(7);
    let started = std::time::Instant::now();
    let mut run = spawn_run(7, &mut None).unwrap();
    let tank = run.party.tank;
    let enemy = run.actors[&29001799];
    crate::combat::deal_direct_damage(run.app.world_mut(), tank, enemy, 1.0);
    let before = serde_json::to_value(
        &run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0,
    )
    .unwrap();
    assert!(
        before
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["kind"] == "Damage"
                && event["source"] == tank.to_bits()
                && event["target"] == enemy.to_bits()
                && event["amount"] == 1)
    );
    run.app.add_systems(bevy::prelude::FixedUpdate, || {
        panic!("explicit native FixedUpdate abort regression");
    });
    let report = super::run_prepared(run, &mut None, started, started.elapsed())
        .expect("schedule unwind must become an explicit aborted report, not lose evidence");
    assert_eq!(report.status, "aborted");
    assert!(
        report
            .detail
            .contains("explicit native FixedUpdate abort regression")
    );
    let trace = report.performance["combat_events"].as_array().unwrap();
    assert!(
        trace.starts_with(before.as_array().unwrap()),
        "the ordered native combat trace before the abort must survive unchanged"
    );
    super::super::runtime::write_report(&report).unwrap();
    let output = super::super::runtime::output_directory();
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("run-7.json")).unwrap()).unwrap();
    let exported: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("combat-events-7.json")).unwrap())
            .unwrap();
    assert_eq!(saved["status"], "aborted");
    assert!(
        saved["detail"]
            .as_str()
            .unwrap()
            .contains("explicit native FixedUpdate abort regression")
    );
    assert_eq!(exported["events"], report.performance["combat_events"]);
    assert_eq!(exported["status"], "aborted");
    assert_eq!(exported["reason"], saved["detail"]);
}
