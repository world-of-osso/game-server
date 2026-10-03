//! Real-DB cast tests (data/world.db): a level 10 Human hunter tames
//! Diseased Young Wolf 299, fails on Hogger 448, and calls and dismisses the stored pet,
//! with the stable in a redb file that a second app reloads.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use shared::components::{ModelDisplay, Npc, Position, UnitFactionTemplate, UnitLevel};
use shared::spell_data::CastFailReason;

use super::*;
use crate::combat::{CorpseDelay, DeathState, RespawnDelay};
use crate::persistence::{self, GameDatabase};
use crate::spell_cast::test_harness::{Harness, harness_with_plugins};
use crate::world_data::{CreatureTemplate, load_expected_stats, load_templates};

const TAME_BEAST_TRIGGER: u32 = 13_481;
const CALL_PET_1: u32 = 883;
const CALL_PET_2: u32 = 83_242;
const DISMISS_PET: u32 = 2641;
/// Diseased Young Wolf: a tameable Beast (family 1, Wolf) near Northshire.
const DISEASED_YOUNG_WOLF: u32 = 299;
/// Hogger: Humanoid.
const HOGGER: u32 = 448;
/// The harness connection's `InWorld` character.
const CHARACTER_ID: u64 = 1;
/// A wolf display of template 299.
const WOLF_DISPLAY: u32 = 1_413;
/// Frames of 50 ms covering the 6 s Tame Beast channel.
const CHANNEL_FRAMES: usize = 6_000 / 50 + 4;

/// Templates 299 and 448 of world.db.
fn templates() -> Option<HashMap<u32, CreatureTemplate>> {
    let path = crate::test_data::world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut templates = load_templates(&conn, &load_expected_stats(&conn));
    templates.retain(|entry, _| [DISEASED_YOUNG_WOLF, HOGGER].contains(entry));
    Some(templates)
}

fn open_db(path: &Path) -> GameDatabase {
    let db = redb::Database::create(path).expect("redb");
    persistence::create_tables(&db);
    GameDatabase { db }
}

fn db_path() -> PathBuf {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("pets.redb");
    std::mem::forget(dir);
    path
}

/// A level 10 hunter knowing Tame Beast, Call Pet 1-2 and Dismiss Pet, with the stable
/// database at `db`; the harness kobold becomes creature `entry` at level 10.
fn hunter(db: &Path, entry: u32) -> Option<Harness> {
    let templates = templates()?;
    let db = open_db(db);
    let mut h = harness_with_plugins(
        Vec::new(),
        &[
            TAME_BEAST,
            TAME_BEAST_TRIGGER,
            CALL_PET_1,
            CALL_PET_2,
            DISMISS_PET,
        ],
        CLASS_HUNTER,
        false,
        |app| {
            app.insert_resource(WorldData {
                templates,
                ..Default::default()
            });
            app.insert_resource(db);
            app.add_plugins((HunterPetPlugin, crate::creature_ai::CreatureAiPlugin));
        },
    )?;
    let template = &h.app.world().resource::<WorldData>().templates[&entry];
    let (name, faction) = (template.name.clone(), u32::from(template.faction));
    h.app.world_mut().entity_mut(h.npc).insert((
        Npc {
            template_id: entry,
            name,
        },
        UnitFactionTemplate(faction),
        UnitLevel(10),
        ModelDisplay {
            display_id: WOLF_DISPLAY,
        },
        RespawnDelay(120.0),
        CorpseDelay(60.0),
    ));
    h.advance(1);
    Some(h)
}

fn stable(h: &Harness) -> Option<&PetStable> {
    h.app.world().get::<PetStable>(h.caster)
}

fn stored_wolf() -> StoredPet {
    StoredPet {
        creature_entry: DISEASED_YOUNG_WOLF,
        display_id: WOLF_DISPLAY,
        name: "Diseased Young Wolf".into(),
        level: 10,
        family: 1,
    }
}

fn seed_stable(path: &Path, current: Option<u8>) {
    let db = open_db(path);
    let mut stored = StoredPetStable {
        current,
        ..Default::default()
    };
    stored.active[0] = Some(stored_wolf());
    save_pet_stable(&db.db, CHARACTER_ID, &stored);
}

fn pet_out(h: &Harness) -> Entity {
    stable(h)
        .and_then(|stable| stable.summoned)
        .expect("a pet is out")
}

fn saved(h: &Harness) -> StoredPetStable {
    let db = h.app.world().resource::<GameDatabase>();
    load_pet_stable(&db.db, CHARACTER_ID).expect("stable saved")
}

