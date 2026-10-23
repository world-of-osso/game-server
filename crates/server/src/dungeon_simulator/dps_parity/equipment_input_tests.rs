//! Native reference gear must reach StatsPlugin with per-slot redirected allocations.
use super::inputs;
use crate::class_progression::ActiveSpec;
use bevy::prelude::*;
use shared::components::{CombatRatings, DerivedStats, Health, Level, Player, UnitStats};

fn equipment_app() -> App {
    let mut app = App::new();
    inputs::install_reference_catalogs(&mut app).unwrap();
    let path = std::env::var_os("DPS_PARITY_WORLD_DB").expect("frozen native database");
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (spells, classes) = crate::spell_info::load_spell_data(&db).unwrap();
    app.insert_resource(spells)
        .insert_resource(classes)
        .add_plugins(crate::stats::StatsPlugin);
    app
}

fn spawn_reference_player(app: &mut App) -> Entity {
    // Pinned103 is a level90 Night Elf Druid; no traits/auras/consumables here:
    // this boundary measures base item allocation, not combat or proc stat grants.
    app.world_mut()
        .spawn((
            Player {
                name: "NativeGearInput".into(),
                class: 11,
                race: 4,
                appearance: Default::default(),
            },
            Level(90),
            ActiveSpec(103),
            UnitStats::default(),
            CombatRatings::default(),
            DerivedStats::default(),
            Health {
                current: 1.0,
                max: 1.0,
            },
        ))
        .id()
}

fn equip_base_slot(
    app: &mut App,
    actor: Entity,
    name: &str,
) -> (u32, crate::item_templates::ItemTemplate) {
    let path = inputs::directory().unwrap().join("103.profile.json");
    let mut profile: inputs::Profile = inputs::read_json(&path).unwrap();
    profile.equipment.retain(|item| item.slot == name);
    assert_eq!(profile.equipment.len(), 1, "actual profile slot {name}");
    let item = &mut profile.equipment[0];
    item.enchant = 0;
    item.gems.clear();
    let id = item.item_id;
    let original = *app
        .world()
        .resource::<crate::item_templates::ItemTemplates>()
        .retail
        .get(&id)
        .expect("actual native profile item");
    inputs::equip(app.world_mut(), actor, &profile).unwrap();
    app.update();
    (id, original)
}

#[test]
fn dps_parity_equipment_redirected_base_stats_are_modeled_without_hiding_other_options() {
    let profile: inputs::Profile = serde_json::from_value(serde_json::json!({
        "spec_id": 103, "level": "90", "race": "night_elf", "consumables": {},
        "equipment": [{"slot": "head", "item_id": 271528,
            "bonus_list": [4786,4800,13692,13698,13847,13848], "item_level_override": 344,
            "enchant": 8017, "gems": [], "crafted_stats": [],
            "options": {"redirected_base_stats": "271875", "future_item_option": "1"}}]
    }))
    .unwrap();
    let gaps = super::report::gaps(&profile, true);
    assert!(
        !gaps.iter().any(|gap| gap.contains("redirected_base_stats")),
        "implemented option is not an unsupported gear gap: {gaps:?}"
    );
    assert!(
        gaps.contains(&"ITEM_OPTION_UNMODELED:271528:future_item_option=1".into()),
        "other unsupported options remain explicit: {gaps:?}"
    );
}

#[test]
#[ignore = "native per-slot redirected-base-stats RED; frozen103 profile/database required"]
fn dps_parity_equipment_redirected_native_base_stats_match_all_fifteen_slots() {
    // Exact per-slot base ratings from pinned76f8118's103 profile. All ten
    // non-redirected controls already agree; only the five tier allocations differ.
    // Order: Crit, Haste, Mastery, Versatility. No enchant/gem/food contributions.
    let cases = [
        ("head", [0.0, 209.0, 0.0, 0.0]),
        ("neck", [107.0, 107.0, 107.0, 107.0]),
        ("shoulders", [99.0, 0.0, 52.0, 0.0]),
        ("back", [0.0, 56.0, 56.0, 0.0]),
        ("chest", [0.0, 139.0, 62.0, 0.0]),
        ("wrists", [0.0, 56.0, 56.0, 0.0]),
        ("hands", [48.0, 0.0, 102.0, 0.0]),
        ("waist", [0.0, 49.0, 108.0, 0.0]),
        ("legs", [0.0, 66.0, 143.0, 0.0]),
        ("feet", [105.0, 46.0, 0.0, 0.0]),
        ("finger1", [73.0, 0.0, 332.0, 0.0]),
        ("finger2", [338.0, 67.0, 0.0, 0.0]),
        ("trinket1", [149.0, 0.0, 0.0, 0.0]),
        ("trinket2", [0.0, 0.0, 0.0, 0.0]),
        ("main_hand", [0.0, 0.0, 209.0, 0.0]),
    ];
    let mut app = equipment_app();
    let actor = spawn_reference_player(&mut app);
    let mut mismatches = Vec::new();
    for (name, expected) in cases {
        let (item_id, original) = equip_base_slot(&mut app, actor, name);
        let sheet = app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap();
        let actual = [
            sheet.ratings.crit,
            sheet.ratings.haste,
            sheet.ratings.mastery,
            sheet.ratings.versatility,
        ];
        if actual != expected {
            mismatches.push(format!(
                "{name} item{item_id}: native{actual:?}, expected{expected:?}"
            ));
        }
        let equipped = app
            .world()
            .get::<crate::stats::EquippedItems>(actor)
            .unwrap();
        assert_eq!(
            equipped.items[0].2, item_id,
            "redirect must not replace equipped identity"
        );
        let live = app
            .world()
            .resource::<crate::item_templates::ItemTemplates>();
        assert_eq!(
            live.retail.get(&item_id),
            Some(&original),
            "never mutate native catalog"
        );
    }
    assert!(
        mismatches.is_empty(),
        "native per-slot base-stat differences: {mismatches:#?}"
    );
}

#[test]
#[ignore = "native redirected-base-stats invalid-input RED; frozen103 profile/database required"]
fn dps_parity_equipment_redirected_base_stats_reject_invalid_native_donors() {
    let mut app = equipment_app();
    let actor = spawn_reference_player(&mut app);
    let path = inputs::directory().unwrap().join("103.profile.json");
    let mut profile: inputs::Profile = inputs::read_json(&path).unwrap();
    profile.equipment.retain(|item| item.slot == "head");
    for donor in ["not_an_item".to_string(), u32::MAX.to_string()] {
        profile.equipment[0]
            .options
            .insert("redirected_base_stats".into(), donor.clone());
        let result = inputs::equip(app.world_mut(), actor, &profile);
        assert!(
            result.is_err(),
            "invalid redirect {donor} cannot silently keep original allocation"
        );
    }
}
