use super::healer_intake::Intake;
use std::path::Path;

#[test]
#[ignore = "reads the pinned MID2 Protection profile"]
fn tank_replay_rejects_profiles_without_level_90_mid2_inputs() {
    let (mut profile, _) = super::dps_parity::read_reference(66).unwrap();
    super::dps_parity::require_mid2_profile(&profile).unwrap();
    profile.level = "89".into();
    assert!(super::dps_parity::require_mid2_profile(&profile).is_err());
    profile.level = "90".into();
    profile
        .options
        .insert("reference_tier".into(), serde_json::json!("MID1"));
    assert!(super::dps_parity::require_mid2_profile(&profile).is_err());
}

#[test]
#[ignore = "uses the selected MID2 player and native Holy Bulwark casts"]
fn holy_bulwark_absorbs_incoming_hits_and_grows_to_native_cap() {
    fn cast_bulwark() -> super::dps_parity::Run {
        let (profile, traits) = super::dps_parity::read_reference(66).unwrap();
        let mut run = super::dps_parity::spawn_reference(
            profile,
            &traits,
            1,
            crate::followers::simulator::ParityActor::Player { race: 6 },
        )
        .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            432459,
            Some(run.actor),
            false,
        )
        .unwrap();
        for _ in 0..2 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 432496));
        run
    }
    let mut run = cast_bulwark();
    let max = run
        .app
        .world()
        .get::<shared::components::Health>(run.actor)
        .unwrap()
        .max;
    let vers = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .versatility_pct;
    let (_, consumed) = crate::auras::absorb_damage(run.app.world_mut(), run.actor, 1, max * 0.01);
    assert!((consumed - max * 0.01).abs() < 2.0);
    for _ in 0..40 {
        run.app.update();
    }
    let (_, remaining) = crate::auras::absorb_damage(run.app.world_mut(), run.actor, 1, max);
    let expected = max * (0.15 * (1.0 + vers / 100.0) + 0.02 - 0.01);
    assert!(
        (remaining - expected).abs() < 2.0,
        "{remaining} != {expected}"
    );

    let mut run = cast_bulwark();
    let max = run
        .app
        .world()
        .get::<shared::components::Health>(run.actor)
        .unwrap()
        .max;
    for _ in 0..360 {
        run.app.update();
    }
    let (_, absorbed) = crate::auras::absorb_damage(run.app.world_mut(), run.actor, 1, max);
    assert!(
        (absorbed - max * 0.30).abs() < 2.0,
        "{absorbed} != {}",
        max * 0.30
    );
}

#[test]
#[ignore = "uses the selected MID2 66 player and native armament override"]
fn protection_holy_armaments_alternate_after_real_casts() {
    let (profile, traits) = super::dps_parity::read_reference(66).unwrap();
    let mut run = super::dps_parity::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 6 },
    )
    .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 432459)
            .unwrap(),
        432459
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        432459,
        Some(run.actor),
        false,
    )
    .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 432478));
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 432459)
            .unwrap(),
        432472
    );
    let replacement =
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 432459)
            .unwrap();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        replacement,
        Some(run.actor),
        false,
    )
    .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 432478));
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 432459)
            .unwrap(),
        432459
    );
}

