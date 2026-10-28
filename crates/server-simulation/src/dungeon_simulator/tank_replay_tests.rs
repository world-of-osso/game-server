use super::healer_intake::Intake;
use std::path::Path;

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
