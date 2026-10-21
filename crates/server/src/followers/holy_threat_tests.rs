//! Actual native103 casts must suppress an NPC reference without erasing raw threat.
use super::kit_clearcasting_tests::{complete_real_cast, frame_seconds, own_aura};
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use crate::spell_cast::test_harness::{Harness, load_native_creature_world};
use bevy::prelude::*;
use shared::components::{Level, ModelDisplay, ThreatTable, UnitLevel};

fn native_fixture(entry: u32) -> (Harness, Entity) {
    let (mut h, old_unit, _profile) = spawn_profile_fixture_with_plugins(entry, |_| {});
    h.app.world_mut().despawn(old_unit);
    let owner = h.caster;
    crate::combat::attack_stop(h.app.world_mut(), owner);
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((Level(44), UnitLevel(44)));
    let data = load_native_creature_world(&[entry, 6]);
    let display_id = data.templates[&6].models[0].display_id;
    h.app.insert_resource(data);
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(super::catalog::load_catalog(&db).unwrap());
    h.app
        .insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&db).unwrap());
    let unit = super::native_fixture::spawn(h.app.world_mut(), owner, entry).unwrap();
    h.app.world_mut().entity_mut(h.npc).insert((
        ModelDisplay { display_id },
        ThreatTable::default(),
        crate::creature_ai::CreatureState::Idle,
    ));
    h.advance(3);
    (h, unit)
}

fn prove_reversible_native_threat(entry: u32, spell: u32) {
    let (mut h, unit) = native_fixture(entry);
    let enemy = h.npc;
    let owner = h.caster;
    crate::creature_aggro::add_threat(h.app.world_mut(), enemy, unit, 1000.0);
    crate::creature_aggro::add_threat(h.app.world_mut(), enemy, owner, 500.0);
    assert_eq!(
        crate::creature_aggro::current_victim(h.app.world_mut(), enemy),
        Some(unit)
    );
    complete_real_cast(&mut h, unit, spell);
    assert!(
        own_aura(&h, unit, spell).is_some(),
        "native threat aura must really apply"
    );
    assert_eq!(
        crate::creature_aggro::current_victim(h.app.world_mut(), enemy),
        Some(owner),
        "temporary threat must affect a native NPC caster, not only Player-tagged units"
    );
    assert_eq!(
        h.app
            .world()
            .get::<ThreatTable>(enemy)
            .unwrap()
            .threat_for(unit.to_bits()),
        1000.0,
        "suppression must not destroy native base threat"
    );
    let expiry = own_aura(&h, unit, spell).unwrap().expires_at.unwrap();
    let remaining = expiry - crate::auras::aura_now(h.app.world());
    h.advance((remaining / frame_seconds(&h)).ceil() as usize + 3);
    assert!(own_aura(&h, unit, spell).is_none());
    assert_eq!(
        crate::creature_aggro::current_victim(h.app.world_mut(), enemy),
        Some(unit),
        "ordinary expiry must restore the same unmodified threat reference"
    );
}

#[test]
fn holy_threat_native_ai_fades_when_the_enemy_attacks_henry() {
    let (mut h, priest) = native_fixture(215011);
    let enemy = h.npc;
    let owner = h.caster;
    crate::creature_aggro::add_threat(h.app.world_mut(), enemy, priest, 1000.0);
    crate::creature_aggro::add_threat(h.app.world_mut(), enemy, owner, 500.0);
    assert_eq!(
        crate::creature_aggro::current_victim(h.app.world_mut(), enemy),
        Some(priest)
    );
    super::kit_tests::uses_spell(&mut h, priest, 586);
    assert!(own_aura(&h, priest, 586).is_some());
    assert_eq!(
        crate::creature_aggro::current_victim(h.app.world_mut(), enemy),
        Some(owner)
    );
}

#[test]
fn holy_threat_native_henry_fade_switches_victim_then_restores_on_expiry() {
    prove_reversible_native_threat(215011, 586);
}

#[test]
fn holy_threat_native_meredy_mirror_image_has_the_same_reversible_modifier() {
    // This qualifies55342's native103 only, not its separate guardian/DUMMY3 consequence.
    prove_reversible_native_threat(209059, 55342);
}