#[test]
#[ignore = "uses the real MID2 Protection profile and native fields"]
fn protection_apl_reads_selected_apex_and_native_consecration() {
    use super::tank_replay::protection_rotation::{self, Context};
    use server::simc_apl::Facts;
    let (profile, traits) = super::dps_parity::read_reference(66).unwrap();
    let potion = profile.consumables["potion"].clone();
    let mut run = super::dps_parity::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 6 },
    )
    .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    let mut bindings = protection_rotation::bindings();
    let equipment = super::dps_parity::literal_rotation::equipment::Equipment::load(
        run.app.world(),
        run.actor,
        &mut bindings,
    )
    .unwrap();
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let active =
        crate::spell_cast::scripts::has_active_consecration(run.app.world_mut(), run.actor);
    let context = Context::new(
        run.app.world(),
        run.actor,
        &run.targets,
        &bindings,
        &equipment,
        encounter,
        active,
        None,
        &potion,
    );
    assert_eq!(context.value("apex.3").unwrap(), 1.0);
    assert_eq!(
        context
            .value("talent.instrument_of_the_divine.enabled")
            .unwrap(),
        1.0
    );
    assert_eq!(context.value("consecration.up").unwrap(), 0.0);
    assert_eq!(context.value("potion.liquid_luster").unwrap(), 1.0);
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 26573, None, false).unwrap();
    for _ in 0..5 {
        run.app.update();
    }
    let active =
        crate::spell_cast::scripts::has_active_consecration(run.app.world_mut(), run.actor);
    let context = Context::new(
        run.app.world(),
        run.actor,
        &run.targets,
        &bindings,
        &equipment,
        encounter,
        active,
        None,
        &potion,
    );
    assert_eq!(context.value("consecration.up").unwrap(), 1.0);
    assert!(
        context.value("next_armament").is_err(),
        "uninitialized armament state must not be invented"
    );
}

#[test]
#[ignore = "uses the native Consecration ground field"]
fn protection_apl_consecration_tracks_field_not_caster_position() {
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[26573, 81297], 2).unwrap();
    assert!(!crate::spell_cast::scripts::has_active_consecration(
        h.app.world_mut(),
        h.caster
    ));
    h.cast_and_deliver(26573, None);
    h.advance(5);
    assert!(crate::spell_cast::scripts::has_active_consecration(
        h.app.world_mut(),
        h.caster
    ));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::Position {
            x: 40.0,
            y: 0.0,
            z: 0.0,
        });
    assert!(crate::spell_cast::scripts::has_active_consecration(
        h.app.world_mut(),
        h.caster
    ));
    h.advance(340);
    assert!(!crate::spell_cast::scripts::has_active_consecration(
        h.app.world_mut(),
        h.caster
    ));
}

#[test]
#[ignore = "reads the pinned MID2 Protection profile"]
fn protection_apl_selects_shield_using_the_real_profile_conditions() {
    use server::simc_apl::{Action, ActionFacts, Facts};
    struct FactsForShield {
        full_power: bool,
    }
    impl Facts for FactsForShield {
        fn value(&self, name: &str) -> Result<f64, String> {
            Ok(match name {
                "holy_power" if self.full_power => 5.0,
                "talent.instrument_of_the_divine" | "buff.valor.up" | "buff.divine_purpose.up"
                    if !self.full_power =>
                {
                    1.0
                }
                _ => 0.0,
            })
        }
    }
    impl ActionFacts for FactsForShield {
        fn value_for(&self, _: &Action, name: &str) -> Result<f64, String> {
            self.value(name)
        }
    }
    let (profile, _) = super::dps_parity::read_reference(66).unwrap();
    let program = super::tank_replay::protection_program(&profile.options["actions"]).unwrap();
    let select = |full_power| {
        program
            .select(
                "default",
                &FactsForShield { full_power },
                &mut program.state(),
                |action| Ok(action.name == "shield_of_the_righteous"),
            )
            .unwrap()
    };
    assert_eq!(select(true).unwrap().name, "shield_of_the_righteous");
    assert!(select(false).is_none());
}

#[test]
#[ignore = "requires canonical MID2 profile inputs and a SimC tank-dummy trace"]
fn unregistered_tank_driver_fails_instead_of_using_a_follower() {
    let path = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let error = super::tank_replay::replay_simc(0, Path::new(&path), 300, 70448).unwrap_err();
    assert_eq!(error, "NATIVE_LITERAL_DRIVER_UNAVAILABLE:0");
}

