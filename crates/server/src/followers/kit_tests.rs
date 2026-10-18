//! Native 70448 level-32 followers must use more than their four legacy slots.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use super::*;
use crate::spell_cast::test_harness::Harness;
use shared::components::{Health, Level};

fn level32(entry: u32) -> (Harness, Entity) {
    level32_with_plugins(entry, |_| {})
}

fn level32_with_plugins(entry: u32, configure: impl FnOnce(&mut App)) -> (Harness, Entity) {
    at_level(entry, 32, configure)
}

fn at_level(entry: u32, level: u8, configure: impl FnOnce(&mut App)) -> (Harness, Entity) {
    let (mut h, unit, profile) = spawn_profile_fixture_with_plugins(entry, configure);
    h.app.world_mut().get_mut::<Level>(h.caster).unwrap().0 = level;
    let kit_db = std::env::var_os("FOLLOWER_KIT_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(crate::test_data::world_db_path);
    let db =
        rusqlite::Connection::open_with_flags(kit_db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let catalog = catalog::load_catalog(&db).unwrap();
    assert!(
        !catalog.kits.is_empty(),
        "publish follower_spec_kit or supply FOLLOWER_KIT_DB copy"
    );
    h.app.insert_resource(catalog);
    spawning::initialize(h.app.world_mut(), h.caster, unit, profile).unwrap();
    h.attack(h.npc);
    h.advance(3);
    (h, unit)
}

fn wounded(h: &mut Harness, unit: Entity, fraction: f32) {
    let mut health = h.app.world_mut().get_mut::<Health>(unit).unwrap();
    health.current = health.max * fraction;
}

fn uses_spell(h: &mut Harness, unit: Entity, spell: u32) {
    for _ in 0..600 {
        update_ai(h.app.world_mut());
        h.advance(1);
        if h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == spell)
        {
            return;
        }
    }
    panic!(
        "follower did not cast native spell {spell}; casts: {:?}",
        h.inbox
            .go
            .iter()
            .filter(|go| go.caster == unit.to_bits())
            .map(|go| go.spell_id)
            .collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "RED: four-slot Garrick never selects Lay on Hands"]
fn follower_kit_garrick32_emergency_heal() {
    let (mut h, unit) = level32(209057);
    wounded(&mut h, unit, 0.15);
    uses_spell(&mut h, unit, 633);
    assert!(h.health(unit) > h.app.world().get::<Health>(unit).unwrap().max * 0.15);
}

#[test]
#[ignore = "RED: four-slot Crenna never maintains Rejuvenation"]
fn follower_kit_crenna32_rejuvenation() {
    let (mut h, unit) = level32(209072);
    let patient = h.caster;
    wounded(&mut h, patient, 0.7);
    uses_spell(&mut h, unit, 774);
    assert!(crate::auras::has_aura(h.app.world(), patient, 774));
}

#[test]
#[ignore = "RED: Breka has no multi-enemy Thunder Clap priority"]
fn follower_kit_breka32_area_attack() {
    let (mut h, unit) = level32(215517);
    uses_spell(&mut h, unit, 6343);
}

#[test]
#[ignore = "RED: Henry never prioritizes Holy Word Serenity"]
fn follower_kit_henry32_serenity() {
    let (mut h, unit) = level32(215011);
    let patient = h.caster;
    wounded(&mut h, patient, 0.3);
    uses_spell(&mut h, unit, 2050);
}

#[test]
#[ignore = "RED: Shuja has no Flame Shock maintenance priority"]
fn follower_kit_shuja32_damage_over_time() {
    let (mut h, unit) = level32(209053);
    uses_spell(&mut h, unit, 470411);
}

#[test]
#[ignore = "RED: Meredy previously only used Fireball and Fire Blast"]
fn follower_kit_meredy32_combustion() {
    let (mut h, unit) = level32(209059);
    uses_spell(&mut h, unit, 190319);
}

#[test]
#[ignore = "RED: Austin previously lacked a self-heal priority"]
fn follower_kit_austin32_exhilaration() {
    let (mut h, unit) = level32(209065);
    wounded(&mut h, unit, 0.2);
    uses_spell(&mut h, unit, 109304);
}

#[test]
#[ignore = "RED: Kybel previously lacked a party-heal priority"]
fn follower_kit_kybel32_verdant_embrace() {
    let (mut h, unit) = level32(215240);
    wounded(&mut h, unit, 0.2);
    uses_spell(&mut h, unit, 360995);
}

#[test]
#[ignore = "RED: Bo previously lacked Blackout Kick"]
fn follower_kit_bo32_blackout_kick() {
    let (mut h, unit) = level32(227734);
    uses_spell(&mut h, unit, 100784);
}

#[test]
#[ignore = "RED: Ralia previously lacked Ferocious Bite"]
fn follower_kit_ralia32_ferocious_bite() {
    let (mut h, unit) = level32(232800);
    uses_spell(&mut h, unit, 22568);
}

#[test]
#[ignore = "RED: Lantha previously lacked Dreadstalkers priority"]
fn follower_kit_lantha32_dreadstalkers() {
    let prepared = crate::spell_cast::test_harness::prepare_class_world();
    let data = prepared.world_data();
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let pets = crate::pet_control::load_pet_levelup_spells(&db).unwrap();
    let (mut h, unit) = level32_with_plugins(230861, |app| {
        app.insert_resource(data)
            .insert_resource(pets)
            .add_plugins((
                crate::hunter_pet::HunterPetPlugin,
                crate::pet_control::PetControlPlugin,
            ));
    });
    uses_spell(&mut h, unit, 104316);
}

#[test]
#[ignore = "RED: Garrick previously lacked Word of Glory"]
fn follower_kit_garrick32_word_of_glory() {
    let (mut h, unit) = level32(209057);
    let power = shared::components::PowerType::HolyPower;
    let cost = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(85673)
        .unwrap()
        .power_costs
        .iter()
        .find(|cost| cost.power_type == power as i32)
        .unwrap()
        .mana_cost as f32;
    for _ in 0..600 {
        if crate::spell_cast::powers::power_value(h.app.world(), unit, power)
            .unwrap()
            .0
            >= cost
        {
            break;
        }
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        crate::spell_cast::powers::power_value(h.app.world(), unit, power)
            .unwrap()
            .0
            >= cost,
        "native attacks must earn WoG's sourced cost, never a fixture power grant"
    );
    wounded(&mut h, unit, 0.3);
    uses_spell(&mut h, unit, 85673);
}

#[test]
#[ignore = "RED: urgent cast restarts every AI tick instead of completing"]
fn follower_kit_garrick32_finishes_flash_of_light() {
    let (mut h, unit) = level32(209057);
    wounded(&mut h, unit, 0.3);
    let before = h.health(unit);
    uses_spell(&mut h, unit, 19750);
    assert!(h.health(unit) > before);
}

#[test]
#[ignore = "RED: Garrick previously lacked active mitigation"]
fn follower_kit_garrick32_shield_of_the_righteous() {
    let (mut h, unit) = level32(209057);
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.npc)
        .unwrap()
        .x = 2.0;
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::networking::CombatTarget(Some(unit)));
    uses_spell(&mut h, unit, 53600);
    assert!(crate::auras::has_aura(h.app.world(), unit, 132403));
}

