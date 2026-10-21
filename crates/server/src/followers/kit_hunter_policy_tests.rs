//! FOLLOWER_HUNTER_PET_SIMC_DEFAULT: real native125 Waterfowl, ordinary Call Pet.
use super::kit_tests::at_level;
use crate::hunter_pet::{HunterPet, HunterPetPlugin, PetStable, summoned_pet};
use shared::components::{Npc, UnitLevel};

const AUSTIN: u32 = 209065;
const NATIVE_DUCK: u32 = 174884;
const WATERFOWL: u32 = 125;

fn prepare_native_duck(
    level: u8,
) -> (
    crate::spell_cast::test_harness::Harness,
    bevy::prelude::Entity,
    bevy::prelude::Entity,
) {
    let prepared = crate::spell_cast::test_harness::prepare_class_world();
    let data = prepared.world_data();
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("native-follower-duck.redb")).unwrap();
    crate::persistence::create_tables(&db);
    let (mut h, caster) = at_level(AUSTIN, level, |app| {
        app.insert_resource(crate::persistence::GameDatabase::new(db));
        app.insert_resource(data);
        app.add_plugins((
            HunterPetPlugin,
            crate::pet_control::PetControlPlugin,
            crate::creature_ai::CreatureAiPlugin,
        ));
    });
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::pet_control::load_pet_levelup_spells(&source).unwrap());
    assert!(
        h.app
            .world()
            .get::<shared::components::Player>(caster)
            .is_none()
    );
    for _ in 0..200 {
        super::update_ai(h.app.world_mut());
        h.advance(1);
        if summoned_pet(h.app.world(), caster).is_some() {
            break;
        }
    }
    let pet = summoned_pet(h.app.world(), caster)
        .expect("BM follower must prepare its policy-selected real native pet");
    assert_eq!(h.app.world().get::<HunterPet>(pet).unwrap().owner, caster);
    assert_eq!(
        h.app.world().get::<Npc>(pet).unwrap().template_id,
        NATIVE_DUCK
    );
    assert_eq!(h.app.world().get::<UnitLevel>(pet).unwrap().0, level);
    let stable = h.app.world().get::<PetStable>(caster).unwrap();
    assert_eq!(
        stable.character_id, None,
        "runtime pet must never invent persistent player identity"
    );
    let stored = stable.stored.active[0].as_ref().unwrap();
    assert_eq!(stored.family, WATERFOWL);
    assert!(
        stored.specialization.is_some(),
        "use the native family specialization, not bespoke follower pet stats"
    );
    assert!(
        h.inbox
            .go
            .iter()
            .any(|event| event.caster == caster.to_bits() && event.spell_id == 883),
        "pet acquisition must use ordinary learned Call Pet"
    );
    (h, caster, pet)
}

fn check_native_duck(level: u8) {
    prepare_native_duck(level);
}

#[test]
fn rotation_fidelity_253_level90_prepares_native_simc_default_duck_via_call_pet() {
    check_native_duck(90);
}

#[test]
fn rotation_fidelity_253_level32_prepares_native_simc_default_duck_via_call_pet() {
    check_native_duck(32);
}

/// Observe two completed swing clocks; misses also count as real attack attempts.
fn melee_swing_frames(
    h: &mut crate::spell_cast::test_harness::Harness,
    unit: bevy::prelude::Entity,
) -> usize {
    let start = h
        .app
        .world()
        .resource::<crate::combat::CombatLog>()
        .range(0.0, f64::MAX)
        .len();
    let mut first = None;
    let mut observed = 0;
    for frame in 0usize..200 {
        h.advance(1);
        let log = h.app.world().resource::<crate::combat::CombatLog>();
        let count = log
            .range(0.0, f64::MAX)
            .iter()
            .skip(start)
            .filter(|entry| entry.event.attacker == unit.to_bits() && entry.event.spell_id == 0)
            .count();
        if count > observed {
            if let Some(first) = first {
                return frame - first;
            }
            first = Some(frame);
            observed = count;
        }
    }
    panic!("native unit{unit} did not produce two melee attempts");
}

#[test]
fn rotation_fidelity_253_missing_duck_data_still_shoots() {
    let (mut h, caster) = at_level(AUSTIN, 32, |_| {});
    assert!(summoned_pet(h.app.world(), caster).is_none());
    super::kit_tests::uses_spell(&mut h, caster, 217200);
    assert!(summoned_pet(h.app.world(), caster).is_none());
}

#[test]
#[ignore = "RED: native1264934 Frenzy has no active-pet attack-speed consumer"]
fn rotation_fidelity_253_native_frenzy_shortens_pet_melee_interval() {
    const FRENZY: u32 = 1_264_934;
    let (mut h, caster, pet) = prepare_native_duck(90);
    assert!(crate::auras::has_aura(h.app.world(), caster, FRENZY));
    let amount = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(FRENZY)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0 && effect.aura == 4)
        .unwrap()
        .base_points;
    let target = h.npc;
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(target)
        .unwrap()
        .max = 1_000_000_000.0;
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(target)
        .unwrap()
        .current = 1_000_000_000.0;
    let position = *h
        .app
        .world()
        .get::<shared::components::Position>(target)
        .unwrap();
    h.app
        .world_mut()
        .entity_mut(pet)
        .insert((position, crate::combat::AutoAttack::new(target)));
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::combat::AutoAttack::new(target));
    let owner_with_frenzy = melee_swing_frames(&mut h, caster);
    let accelerated = melee_swing_frames(&mut h, pet);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == FRENZY,
    );
    let ordinary = melee_swing_frames(&mut h, pet);
    assert_eq!(
        melee_swing_frames(&mut h, caster),
        owner_with_frenzy,
        "owner Frenzy must modify only the pet's attack clock"
    );
    let expected = ordinary as f32 / (1.0 + amount / 100.0);
    assert!(
        (accelerated as f32 - expected).abs() <= 1.0,
        "Frenzy native{amount}% must shorten pet interval: {accelerated} vs ordinary{ordinary}, expected{expected}"
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), pet, FRENZY),
        "owner passive must not become a synthetic pet buff"
    );
}