#[test]
fn hunter_tames_a_diseased_young_wolf_after_the_channel() {
    let path = db_path();
    let Some(mut h) = hunter(&path, DISEASED_YOUNG_WOLF) else {
        return;
    };
    let wolf = h.npc;
    h.cast_and_deliver(TAME_BEAST, Some(wolf));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(stable(&h).is_none(), "tamed before the channel ended");

    h.advance(CHANNEL_FRAMES);

    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app.world().get::<DeathState>(wolf).is_some(),
        "wolf not despawned"
    );
    let pet = pet_out(&h);
    let world = h.app.world();
    assert_eq!(
        world.get::<HunterPet>(pet),
        Some(&HunterPet {
            owner: h.caster,
            slot: 0
        })
    );
    assert_eq!(
        world.get::<Npc>(pet).unwrap().template_id,
        DISEASED_YOUNG_WOLF
    );
    assert_eq!(world.get::<UnitLevel>(pet), Some(&UnitLevel(10)));
    assert_eq!(
        world.get::<UnitFactionTemplate>(pet),
        Some(&UnitFactionTemplate(1))
    );
    assert_eq!(
        world.get::<ModelDisplay>(pet).unwrap().display_id,
        WOLF_DISPLAY
    );
    let expected = StoredPetStable {
        current: Some(0),
        active: [Some(stored_wolf()), None, None, None, None],
    };
    assert_eq!(stable(&h).unwrap().stored, expected);
    assert_eq!(saved(&h), expected);
}

#[test]
fn hogger_cannot_be_tamed() {
    let path = db_path();
    let Some(mut h) = hunter(&path, HOGGER) else {
        return;
    };
    h.cast_and_deliver(TAME_BEAST, Some(h.npc));
    let failed = h.inbox.failed.first().expect("Tame Beast refused");
    assert_eq!(failed.spell_id, TAME_BEAST);
    assert_eq!(failed.reason, CastFailReason::InvalidTarget);
    h.advance(CHANNEL_FRAMES);
    assert!(stable(&h).is_none());
    assert!(h.app.world().get::<DeathState>(h.npc).is_none());
}

#[test]
fn a_tamed_pet_survives_a_server_reload() {
    let path = db_path();
    {
        let Some(mut h) = hunter(&path, DISEASED_YOUNG_WOLF) else {
            return;
        };
        h.cast_and_deliver(TAME_BEAST, Some(h.npc));
        h.advance(CHANNEL_FRAMES);
        pet_out(&h);
    }
    let mut h = hunter(&path, HOGGER).expect("world.db present");
    h.advance(1);
    let pet = pet_out(&h);
    assert_eq!(
        h.app.world().get::<HunterPet>(pet),
        Some(&HunterPet {
            owner: h.caster,
            slot: 0
        })
    );
    assert_eq!(
        h.app.world().get::<Npc>(pet).unwrap().name,
        "Diseased Young Wolf"
    );
}

#[test]
fn dismissed_pet_stays_stored_and_call_pet_brings_it_back() {
    let path = db_path();
    seed_stable(&path, Some(0));
    let Some(mut h) = hunter(&path, HOGGER) else {
        return;
    };
    let first = pet_out(&h);

    // Dismiss Pet: a 3 s cast.
    h.cast_and_deliver(DISMISS_PET, None);
    h.advance(3_000 / 50 + 2);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.app.world().get_entity(first).is_err(), "pet still out");
    let dismissed = StoredPetStable {
        current: None,
        active: [Some(stored_wolf()), None, None, None, None],
    };
    assert_eq!(stable(&h).unwrap().stored, dismissed);
    assert_eq!(saved(&h), dismissed);

    h.cast_and_deliver(CALL_PET_1, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let pet = pet_out(&h);
    assert_eq!(saved(&h).current, Some(0));
    let at = h.app.world().get::<Position>(pet).unwrap();
    let owner = h.app.world().get::<Position>(h.caster).unwrap();
    assert!(
        (at.x - owner.x).hypot(at.z - owner.z) <= 1.6,
        "{at:?} far from {owner:?}"
    );
}

#[test]
fn call_pet_of_an_empty_slot_has_no_pet_to_summon() {
    let path = db_path();
    seed_stable(&path, None);
    let Some(mut h) = hunter(&path, HOGGER) else {
        return;
    };
    h.cast_and_deliver(CALL_PET_2, None);
    let failed = h.inbox.failed.first().expect("Call Pet 2 refused");
    assert_eq!(
        failed.detail.as_deref(),
        Some("You do not have a pet to summon")
    );
    assert!(stable(&h).unwrap().summoned.is_none());
}

#[test]
fn dismiss_pet_without_a_pet_out_fails() {
    let path = db_path();
    seed_stable(&path, None);
    let Some(mut h) = hunter(&path, HOGGER) else {
        return;
    };
    h.cast_and_deliver(DISMISS_PET, None);
    let failed = h.inbox.failed.first().expect("Dismiss Pet refused");
    assert_eq!(failed.detail.as_deref(), Some("You do not have a pet."));
}

#[test]
fn a_summoned_pet_runs_after_its_owner() {
    let path = db_path();
    seed_stable(&path, Some(0));
    let Some(mut h) = hunter(&path, HOGGER) else {
        return;
    };
    let pet = pet_out(&h);
    h.app.world_mut().entity_mut(h.caster).insert(Position {
        x: 20.0,
        y: 0.0,
        z: 0.0,
    });
    // 20 yards at 7-9.1 yards per second.
    h.advance(80);
    let at = *h.app.world().get::<Position>(pet).unwrap();
    let gap = (20.0 - at.x).hypot(at.z);
    assert!(gap <= 4.5, "pet {gap} yards from its owner at {at:?}");
}
