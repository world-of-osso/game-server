//! Real-DB cast tests (data/world.db): a level 10 Human hunter tames
//! Diseased Young Wolf 299, fails on Hogger 448, and calls and dismisses the stored pet,
//! with the stable in a redb file that a second app reloads.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use shared::components::{ModelDisplay, Npc, Position, UnitFactionTemplate, UnitLevel};
use shared::protocol::{
    InteractNpc, InteractionChannel, InteractionKind, InteractionOpened, NpcRole, PetStableChannel,
    PetStableContents, PetStableError, PetStableFailed, PetStableOperation, PetStableRequest,
    SelectGossipOption,
};
use shared::spell_data::CastFailReason;

use super::*;
use crate::combat::{CorpseDelay, DeathState, RespawnDelay};
use crate::persistence::{self, GameDatabase};
use crate::spell_cast::test_harness::{Harness, harness_with_plugins};
use crate::world_data::{CreatureTemplate, load_expected_stats, load_templates};

const TAME_BEAST_TRIGGER: u32 = 13_481;
const CALL_PET_1: u32 = 883;
const CALL_PET_2: u32 = 83_242;
const CALL_PET_3: u32 = 83_243;
const CALL_PET_4: u32 = 83_244;
const CALL_PET_5: u32 = 83_245;
const DISMISS_PET: u32 = 2641;
const REVIVE_PET: u32 = 982;
const MEND_PET: u32 = 136;
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
    GameDatabase::new(db)
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
            CALL_PET_3,
            CALL_PET_4,
            CALL_PET_5,
            DISMISS_PET,
            REVIVE_PET,
            MEND_PET,
        ],
        CLASS_HUNTER,
        false,
        |app| {
            app.insert_resource(WorldData {
                templates,
                ..Default::default()
            });
            app.insert_resource(db);
            app.insert_resource(
                crate::faction::world_db_faction_templates().expect("real faction metadata"),
            );
            let conn = Connection::open_with_flags(
                crate::test_data::world_db_path(),
                OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            app.insert_resource(crate::npc_interaction::load_gossip_catalog(&conn).unwrap());
            app.init_resource::<StableInbox>()
                .add_systems(First, collect_stable_inbox);
            app.add_plugins((
                HunterPetPlugin,
                crate::creature_ai::CreatureAiPlugin,
                crate::npc_interaction::NpcInteractionPlugin,
            ));
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
        crate::maps::InMap::continent(0),
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
        react_state: shared::protocol::REACT_PASSIVE,
        health: Some(378.0),
        specialization: Some(74),
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
        ..Default::default()
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
        ..Default::default()
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
fn call_pet_without_character_pet_storage_fails_before_success_or_spawn() {
    let path = db_path();
    let mut h = hunter(&path, HOGGER).expect("real hunter pet data");
    assert!(
        stable(&h).is_none(),
        "fresh hunter has no loaded pet stable"
    );
    assert!(
        load_pet_stable(&h.app.world().resource::<GameDatabase>().db, CHARACTER_ID).is_none(),
        "fresh character has no tamed or stabled pet record"
    );

    // Same native bot request: Call Pet 1 with the hostile selected target.
    h.cast_and_deliver(CALL_PET_1, Some(h.npc));
    let failed = h
        .inbox
        .failed
        .first()
        .expect("Call Pet 1 must refuse absent storage");
    assert_eq!(
        failed.detail.as_deref(),
        Some("You do not have a pet to summon")
    );
    assert!(h.inbox.log.iter().all(|event| {
        event.spell_id != Some(CALL_PET_1)
            || event.kind != shared::protocol::CombatLogKind::CastSuccess
    }));
    assert!(stable(&h).is_none());
    assert_eq!(
        h.app
            .world_mut()
            .query::<&HunterPet>()
            .iter(h.app.world())
            .count(),
        0
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
fn pet_lifecycle_each_authored_call_spell_loads_its_owned_active_slot() {
    for (slot, spell) in [CALL_PET_1, CALL_PET_2, CALL_PET_3, CALL_PET_4, CALL_PET_5]
        .into_iter()
        .enumerate()
    {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("call-pets.redb");
        {
            let db = open_db(&path);
            let mut stable = StoredPetStable::default();
            let mut record = stored_wolf();
            record.name = format!("Wolf{}", slot + 1);
            stable.active[slot] = Some(record);
            save_pet_stable(&db.db, CHARACTER_ID, &stable);
        }
        let mut h = hunter(&path, HOGGER).expect("real pet data");
        h.app.world_mut().entity_mut(h.caster).insert(Level(60));
        h.cast_and_deliver(spell, None);
        assert!(
            h.inbox.failed.is_empty(),
            "spell{spell}: {:?}",
            h.inbox.failed
        );
        let pet = pet_out(&h);
        assert_eq!(
            h.app.world().get::<HunterPet>(pet).unwrap().slot,
            slot as u8
        );
        assert_eq!(
            h.app.world().get::<Npc>(pet).unwrap().name,
            format!("Wolf{}", slot + 1)
        );
        assert_eq!(saved(&h).current, Some(slot as u8));
    }
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

#[derive(Resource, Default)]
struct StableInbox {
    opened: Vec<InteractionOpened>,
    contents: Vec<PetStableContents>,
    failed: Vec<PetStableFailed>,
}

// HostClient replies must be consumed in First, before Lightyear clears them in Last.
fn collect_stable_inbox(
    mut opened: Query<&mut MessageReceiver<InteractionOpened>>,
    mut contents: Query<&mut MessageReceiver<PetStableContents>>,
    mut failed: Query<&mut MessageReceiver<PetStableFailed>>,
    mut inbox: ResMut<StableInbox>,
) {
    for mut receiver in &mut opened {
        inbox.opened.extend(receiver.receive());
    }
    for mut receiver in &mut contents {
        inbox.contents.extend(receiver.receive());
    }
    for mut receiver in &mut failed {
        inbox.failed.extend(receiver.receive());
    }
}

fn stable_master_hunter(path: &Path) -> Harness {
    let mut h = hunter(path, HOGGER).expect("real stable hunter fixture");
    let at = *h.app.world().get::<Position>(h.caster).unwrap();
    h.app.world_mut().entity_mut(h.npc).insert((
        Npc {
            template_id: 6749,
            name: "Erma".into(),
        },
        NpcFlags(NpcFlags::GOSSIP | NpcFlags::STABLEMASTER),
        UnitFactionTemplate(1),
        crate::maps::InMap::continent(0),
        at,
    ));
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<InteractNpc>::default(),
        MessageReceiver::<InteractNpc>::default(),
        MessageSender::<InteractionOpened>::default(),
        MessageReceiver::<InteractionOpened>::default(),
        MessageSender::<SelectGossipOption>::default(),
        MessageReceiver::<SelectGossipOption>::default(),
        MessageSender::<PetStableRequest>::default(),
        MessageReceiver::<PetStableRequest>::default(),
        MessageSender::<PetStableContents>::default(),
        MessageReceiver::<PetStableContents>::default(),
        MessageSender::<PetStableFailed>::default(),
        MessageReceiver::<PetStableFailed>::default(),
    ));
    h.app
        .world_mut()
        .get_mut::<MessageSender<InteractNpc>>(h.connection)
        .unwrap()
        .send::<InteractionChannel>(InteractNpc {
            npc: h.npc.to_bits(),
        });
    h.advance(3);
    let opened = h
        .app
        .world_mut()
        .resource_mut::<StableInbox>()
        .opened
        .pop()
        .expect("real NPC hello response");
    if let InteractionKind::Gossip(menu) = opened.kind {
        // Authored content_gossip_menu_option9821/0 is StableMaster; other visible
        // options are battle-pet healing, not additional combat-pet services.
        let stable_option = menu
            .options
            .iter()
            .find(|option| option.text == "I'd like to stable my pet here.")
            .expect("Erma's authored combat-pet stable option")
            .option_id;
        h.app
            .world_mut()
            .get_mut::<MessageSender<SelectGossipOption>>(h.connection)
            .unwrap()
            .send::<InteractionChannel>(SelectGossipOption {
                npc: h.npc.to_bits(),
                option_id: stable_option,
            });
        h.advance(3);
        let opened = h
            .app
            .world_mut()
            .resource_mut::<StableInbox>()
            .opened
            .pop()
            .expect("stable gossip selection response");
        assert_eq!(opened.kind, InteractionKind::Role(NpcRole::StableMaster));
    } else {
        assert_eq!(opened.kind, InteractionKind::Role(NpcRole::StableMaster));
    }
    h
}

fn stable_request(h: &mut Harness, operation: PetStableOperation) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<PetStableRequest>>(h.connection)
        .unwrap()
        .send::<PetStableChannel>(PetStableRequest {
            npc: h.npc.to_bits(),
            operation,
        });
    h.advance(3);
}

fn stable_contents(h: &mut Harness) -> PetStableContents {
    h.app
        .world_mut()
        .resource_mut::<StableInbox>()
        .contents
        .pop()
        .expect("authoritative stable contents response")
}

#[test]
fn pet_lifecycle_stable_move_rename_spec_and_reopen() {
    let path = db_path();
    seed_stable(&path, Some(0));
    let mut h = stable_master_hunter(&path);
    stable_request(&mut h, PetStableOperation::List);
    let contents = stable_contents(&mut h);
    assert_eq!(contents.current, Some(0));
    assert_eq!(contents.pets[0].family, "Wolf");
    assert_eq!(contents.pets[0].specialization, Some(74));
    let pet = pet_out(&h);
    h.app.world_mut().get_mut::<Health>(pet).unwrap().current = 23.0;
    stable_request(&mut h, PetStableOperation::Swap { from: 0, to: 204 });
    let contents = stable_contents(&mut h);
    assert_eq!(contents.current, None);
    assert_eq!(contents.pets[0].slot, 204);
    assert_eq!(contents.pets[0].health, 23.0);
    assert!(h.app.world().get_entity(pet).is_err());
    stable_request(
        &mut h,
        PetStableOperation::Rename {
            slot: 204,
            name: "Ash".into(),
        },
    );
    assert_eq!(stable_contents(&mut h).pets[0].name, "Ash");
    stable_request(
        &mut h,
        PetStableOperation::Specialization {
            slot: 204,
            specialization: 79,
        },
    );
    assert_eq!(stable_contents(&mut h).pets[0].specialization, Some(79));
    drop(h);

    let mut reloaded = stable_master_hunter(&path);
    stable_request(&mut reloaded, PetStableOperation::List);
    let contents = stable_contents(&mut reloaded);
    assert_eq!(contents.current, None);
    assert_eq!(contents.pets[0].name, "Ash");
    assert_eq!(contents.pets[0].health, 23.0);
    assert_eq!(contents.pets[0].specialization, Some(79));
    stable_request(&mut reloaded, PetStableOperation::Swap { from: 204, to: 0 });
    assert_eq!(stable_contents(&mut reloaded).pets[0].slot, 0);
    reloaded.cast_and_deliver(CALL_PET_1, None);
    assert_eq!(
        reloaded
            .app
            .world()
            .get::<Health>(pet_out(&reloaded))
            .unwrap()
            .current,
        23.0
    );
    assert_eq!(
        reloaded
            .app
            .world()
            .get::<Npc>(pet_out(&reloaded))
            .unwrap()
            .name,
        "Ash"
    );
    stable_request(&mut reloaded, PetStableOperation::Abandon { slot: 0 });
    assert!(stable_contents(&mut reloaded).pets.is_empty());
    drop(reloaded);
    let last = hunter(&path, HOGGER).expect("reopened abandoned pet fixture");
    assert!(saved(&last).active[0].is_none());
    assert!(saved(&last).stabled.iter().all(Option::is_none));
}

#[test]
fn pet_lifecycle_stable_swaps_active_slots_without_replacing_owned_pet() {
    let path = db_path();
    seed_stable(&path, Some(0));
    let db = open_db(&path);
    let mut stored = load_pet_stable(&db.db, CHARACTER_ID).unwrap();
    let mut second = stored_wolf();
    second.name = "Ember".into();
    stored.active[1] = Some(second);
    save_pet_stable(&db.db, CHARACTER_ID, &stored);
    drop(db);
    let mut h = stable_master_hunter(&path);
    let pet = pet_out(&h);
    stable_request(&mut h, PetStableOperation::Swap { from: 0, to: 0 });
    assert_eq!(stable_contents(&mut h).pets.len(), 2);
    stable_request(&mut h, PetStableOperation::Swap { from: 0, to: 1 });
    assert_eq!(stable_contents(&mut h).current, Some(1));
    assert_eq!(pet_out(&h), pet);
    assert_eq!(h.app.world().get::<HunterPet>(pet).unwrap().slot, 1);
    stable_request(
        &mut h,
        PetStableOperation::Rename {
            slot: 1,
            name: "Ash".into(),
        },
    );
    assert_eq!(stable_contents(&mut h).pets[1].name, "Ash");
    assert_eq!(h.app.world().get::<Npc>(pet).unwrap().name, "Ash");
    stable_request(&mut h, PetStableOperation::Abandon { slot: 0 });
    assert_eq!(stable_contents(&mut h).pets.len(), 1);
    assert_eq!(pet_out(&h), pet);
}

#[test]
fn pet_lifecycle_stable_refusals_preserve_records() {
    let path = db_path();
    seed_stable(&path, Some(0));
    let mut h = stable_master_hunter(&path);
    let known: Vec<_> = h
        .app
        .world()
        .get::<KnownSpells>(h.caster)
        .unwrap()
        .iter()
        .filter(|spell| *spell != CALL_PET_5)
        .collect();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(KnownSpells::fixture(known.clone()));
    let before = saved(&h);
    for (operation, error) in [
        (
            PetStableOperation::Swap { from: 0, to: 205 },
            PetStableError::InvalidSlot,
        ),
        (
            PetStableOperation::Swap { from: 0, to: 4 },
            PetStableError::SlotLocked,
        ),
        (
            PetStableOperation::Rename {
                slot: 0,
                name: "8Bad".into(),
            },
            PetStableError::InvalidName,
        ),
        (
            PetStableOperation::Specialization {
                slot: 0,
                specialization: 999,
            },
            PetStableError::InvalidSpecialization,
        ),
    ] {
        stable_request(&mut h, operation);
        let failure = h
            .app
            .world_mut()
            .resource_mut::<StableInbox>()
            .failed
            .pop()
            .expect("stable refusal response");
        assert_eq!(failure.error, error);
        assert_eq!(saved(&h), before);
    }
    // The same slot accepts the swap once its authored Call Pet spell is learned.
    let mut unlocked = known;
    unlocked.push(CALL_PET_5);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(KnownSpells::fixture(unlocked));
    stable_request(&mut h, PetStableOperation::Swap { from: 0, to: 4 });
    let contents = stable_contents(&mut h);
    assert_eq!(contents.current, Some(4));
    assert_eq!(contents.pets[0].slot, 4);
    assert!(h.app.world().resource::<StableInbox>().failed.is_empty());
}

#[test]
fn pet_lifecycle_stable_rejects_closed_interaction_without_mutation() {
    let path = db_path();
    seed_stable(&path, Some(0));
    let mut h = stable_master_hunter(&path);
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .remove::<crate::npc_interaction::ActiveInteraction>();
    stable_request(&mut h, PetStableOperation::Abandon { slot: 0 });
    let failures = &h.app.world().resource::<StableInbox>().failed;
    assert_eq!(
        failures.last().map(|failure| failure.error),
        Some(PetStableError::NotInteracting)
    );
    assert!(saved(&h).active[0].is_some());
}

#[test]
fn pet_lifecycle_revive_restores_lethally_damaged_owned_pet() {
    let path = db_path();
    seed_stable(&path, Some(0));
    let mut h = hunter(&path, HOGGER).expect("real revive fixture");
    let pet = pet_out(&h);
    let max_health = h.app.world().get::<Health>(pet).unwrap().max;
    crate::combat::deal_direct_damage(h.app.world_mut(), h.npc, pet, max_health + 1.0);
    assert_eq!(h.app.world().get::<Health>(pet).unwrap().current, 0.0);
    h.advance(3);
    h.cast_and_deliver(REVIVE_PET, None);
    h.advance(130);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let revived = pet_out(&h);
    assert_eq!(
        h.app.world().get::<Health>(revived).unwrap().current,
        max_health
    );
    assert!(h.app.world().get::<DeathState>(revived).is_none());
    assert_eq!(
        saved(&h).active[0].as_ref().unwrap().health,
        Some(max_health)
    );
    drop(h);
    let reloaded = hunter(&path, HOGGER).expect("reopened revived fixture");
    assert_eq!(
        reloaded
            .app
            .world()
            .get::<Health>(pet_out(&reloaded))
            .unwrap()
            .current,
        max_health
    );
}

#[test]
fn pet_lifecycle_revive_summons_first_dead_active_slot_not_a_living_one() {
    let path = db_path();
    seed_stable(&path, None);
    let db = open_db(&path);
    let mut stored = load_pet_stable(&db.db, CHARACTER_ID).unwrap();
    let mut dead = stored_wolf();
    dead.name = "Ember".into();
    dead.health = Some(0.0);
    stored.active[1] = Some(dead);
    save_pet_stable(&db.db, CHARACTER_ID, &stored);
    drop(db);
    let mut h = hunter(&path, HOGGER).expect("dismissed dead-slot revive fixture");
    h.cast_and_deliver(REVIVE_PET, None);
    h.advance(130);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let pet = pet_out(&h);
    assert_eq!(h.app.world().get::<HunterPet>(pet).unwrap().slot, 1);
    assert_eq!(h.app.world().get::<Npc>(pet).unwrap().name, "Ember");
    assert_eq!(
        h.app.world().get::<Health>(pet).unwrap().current,
        h.app.world().get::<Health>(pet).unwrap().max
    );
    assert_eq!(saved(&h).current, Some(1));
    assert_eq!(
        saved(&h).active[0].as_ref().unwrap().name,
        "Diseased Young Wolf"
    );
}

#[test]
fn pet_lifecycle_revive_refuses_living_pet_before_cast() {
    let path = db_path();
    seed_stable(&path, Some(0));
    let mut h = hunter(&path, HOGGER).expect("living-pet revive fixture");
    let pet = pet_out(&h);
    h.cast_and_deliver(REVIVE_PET, None);
    assert_eq!(
        h.inbox.failed.first().map(|failure| failure.reason),
        Some(CastFailReason::NotSupported)
    );
    assert_eq!(pet_out(&h), pet);
}

#[test]
fn pet_lifecycle_mend_heals_owned_pet_by_authored_periodic_percentage() {
    let path = db_path();
    seed_stable(&path, Some(0));
    let mut h = hunter(&path, HOGGER).expect("real Mend fixture");
    let pet = pet_out(&h);
    let max_health = h.app.world().get::<Health>(pet).unwrap().max;
    h.app.world_mut().get_mut::<Health>(pet).unwrap().current = 50.0;
    h.cast_and_deliver(MEND_PET, None);
    h.advance(45);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    // Local SpellEffect136: aura20, 10% of max health every2s, whole-HP healing.
    let healed = (max_health * 0.1).round();
    assert_eq!(
        h.app.world().get::<Health>(pet).unwrap().current,
        50.0 + healed
    );
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(MEND_PET)
                && event.target == Some(pet.to_bits())
                && event.periodic
                && event.amount == healed as i32)
    );
}