#[test]
#[ignore = "runs the MID2 Protection APL and SimC tank-dummy pressure"]
fn protection_paladin_simc_apl_uses_shield_of_the_righteous() {
    let path = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report = super::tank_replay::replay_simc(66, Path::new(&path), 300, 70448).unwrap();
    assert_eq!(report["spec"], 66);
    assert_eq!(report["level"], 90);
    assert_eq!(report["rotation_status"], "registered_literal_simc");
    assert!(
        report["tank_casts"]["53600"].as_u64().unwrap_or(0) > 0,
        "{report}"
    );
    assert!(
        report["active_mitigation_uptime"].as_f64().unwrap() > 0.0,
        "{report}"
    );
    assert!(report["damage_taken"].as_f64().unwrap() > 0.0, "{report}");
    assert!(
        report["absorb_per_second"].as_f64().unwrap() > 0.0,
        "{report}"
    );
    assert!(
        report["tank_casts"]["85673"].as_u64().unwrap_or(0) > 0,
        "{report}"
    );
    assert!(
        report["self_heal_per_second"].as_f64().unwrap() > 0.0,
        "{report}"
    );
    println!("SIMC_TANK_REPLAY {report}");
}

#[test]
#[ignore = "runs a registered parity actor against SimC dummy pressure"]
fn registered_parity_driver_runs_during_real_incoming_damage() {
    let path = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let hits =
        crate::simc_tank_timeline::read_trace(Path::new(&path), "Tank_Dummy_Dungeon").unwrap();
    let _rng = crate::random::seed_scope(70448);
    // Feral is an existing player driver used to exercise shared dispatch, not a tank measurement.
    let run = super::dps_parity::prepare_registered_player(103, 1).unwrap();
    let report = super::tank_replay::replay_registered_run(run, &hits, 300, &[]).unwrap();
    assert_eq!(report["spec"], 103);
    assert_eq!(report["level"], 90);
    assert_eq!(report["rotation_status"], "registered_literal_simc");
    assert!(report["incoming_events"].as_u64().unwrap() > 0, "{report}");
    assert!(report["damage_taken"].as_f64().unwrap() > 0.0, "{report}");
    assert!(
        !report["tank_casts"].as_object().unwrap().is_empty(),
        "{report}"
    );
}

#[test]
#[ignore = "uses the native recorded tank source and recipient"]
fn tank_replay_counts_parried_incoming_damage_as_mitigated() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/dungeons/healer-benchmark/stockade-seed1.intake.json");
    let mut intake: Intake = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut hit = intake.hits[0].clone();
    hit.raw = 100.0;
    if let super::healer_intake::HitKind::Melee {
        ref mut outcome, ..
    } = hit.kind
    {
        *outcome = "Parry".into();
    } else {
        panic!("recorded first hit must be melee");
    }
    intake.end_tick = hit.tick;
    intake.hits = vec![hit];
    let report = super::tank_replay::replay(&intake).unwrap();
    assert_eq!(report["raw_damage"], 100.0);
    assert_eq!(report["damage_taken"], 0.0);
    assert_eq!(report["mitigation_pct"], 100.0);
}

#[test]
#[ignore = "uses native recipient avoidance and the SimC dummy strike policy"]
fn simc_dummy_melee_uses_recipient_avoidance_and_strikes_can_block() {
    use crate::melee::MeleeOutcome;
    let _rng = crate::random::seed_scope(1);
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[], 5).unwrap();
    let mut stats = h
        .app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap();
    stats.defense.dodge = 0.0;
    stats.defense.parry = 100.0;
    stats.defense.block = 100.0;
    drop(stats);
    let mut saw_parry = false;
    for _ in 0..100 {
        let (swing, _) =
            crate::combat::roll_simc_dummy_outcome(h.app.world(), h.npc, h.caster, false).unwrap();
        assert!(matches!(swing, MeleeOutcome::Miss | MeleeOutcome::Parry));
        saw_parry |= swing == MeleeOutcome::Parry;
        let (strike, _) =
            crate::combat::roll_simc_dummy_outcome(h.app.world(), h.npc, h.caster, true).unwrap();
        assert_eq!(strike, MeleeOutcome::Block);
    }
    assert!(saw_parry);
}

