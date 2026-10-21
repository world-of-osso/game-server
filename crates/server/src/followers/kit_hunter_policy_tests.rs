//! FOLLOWER_HUNTER_PET_SIMC_DEFAULT: real native125 Waterfowl, ordinary Call Pet.
use super::kit_tests::at_level;
use crate::hunter_pet::{HunterPet, HunterPetPlugin, PetStable, summoned_pet};
use shared::components::{Level, Npc};

const AUSTIN: u32 = 209065;
const NATIVE_DUCK: u32 = 174884;
const WATERFOWL: u32 = 125;

fn check_native_duck(level: u8) {
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
    assert_eq!(h.app.world().get::<Level>(pet).unwrap().0, level);
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
}

#[test]
fn rotation_fidelity_253_level90_prepares_native_simc_default_duck_via_call_pet() {
    check_native_duck(90);
}

#[test]
fn rotation_fidelity_253_level32_prepares_native_simc_default_duck_via_call_pet() {
    check_native_duck(32);
}