#[test]
fn pet_lifecycle_wounded_health_survives_reopened_database() {
    let path = db_path();
    seed_stable(&path, Some(0));
    let mut first = hunter(&path, HOGGER).expect("real hunter fixture");
    let pet = pet_out(&first);
    first
        .app
        .world_mut()
        .get_mut::<Health>(pet)
        .unwrap()
        .current = 23.0;
    first.advance(3);
    drop(first);

    let second = hunter(&path, HOGGER).expect("reopened real hunter fixture");
    let pet = pet_out(&second);
    assert_eq!(second.app.world().get::<Health>(pet).unwrap().current, 23.0);
}

#[test]
fn pet_lifecycle_logout_saves_health_before_the_next_fixed_tick() {
    let path = db_path();
    seed_stable(&path, Some(0));
    let mut first = hunter(&path, HOGGER).expect("real hunter fixture");
    let pet = pet_out(&first);
    first
        .app
        .world_mut()
        .get_mut::<Health>(pet)
        .unwrap()
        .current = 17.0;
    crate::networking::queue_player_despawn(&mut first.app.world_mut().commands(), first.caster);
    first.app.world_mut().flush();
    drop(first);

    let second = hunter(&path, HOGGER).expect("reopened real hunter fixture");
    assert_eq!(
        second
            .app
            .world()
            .get::<Health>(pet_out(&second))
            .unwrap()
            .current,
        17.0
    );
}