#[test]
#[ignore = "uses native recipient armor and absorb data"]
fn simc_dummy_bleed_uses_native_absorbs_but_not_armor() {
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[17], 5).unwrap();
    h.app
        .insert_resource(crate::test_data::world_data().armor_constants.clone());
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .armor = 5000.0;
    let before = h.health(h.caster);
    crate::spell_cast::effects::replay_simc_dummy_bleed(h.app.world_mut(), h.npc, h.caster, 200.0);
    assert_eq!(
        before - h.health(h.caster),
        200.0,
        "physical bleed bypasses armor"
    );
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 17,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let shielded = h.health(h.caster);
    crate::spell_cast::effects::replay_simc_dummy_bleed(h.app.world_mut(), h.npc, h.caster, 200.0);
    assert_eq!(
        h.health(h.caster),
        shielded,
        "native shield absorbs the bleed"
    );
}

#[test]
#[ignore = "requires the MID2 Protection Paladin profile and canonical world database"]
fn protection_paladin_uses_parity_player_profile_and_equipment() {
    let canonical = Path::new("/home/osso/Projects/world-of-osso/game-server/data/world.db")
        .canonicalize()
        .unwrap();
    assert_eq!(
        crate::test_data::world_db_path().canonicalize().unwrap(),
        canonical
    );
    assert_eq!(
        Path::new(&std::env::var("DPS_PARITY_WORLD_DB").unwrap())
            .canonicalize()
            .unwrap(),
        canonical
    );
    let (profile, traits) = super::dps_parity::read_reference(66).unwrap();
    assert_eq!(profile.level, "90");
    assert_eq!(profile.race, "tauren");
    let mut expected: Vec<_> = profile.equipment.iter().map(|item| item.item_id).collect();
    expected.sort_unstable();
    let run = super::dps_parity::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 6 },
    )
    .unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Level>(run.actor)
            .unwrap()
            .0,
        90
    );
    let mut actual: Vec<_> = run
        .app
        .world()
        .get::<crate::stats::EquippedItems>(run.actor)
        .unwrap()
        .items
        .iter()
        .map(|(_, _, id)| *id)
        .collect();
    actual.sort_unstable();
    assert_eq!(actual, expected);
    let loadout = run
        .app
        .world()
        .get::<crate::class_progression::TraitLoadout>(run.actor)
        .unwrap();
    assert!(
        loadout
            .active_talent_spells()
            .any(|spell| spell.spell_id == 433568)
    );
    let modifiers = run
        .app
        .world()
        .get::<crate::stats::EquippedItemModifiers>(run.actor)
        .unwrap();
    assert!(
        modifiers.temporary_enchants.is_empty(),
        "selected Rite must suppress the conditional oil"
    );
    assert_eq!(run.targets.len(), 1);
}

#[test]
#[ignore = "replays a recorded Stockade intake against the canonical world database"]
fn protection_paladin_replays_recorded_damage_without_healer() {
    let canonical = Path::new("/home/osso/Projects/world-of-osso/game-server/data/world.db")
        .canonicalize()
        .unwrap();
    assert_eq!(
        crate::test_data::world_db_path().canonicalize().unwrap(),
        canonical
    );
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/dungeons/healer-benchmark/stockade-seed1.intake.json");
    let intake: Intake = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let report = super::tank_replay::replay(&intake).unwrap();
    assert_eq!(report["spec"], 66);
    let raw = report["raw_damage"].as_f64().unwrap();
    let taken = report["damage_taken"].as_f64().unwrap();
    assert!(raw > 0.0 && taken > 0.0 && taken <= raw, "{report}");
    assert!(
        (report["mitigation_pct"].as_f64().unwrap() - 100.0 * (raw - taken) / raw).abs() < 1e-6
    );
    let duration = report["duration_seconds"].as_f64().unwrap();
    assert!(duration > 0.0);
    assert!((0.0..=1.0).contains(&report["active_mitigation_uptime"].as_f64().unwrap()));
    assert!(report["largest_spike"].as_f64().unwrap() > 0.0);
    assert!(
        !report["tank_casts"].as_object().unwrap().is_empty(),
        "{report}"
    );
    if !report["time_to_death"].is_null() {
        assert_eq!(report["time_to_death"].as_f64().unwrap(), duration);
    }
    println!("TANK_REPLAY {report}");
}
