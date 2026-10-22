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
#[ignore = "explicit native Azure70 seed7 pet Death, Fury evade and pending-fight recovery"]
fn seed_seven_resumes_the_native_pending_fury_after_its_pet_dies() {
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
                pet_died |= pet.is_some_and(|pet| event.target == Some(pet.to_bits()));
                selected_died |= event.target == Some(selected.to_bits());
            }
            if pet_died
                && selected_died
                && event.kind == CombatLogKind::Damage
                && event.source == Some(tank.to_bits())
                && event.target == Some(pending.to_bits())
                && event.amount > event.overflow.max(0)
            {
                assert_eq!(run.current_pack, Some(29001792));
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
                pet_died && selected_died,
                "native failure ordering must include actual pet and selected Fury Deaths"
            );
            panic!(
                "native pending Fury must be resumed, not strand a fully ready party: {}",
                run.report.detail
            );
        }
    }
    panic!("native pending-fight recovery must occur within the unchanged campaign bound");
}