#[test]
fn pet_lifecycle_dead_health_survives_reopened_database() {
    let path = db_path();
    seed_stable(&path, Some(0));
    let mut first = hunter(&path, HOGGER).expect("real hunter fixture");
    let pet = pet_out(&first);
    first
        .app
        .world_mut()
        .get_mut::<Health>(pet)
        .unwrap()
        .current = 0.0;
    first.advance(3);
    drop(first);

    let second = hunter(&path, HOGGER).expect("reopened real hunter fixture");
    let pet = pet_out(&second);
    assert_eq!(second.app.world().get::<Health>(pet).unwrap().current, 0.0);
}

/// A stable saved before stances were kept loads its pets Passive, as `character_pet`'s
/// `ReactState` column defaults to 0.
#[test]
fn a_stable_saved_without_stances_loads_passive_pets() {
    use crate::persistence::pet_store::{LegacyStoredPet, LegacyStoredPetStable, PET_STABLES};
    let db = open_db(&db_path());
    let mut legacy = LegacyStoredPetStable {
        current: Some(0),
        active: Default::default(),
    };
    legacy.active[0] = Some(LegacyStoredPet {
        creature_entry: DISEASED_YOUNG_WOLF,
        display_id: WOLF_DISPLAY,
        name: "Diseased Young Wolf".into(),
        level: 10,
        family: 1,
    });
    let txn = db.db.begin_write().unwrap();
    txn.open_table(PET_STABLES)
        .unwrap()
        .insert(CHARACTER_ID, bitcode::encode(&legacy).as_slice())
        .unwrap();
    txn.commit().unwrap();
    let mut expected = StoredPetStable {
        current: Some(0),
        ..Default::default()
    };
    let mut migrated_wolf = stored_wolf();
    migrated_wolf.health = None;
    migrated_wolf.specialization = None;
    expected.active[0] = Some(migrated_wolf);
    assert_eq!(load_pet_stable(&db.db, CHARACTER_ID), Some(expected));
}
