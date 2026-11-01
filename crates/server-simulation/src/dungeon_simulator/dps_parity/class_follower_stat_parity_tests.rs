//! Real class-follower and Player builds must respond alike to native stat auras and gear changes.
use super::*;
use shared::components::{CombatRatings, PowerType};
use shared::protocol::EquipmentSlot;

fn matching_blood_builds() -> (App, Entity, Entity) {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let source = crate::followers::class_profiles::read_profiles()
        .unwrap()
        .into_iter()
        .find(|profile| profile.spec_id == 250)
        .unwrap();
    let follower = crate::followers::simulator::spawn_parity_attacker(
        app.world_mut(),
        250,
        90,
        copy,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    let player = crate::followers::simulator::spawn_parity_attacker(
        app.world_mut(),
        250,
        90,
        copy,
        crate::followers::simulator::ParityActor::Player { race: source.race },
    )
    .unwrap();
    crate::followers::class_spawn::install_class(
        app.world_mut(),
        player,
        player,
        &source,
        0,
        0,
        crate::followers::class_spawn::GearSource::Journal,
    )
    .unwrap();
    for _ in 0..10 {
        app.update();
    }
    let follower_items = app
        .world()
        .get::<crate::stats::EquippedItems>(follower)
        .unwrap();
    let player_items = app
        .world()
        .get::<crate::stats::EquippedItems>(player)
        .unwrap();
    assert_eq!(
        follower_items, player_items,
        "both actors must wear the same real Journal gear"
    );
    assert!(!follower_items.items.is_empty());
    (app, follower, player)
}

fn apply_native_aura(app: &mut App, units: [Entity; 2], spell: u32) {
    for unit in units {
        crate::auras::apply_aura(
            app.world_mut(),
            &crate::auras::AuraApplication {
                caster: unit,
                target: unit,
                spell_id: spell,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
        assert!(crate::auras::has_aura(app.world(), unit, spell));
    }
    for _ in 0..10 {
        app.update();
    }
}

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.1,
        "follower {actual}, player {expected}"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn class_follower_player_stat_primary_percent_matches_same_gear_player() {
    let (mut app, follower, player) = matching_blood_builds();
    let before = app
        .world()
        .get::<crate::stats::CharacterStats>(player)
        .unwrap()
        .primary
        .strength;
    apply_native_aura(&mut app, [follower, player], 16612);
    let world = app.world();
    let follower_value = world
        .get::<crate::stats::CharacterStats>(follower)
        .unwrap()
        .primary
        .strength;
    let player_value = world
        .get::<crate::stats::CharacterStats>(player)
        .unwrap()
        .primary
        .strength;
    assert!(
        player_value > before,
        "native Strength aura must change the control"
    );
    close(follower_value, player_value);
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn class_follower_player_stat_rating_conversion_matches_same_gear_player() {
    let (mut app, follower, player) = matching_blood_builds();
    let before = app.world().get::<CombatRatings>(player).unwrap().dodge;
    apply_native_aura(&mut app, [follower, player], 231065);
    let world = app.world();
    let follower_value = world.get::<CombatRatings>(follower).unwrap().dodge;
    let player_value = world.get::<CombatRatings>(player).unwrap().dodge;
    assert!(
        player_value > before,
        "native Lightning Reflexes must change the control"
    );
    close(follower_value, player_value);
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn class_follower_player_stat_max_power_matches_same_gear_player() {
    let (mut app, follower, player) = matching_blood_builds();
    let max = |world: &World, unit| {
        world
            .get::<crate::spell_cast::powers::PowerPools>(unit)
            .unwrap()
            .get(PowerType::RunicPower)
            .unwrap()
            .max
    };
    let before = max(app.world(), player);
    apply_native_aura(&mut app, [follower, player], 228459);
    let world = app.world();
    let follower_value = max(world, follower);
    let player_value = max(world, player);
    assert!(
        player_value > before,
        "native max Runic Power aura must change the control"
    );
    close(follower_value, player_value);
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn class_follower_player_stat_armor_passive_tracks_real_equipment() {
    let (mut app, follower, player) = matching_blood_builds();
    const PLATE_SPECIALIZATION: u32 = 86537;
    for unit in [follower, player] {
        assert!(
            crate::auras::has_aura(app.world(), unit, PLATE_SPECIALIZATION),
            "complete native plate gear earns the passive"
        );
    }
    let gear = app
        .world()
        .get::<crate::stats::EquippedItems>(player)
        .unwrap()
        .clone();
    for unit in [follower, player] {
        app.world_mut()
            .get_mut::<crate::stats::EquippedItems>(unit)
            .unwrap()
            .items
            .retain(|(slot, _, _)| *slot != EquipmentSlot::Chest);
    }
    for _ in 0..10 {
        app.update();
    }
    for unit in [follower, player] {
        assert!(
            !crate::auras::has_aura(app.world(), unit, PLATE_SPECIALIZATION),
            "removing plate chest removes the passive"
        );
        app.world_mut().entity_mut(unit).insert(gear.clone());
    }
    for _ in 0..10 {
        app.update();
    }
    for unit in [follower, player] {
        assert!(
            crate::auras::has_aura(app.world(), unit, PLATE_SPECIALIZATION),
            "restoring plate chest restores the passive"
        );
    }
}