#[test]
#[ignore = "RED: Garrick previously lacked Consecration"]
fn follower_kit_garrick32_consecration() {
    let (mut h, unit) = level32(209057);
    uses_spell(&mut h, unit, 26573);
    assert!(
        !crate::spell_cast::scripts::owned_ground_fields(h.app.world_mut(), unit, 26573).is_empty()
    );
}

#[test]
#[ignore = "RED: Crenna previously lacked Swiftmend"]
fn follower_kit_crenna32_swiftmend() {
    let (mut h, unit) = level32(209072);
    let patient = h.caster;
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 774, Some(patient), false).unwrap();
    h.advance(35);
    wounded(&mut h, patient, 0.3);
    uses_spell(&mut h, unit, 18562);
}

#[test]
#[ignore = "RED: Crenna previously lacked Ironbark"]
fn follower_kit_crenna32_ironbark() {
    let (mut h, unit) = level32(209072);
    let patient = h.caster;
    wounded(&mut h, patient, 0.45);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::networking::CombatTarget(Some(patient)));
    uses_spell(&mut h, unit, 102342);
}

#[test]
#[ignore = "RED: Crenna previously lacked Lifebloom"]
fn follower_kit_crenna32_lifebloom_on_tank() {
    let (mut h, unit) = level32(209072);
    let profile = h
        .app
        .world()
        .resource::<FollowerCatalog>()
        .profiles
        .iter()
        .find(|profile| profile.entry == 209057)
        .unwrap()
        .clone();
    let tank = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[209057]);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    spawning::initialize(h.app.world_mut(), h.caster, tank, profile).unwrap();
    wounded(&mut h, tank, 0.85);
    uses_spell(&mut h, unit, 33763);
    assert!(crate::auras::has_aura(h.app.world(), tank, 33763));
}

#[test]
fn follower_kit_ground_coverage_tracks_enemies_not_stationary_caster() {
    let (mut h, unit) = level32(209057);
    let entry = h
        .app
        .world()
        .resource::<FollowerCatalog>()
        .kits
        .iter()
        .find(|entry| entry.spell_id == 26573 && entry.available(66, 32))
        .unwrap()
        .clone();
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 26573, Some(unit), false).unwrap();
    h.advance(3);
    let enemy = h.npc;
    let enemies = [(enemy, Some(unit))];
    assert!(
        !super::kit_area::allows(h.app.world_mut(), unit, unit, &enemies, &entry),
        "enemy remains in original field"
    );
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(unit)
        .unwrap()
        .x += 30.0;
    assert!(
        !super::kit_area::allows(h.app.world_mut(), unit, unit, &enemies, &entry),
        "caster moving alone does not invalidate field coverage"
    );
    let position = *h
        .app
        .world()
        .get::<shared::components::Position>(unit)
        .unwrap();
    h.app.world_mut().entity_mut(enemy).insert(position);
    assert!(
        super::kit_area::allows(h.app.world_mut(), unit, unit, &enemies, &entry),
        "enemy moved to new fight position outside old field"
    );
    let fresh = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(fresh).insert(position);
    assert!(
        !super::kit_area::allows(h.app.world_mut(), unit, unit, &enemies, &entry),
        "new pack in area must not be pulled implicitly"
    );
}

#[test]
fn follower_kit_low_level_does_not_use_lay_on_hands_or_legacy_cooldown() {
    let (mut h, unit) = at_level(209057, 1, |_| {});
    wounded(&mut h, unit, 0.15);
    for _ in 0..80 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && [633, 31935, 85673].contains(&go.spell_id))
    );
}
