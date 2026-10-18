use super::*;
use crate::persistence::{self, InventoryData, InventoryItemData};
use crate::spell_cast::test_harness::*;
use shared::components::{CombatStatus, Level, Position};
use shared::protocol::{ChatMessage, ChatType, InventoryChannel, InventoryError, UseItem};

#[path = "toys_akunda_tests.rs"]
mod akunda;
#[path = "toys_azerite_firework_tests.rs"]
mod azerite_firework;
#[path = "toys_bashiok_tests.rs"]
mod bashiok;
#[path = "toys_breakfast_tests.rs"]
mod breakfast;
#[path = "toys_citizens_tests.rs"]
mod citizens;
#[path = "toys_disguise_tests.rs"]
mod disguises;
#[path = "toys_dragon_head_tests.rs"]
mod dragon_head;
#[path = "toys_flute_tests.rs"]
mod flute;
#[path = "toys_force_forward_tests.rs"]
mod force_forward;
#[path = "toys_forlorn_toll_tests.rs"]
mod forlorn_toll;
#[path = "toys_gnoll_barrel_tests.rs"]
mod gnoll_barrel;
#[path = "toys_group_inheritance_tests.rs"]
mod group_inheritance;
#[path = "toys_group_tests.rs"]
mod group_private;
#[path = "toys_hearthstone_tests.rs"]
mod hearthstones;
#[path = "toys_offering_kit_tests.rs"]
mod offering_kit;
#[path = "toys_oomgut_tests.rs"]
mod oomgut;
#[path = "toys_owner_flags_tests.rs"]
mod owner_flags;
#[path = "toys_podium_tests.rs"]
mod podium;
#[path = "toys_scarab_tests.rs"]
mod scarab;
#[path = "toys_shrine_tests.rs"]
mod shrine;
#[path = "toys_sightless_tests.rs"]
mod sightless;
#[path = "toys_stampwhistle_tests.rs"]
mod stampwhistle;
#[path = "toys_stone_whistle_tests.rs"]
mod stone_whistle;
#[path = "toys_summon_level_tests.rs"]
mod summon_levels;
#[path = "toys_waywatcher_tests.rs"]
mod waywatcher;
#[path = "toys_ysera_tests.rs"]
mod ysera;

const FIGURINE: u32 = 32782;
const TRANSFORM: u32 = 41301;
const FLAG_OWNERSHIP: u32 = 38578; // The Flag of Ownership: effect 3, spell 51640.

fn setup(path: &std::path::Path) -> (Harness, u64) {
    setup_for_race(path, 1)
}

fn setup_for_race(path: &std::path::Path, race: u8) -> (Harness, u64) {
    setup_for_race_class(path, race, 8)
}

fn setup_for_race_class(path: &std::path::Path, race: u8, class: u8) -> (Harness, u64) {
    let mut h = harness(
        Vec::new(),
        &[
            TRANSFORM, 51640, 61810, 45426, 176594, 176457, 185712, 302273, 98444, 141917, 156132,
            99104, 304370, 304369, 23126, 167273, 65451, 176889, 225656, 225658, 225660, 225657,
            225659, 376088, 116, 376092, 376155, 376203, 397775, 376172, 376113, 398159, 398162,
            398329, 398328, 398334, 398333, 62736, 42766, 54710, 42753, 42762, 388088, 264091,
            289277, 384585, 203533, 203820, 261981, 286277, 455023, 462145, 1269949, 1280563,
            128328, 406870, 383065, 239560, 254513, 405634, 127771, 128180, 280827, 280828, 360123,
            361085, 361090, 381262, 401253, 417591, 429364, 442240, 469612, 469613, 469614, 469616,
            469617, 469618, 128327, 196783, 318453, 318452, 243209, 362988, 302730, 193333, 408067,
            430760, 71909, 171352, 127803, 128150, 128151, 128152, 176899, 176900, 300104, 300144,
        ],
        class,
    )
    .unwrap();
    let db = redb::Database::create(path).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "ToyMage", race, class).unwrap();
    let item = |id| InventoryItemData {
        item_guid: persistence::next_id(&db, "item_instance"),
        item_id: id,
        name: String::new(),
        quality: 1,
        required_level: 1,
        stack_count: 1,
        vendor_sell_price: 1,
        unique_auction: false,
    };
    persistence::save_inventory(
        &db,
        character,
        &InventoryData {
            gold: 0,
            items: vec![item(FIGURINE), item(FLAG_OWNERSHIP), item(188698)],
        },
    );
    let path = crate::test_data::world_db_path();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app
        .insert_resource(crate::spell_cast::toy_scripts::load_toy_texts(&conn).unwrap());
    h.app.init_resource::<crate::group::GroupRegistry>();
    h.app
        .insert_resource(crate::game_objects::summoned::load_templates(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app.insert_resource(GameDatabase { db });
    h.app.init_resource::<crate::gold::GoldUpdateQueue>();
    h.app.add_systems(
        Update,
        (
            crate::inventory::process_use_requests,
            receive_favourites,
            receive_use,
        )
            .chain(),
    );
    h.app.add_systems(PostUpdate, send_initial);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((CombatStatus(false), Level(80)));
    h.app
        .world_mut()
        .get_mut::<shared::components::Player>(h.caster)
        .unwrap()
        .race = race;
    h.app.world_mut().entity_mut(h.connection).insert((
        InWorld {
            character_id: character,
        },
        MessageSender::<UseItem>::default(),
        MessageReceiver::<UseItem>::default(),
        MessageSender::<InventoryError>::default(),
        MessageReceiver::<InventoryError>::default(),
        MessageSender::<UseToy>::default(),
        MessageReceiver::<UseToy>::default(),
        MessageSender::<SetToyFavourite>::default(),
        MessageReceiver::<SetToyFavourite>::default(),
        MessageSender::<ToyResult>::default(),
        MessageReceiver::<ToyResult>::default(),
        MessageSender::<ToyCollectionUpdate>::default(),
        MessageReceiver::<ToyCollectionUpdate>::default(),
    ));
    (h, character)
}

#[test]
fn achievements_criteria_toys_native_learning_sets_owned_item_and_distinct_count() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("criteria-toys.redb");
    let (mut h, character) = setup(&path);
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    bevy::app::Plugin::build(&crate::achievement::AchievementPlugin, &mut h.app);
    let player = h.caster;
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(crate::achievement::PlayerAchievementState::default());
    let db = &h.app.world().resource::<GameDatabase>().db;
    let mut inventory = persistence::load_inventory(db, character);
    inventory.items.push(InventoryItemData {
        item_guid: persistence::next_id(db, "item_instance"),
        item_id: 37710,
        name: String::new(),
        quality: 1,
        required_level: 1,
        stack_count: 2,
        vendor_sell_price: 1,
        unique_auction: false,
    });
    persistence::save_inventory(db, character, &inventory);
    assert!(
        learn(&mut h, 3)
            .iter()
            .any(|r| r.operation == ToyOperation::Learn && r.error.is_none())
    );
    let state = h
        .app
        .world()
        .get::<crate::achievement::PlayerAchievementState>(player)
        .unwrap();
    assert_eq!(state.criteria_progress.get(&27841), Some(&1));
    assert_eq!(
        state
            .account
            .as_ref()
            .unwrap()
            .criteria_progress
            .get(&26801),
        Some(&1)
    );
    assert!(
        learn(&mut h, 3)
            .iter()
            .any(|r| r.error.as_deref() == Some("toy already learned"))
    );
    learn(&mut h, 0);
    let state = h
        .app
        .world()
        .get::<crate::achievement::PlayerAchievementState>(player)
        .unwrap();
    assert_eq!(
        state
            .account
            .as_ref()
            .unwrap()
            .criteria_progress
            .get(&26801),
        Some(&2)
    );
    assert_eq!(state.criteria_progress.get(&27841), Some(&1));
    drop(h);
    let db = redb::Database::open(&path).unwrap();
    assert_eq!(toy_store::load(&db, 1).len(), 2);
    assert!(toy_store::load(&db, 2).is_empty());
    assert_eq!(
        persistence::load_achievements(&db, character)
            .criteria_progress
            .get(&26801),
        Some(&2)
    );
}

fn pump(h: &mut Harness) -> (Vec<ToyResult>, Vec<ToyCollectionUpdate>) {
    let mut results = Vec::new();
    let mut snapshots = Vec::new();
    for _ in 0..7 {
        h.advance(1);
        results.extend(drain::<ToyResult>(&mut h.app, h.connection));
        snapshots.extend(drain::<ToyCollectionUpdate>(&mut h.app, h.connection));
    }
    (results, snapshots)
}

fn learn(h: &mut Harness, slot: u8) -> Vec<ToyResult> {
    h.app
        .world_mut()
        .get_mut::<MessageSender<UseItem>>(h.connection)
        .unwrap()
        .send::<InventoryChannel>(UseItem {
            location: ItemLocation::Bag { bag: 0, slot },
            target: None,
        });
    pump(h).0
}

fn activate(h: &mut Harness, item_id: u32) -> Vec<ToyResult> {
    activate_at(h, item_id, None)
}

fn activate_at(h: &mut Harness, item_id: u32, destination: Option<[f32; 3]>) -> Vec<ToyResult> {
    activate_witnessed(h, item_id, destination, None)
}

fn activate_witnessed(
    h: &mut Harness,
    item_id: u32,
    destination: Option<[f32; 3]>,
    witness: Option<shared::protocol::WitnessRay>,
) -> Vec<ToyResult> {
    h.app
        .world_mut()
        .get_mut::<MessageSender<UseToy>>(h.connection)
        .unwrap()
        .send::<CollectionChannel>(UseToy {
            item_id,
            destination,
            witness,
        });
    pump(h).0
}

#[test]
fn toys_learn_favourite_and_reopen_on_another_character() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("toys.redb");
    let (mut h, character) = setup(&path);
    assert!(
        learn(&mut h, 0)
            .iter()
            .any(|result| result.operation == ToyOperation::Learn && result.error.is_none())
    );
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert!(
        !persistence::load_inventory(db, character)
            .items
            .iter()
            .any(|item| item.item_id == FIGURINE)
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<SetToyFavourite>>(h.connection)
        .unwrap()
        .send::<CollectionChannel>(SetToyFavourite {
            item_id: FIGURINE,
            favourite: true,
        });
    let (results, updates) = pump(&mut h);
    assert!(
        results
            .iter()
            .any(|result| result.operation == ToyOperation::Favourite && result.error.is_none())
    );
    assert!(updates.iter().any(|update| {
        update
            .toys
            .iter()
            .any(|toy| toy.item_id == FIGURINE && toy.learned && toy.favourite)
    }));
    drop(h); // close the actual redb handle before reopening, not a memory-only relog.
    let db = redb::Database::open(&path).unwrap();
    persistence::create_tables(&db);
    let (other, _) = persistence::create_character(&db, 1, "OtherToyMage", 1, 8).unwrap();
    assert_ne!(other, character);
    let mut h = harness(Vec::new(), &[TRANSFORM], 8).unwrap();
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app.insert_resource(GameDatabase { db });
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .remove::<InWorld>();
    h.app.world_mut().entity_mut(h.connection).insert((
        InWorld {
            character_id: other,
        },
        MessageSender::<ToyCollectionUpdate>::default(),
        MessageReceiver::<ToyCollectionUpdate>::default(),
        MessageSender::<ToyResult>::default(),
        MessageReceiver::<ToyResult>::default(),
    ));
    h.app.add_systems(PostUpdate, send_initial);
    let (_, updates) = pump(&mut h);
    let update = updates
        .last()
        .expect("relogged character gets initial Toy Box update");
    assert!(
        update
            .toys
            .iter()
            .any(|toy| toy.item_id == FIGURINE && toy.learned && toy.favourite)
    );
    assert!(
        !snapshot(h.app.world(), 2)
            .toys
            .iter()
            .any(|toy| toy.learned)
    );
    assert!(
        !persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character)
            .items
            .iter()
            .any(|item| item.item_id == FIGURINE)
    );
}

#[test]
fn toys_use_real_transform_starts_item_recovery_and_rejects_repeat() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&dir.path().join("toys.redb"));
    assert_eq!(
        activate(&mut h, FIGURINE)[0].error.as_deref(),
        Some("toy not learned")
    );
    learn(&mut h, 0);
    let results = activate(&mut h, FIGURINE);
    assert_eq!(results[0].spell_id, Some(TRANSFORM));
    assert_eq!(results[0].error, None);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == TRANSFORM)
    );
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cooldown| cooldown.spell_id == TRANSFORM
                && cooldown.duration_ms == 1_800_000
                && !cooldown.is_gcd)
    );
    assert_eq!(
        activate(&mut h, FIGURINE)[0].error.as_deref(),
        Some("OnCooldown")
    );
    assert!(
        h.inbox
            .failed
            .iter()
            .any(|failed| failed.reason == shared::spell_data::CastFailReason::OnCooldown)
    );
}

#[test]
fn toys_category_override_expires_authoritatively() {
    // Eagger Basket: ItemEffect category1203/10s; SpellCooldowns recovery0,
    // spell category44 must not replace the item category.
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&dir.path().join("toys.redb"));
    place_toy_caster_on_source_area(&mut h, 778032, [-9480.94, -38.5858], 87);
    learn(&mut h, 2);
    assert_eq!(activate(&mut h, 188698)[0].error, None);
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|update| update.spell_id == 61810
                && update.category == 1203
                && update.duration_ms == 10_000)
    );
    assert_eq!(
        activate(&mut h, 188698)[0].error.as_deref(),
        Some("OnCooldown")
    );
    h.advance(201);
    assert_eq!(activate(&mut h, 188698)[0].error, None);
}

#[test]
fn toys_unsupported_dummy_is_not_silent_success() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&dir.path().join("toys.redb"));
    learn(&mut h, 1);
    let results = activate(&mut h, FLAG_OWNERSHIP);
    assert!(results[0].error.as_deref().unwrap().contains("effect 3"));
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .all(|cooldown| cooldown.spell_id != 51640)
    );
    let unavailable = snapshot(h.app.world(), 1)
        .toys
        .into_iter()
        .find(|toy| toy.item_id == FLAG_OWNERSHIP)
        .unwrap();
    assert!(unavailable.learned);
    assert!(unavailable.unavailable_reason.unwrap().contains("effect 3"));
}

#[test]
fn toys_broadcast_learning_and_favourites_only_to_own_account() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&dir.path().join("toys.redb"));
    let other_player = spawn_player(&mut h.app, 8, 0.0);
    let same = spawn_peer_connection(&mut h.app, other_player, 2);
    let foreign_player = spawn_player(&mut h.app, 8, 0.0);
    let foreign = spawn_peer_connection(&mut h.app, foreign_player, 3);
    h.app
        .world_mut()
        .entity_mut(foreign)
        .insert(Authenticated { account_id: 2 });
    for connection in [same, foreign] {
        h.app.world_mut().entity_mut(connection).insert((
            MessageSender::<ToyCollectionUpdate>::default(),
            MessageReceiver::<ToyCollectionUpdate>::default(),
        ));
    }
    let character = h
        .app
        .world()
        .get::<InWorld>(h.connection)
        .unwrap()
        .character_id;
    let caster = h.caster;
    learn_item(
        h.app.world_mut(),
        character,
        caster,
        ItemLocation::Bag { bag: 0, slot: 0 },
    )
    .unwrap();
    h.app
        .world_mut()
        .get_mut::<MessageSender<SetToyFavourite>>(h.connection)
        .unwrap()
        .send::<CollectionChannel>(SetToyFavourite {
            item_id: FIGURINE,
            favourite: true,
        });
    let mut same_updates = Vec::new();
    let mut foreign_updates = Vec::new();
    for _ in 0..7 {
        h.advance(1);
        same_updates.extend(drain::<ToyCollectionUpdate>(&mut h.app, same));
        foreign_updates.extend(drain::<ToyCollectionUpdate>(&mut h.app, foreign));
    }
    assert!(same_updates.iter().any(|update| {
        update
            .toys
            .iter()
            .any(|toy| toy.item_id == FIGURINE && toy.learned && toy.favourite)
    }));
    assert!(
        foreign_updates
            .iter()
            .all(|update| update.toys.iter().all(|toy| !toy.learned && !toy.favourite))
    );
}

#[test]
fn toys_duplicate_learning_keeps_remaining_item_and_favourite_requires_ownership() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    let db = &h.app.world().resource::<GameDatabase>().db;
    let mut inventory = persistence::load_inventory(db, character);
    inventory.items[0].stack_count = 2;
    persistence::save_inventory(db, character, &inventory);
    h.app
        .world_mut()
        .get_mut::<MessageSender<SetToyFavourite>>(h.connection)
        .unwrap()
        .send::<CollectionChannel>(SetToyFavourite {
            item_id: FIGURINE,
            favourite: true,
        });
    assert_eq!(pump(&mut h).0[0].error.as_deref(), Some("toy not learned"));
    learn(&mut h, 0);
    let results = learn(&mut h, 0);
    assert_eq!(results[0].error.as_deref(), Some("toy already learned"));
    let inventory =
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character);
    assert_eq!(
        inventory
            .items
            .iter()
            .find(|item| item.item_id == FIGURINE)
            .unwrap()
            .stack_count,
        1
    );
}

fn learn_effect_toy(h: &mut Harness, character: u64, item_id: u32) {
    let db = &h.app.world().resource::<GameDatabase>().db;
    persistence::save_inventory(
        db,
        character,
        &InventoryData {
            gold: 0,
            items: vec![InventoryItemData {
                item_guid: persistence::next_id(db, "item_instance"),
                item_id,
                name: String::new(),
                quality: 1,
                required_level: 1,
                stack_count: 1,
                vendor_sell_price: 0,
                unique_auction: false,
            }],
        },
    );
    let results = learn(h, 0);
    assert!(
        results
            .iter()
            .any(|result| result.operation == ToyOperation::Learn && result.error.is_none()),
        "physical toy {item_id} did not learn: {results:?}; errors: {:?}",
        drain::<InventoryError>(&mut h.app, h.connection)
    );
}

#[test]
fn toyfx_cosmetic_aura_is_applied_replicated_and_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    learn_effect_toy(&mut h, character, 119144); // Touch of the Naaru, 176594.
    assert_eq!(activate(&mut h, 119144)[0].error, None);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 176594)
    );
    assert!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 176594 && a.caster == Some(h.caster.to_bits()))
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura { spell_id: 176594 });
    h.advance(5);
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 176594)
    );
}

#[test]
fn toyfx_cosmetic_dummy_and_script_deliver_spell_go() {
    for (item, spell, target) in [
        (119083, 176457, true),
        (126931, 185712, false),
        (169768, 302273, false),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("toys.redb"));
        learn_effect_toy(&mut h, character, item);
        if target {
            h.select(Some(h.npc));
            h.advance(2);
        }
        assert_eq!(activate(&mut h, item)[0].error, None, "item {item}");
        h.advance(60); // Includes actual cast time / missile flight, no mocked completion.
        assert!(
            h.inbox.go.iter().any(|go| go.spell_id == spell
                && go.caster == h.caster.to_bits()
                && go.hit_targets.contains(&if target {
                    h.npc.to_bits()
                } else {
                    h.caster.to_bits()
                })),
            "item {item}"
        );
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    }
}

#[test]
fn toyfx_screen_effect_replicates_and_cancel_removes_it() {
    const SCREEN_EFFECT_AURA: u32 = 260;
    const DURATION_MS: u32 = 7_200_000;
    let dir = tempfile::tempdir().unwrap();
    for (item, spell, screen_effect, recovery_ms) in [
        (170187, 304370, 1720, 3_600_000),
        (170380, 304369, 1719, 86_400_000),
    ] {
        let (mut h, character) = setup(&dir.path().join(format!("screen-{item}.redb")));
        assert!(
            snapshot(h.app.world(), 1)
                .toys
                .iter()
                .find(|toy| toy.item_id == item)
                .unwrap()
                .unavailable_reason
                .is_none()
        );
        learn_effect_toy(&mut h, character, item);
        assert_eq!(activate(&mut h, item)[0].error, None, "item{item}");
        h.advance(40);
        assert!(
            h.inbox.failed.is_empty(),
            "item{item}: {:?}",
            h.inbox.failed
        );
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
        let auras = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap();
        let aura = auras.iter().find(|a| a.spell_id == spell).unwrap();
        assert_eq!(aura.duration_ms, DURATION_MS);
        assert_eq!(aura.effects[0].aura_type, SCREEN_EFFECT_AURA);
        assert_eq!(aura.effects[0].misc_values, [screen_effect, 7]);
        let cooldown = h
            .inbox
            .cooldowns
            .iter()
            .find(|cooldown| cooldown.spell_id == spell && !cooldown.is_gcd)
            .unwrap();
        assert_eq!(cooldown.duration_ms, recovery_ms);
        assert!(
            h.app
                .world()
                .get::<shared::components::UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|a| a.spell_id == spell)
        );
        h.app
            .world_mut()
            .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
            .unwrap()
            .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
                spell_id: spell,
            });
        h.advance(5);
        assert!(
            !h.app
                .world()
                .get::<shared::components::UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|a| a.spell_id == spell)
        );
        assert_eq!(
            activate(&mut h, item)[0].error.as_deref(),
            Some("OnCooldown"),
            "item{item}: cancelling the appearance preserves item recovery"
        );
    }
}

#[test]
fn toyfx_transform_changes_display_and_cancel_restores_native() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&dir.path().join("toys.redb"));
    assert!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(h.caster)
            .is_none()
    );
    learn(&mut h, 0); // Time-Lost Figurine32782, real transform creature23423.
    assert_eq!(activate(&mut h, FIGURINE)[0].error, None);
    let template = &h
        .app
        .world()
        .resource::<crate::world_data::WorldData>()
        .templates[&23423];
    let display = h
        .app
        .world()
        .get::<shared::components::ModelDisplay>(h.caster)
        .expect("transform must change display")
        .display_id;
    assert!(
        template
            .models
            .iter()
            .any(|model| model.display_id == display)
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
            spell_id: TRANSFORM,
        });
    h.advance(5);
    assert!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(h.caster)
            .is_none(),
        "native race/sex display restored by removing transform model"
    );
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == TRANSFORM)
    );
}

#[test]
fn toyfx_visual_audit_requires_binding_and_no_extra_effects() {
    for remove_visual in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("toys.redb"));
        learn_effect_toy(&mut h, character, 119144);
        let mut spell = world_db().unwrap().store.get(176594).unwrap().clone();
        if remove_visual {
            spell.has_spell_visual = false;
        } else {
            spell.effects.push(crate::spell_info::SpellEffect {
                effect: 2,
                index: 1,
                implicit_targets: [1, 0],
                base_points: 100.0,
                ..default()
            });
        }
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(vec![spell]));
        assert!(
            activate(&mut h, 119144)[0]
                .error
                .as_deref()
                .unwrap()
                .contains("aura 4")
        );
        assert!(h.inbox.cooldowns.is_empty());
        assert!(h.inbox.go.is_empty());
    }
    let mut world = World::new();
    let spells = [23453, 360598, 170950]
        .into_iter()
        .map(|id| world_db().unwrap().store.get(id).unwrap().clone())
        .collect();
    world.insert_resource(crate::spell_info::SpellStore::fixture(spells));
    for (id, reason) in [
        (23453, "effect 3"),
        (360598, "effect 3"),
        (170950, "aura 226"),
    ] {
        assert!(
            toy_catalog::unsupported_types(&world, id).contains(reason),
            "script spell {id} must stay unavailable"
        );
    }
}

#[test]
fn toyfx_wild_brazier_spawns_owned_with_template_faction() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE summon_properties (
            ID INTEGER, Control INTEGER, Faction INTEGER, Title INTEGER, Slot INTEGER,
            Flags_0 INTEGER, Flags_1 INTEGER
        ); INSERT INTO summon_properties VALUES (64,0,0,0,0,0,0);",
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    learn_effect_toy(&mut h, character, 34686);
    assert_eq!(activate(&mut h, 34686)[0].error, None);
    h.advance(60);
    let world = h.app.world_mut();
    let summons: Vec<_> = world
        .query::<(
            Entity,
            &shared::components::Npc,
            &shared::components::UnitSummonedBy,
        )>()
        .iter(world)
        .filter(|(_, npc, _)| npc.template_id == 25305)
        .map(|(entity, _, owner)| (entity, owner.0))
        .collect();
    assert_eq!(summons.len(), 1);
    assert_eq!(summons[0].1, h.caster.to_bits());
    let faction = world.resource::<crate::world_data::WorldData>().templates[&25305].faction;
    assert_eq!(
        world
            .get::<shared::components::UnitFactionTemplate>(summons[0].0)
            .unwrap()
            .0,
        u32::from(faction)
    );
    assert!(
        world
            .get::<crate::creature_ai::FollowOwner>(summons[0].0)
            .is_none()
    );
}

#[test]
fn toyfx2_scale_toy_applies_replicates_and_reverts() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    learn_effect_toy(&mut h, character, 18660); // World Enlarger, spell23126, -50%.
    assert_eq!(activate(&mut h, 18660)[0].error, None);
    assert!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 23126)
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitScale>(h.caster)
            .unwrap()
            .0,
        0.5
    );
    // World Enlarger -50 + World Shrinker100: additive1.5, not multiplicative1.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 156132,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitScale>(h.caster)
            .unwrap()
            .0,
        1.5
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 156132,
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitScale>(h.caster)
            .unwrap()
            .0,
        0.5
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura { spell_id: 23126 });
    h.advance(5);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitScale>(h.caster)
            .unwrap()
            .0,
        1.0
    );
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 23126)
    );
}

#[test]
fn toyfx2_feather_fall_authentic_toy_spell_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    // Imported Whispers113542 now learns/activates through the real item boundary.
    assert!(
        snapshot(h.app.world(), 1)
            .toys
            .iter()
            .find(|toy| toy.item_id == 113542)
            .unwrap()
            .unavailable_reason
            .is_none()
    );
    learn_effect_toy(&mut h, character, 113542);
    assert_eq!(activate(&mut h, 113542)[0].error, None);
    h.advance(40); // Authentic one-second cast, not an immediate aura fixture.
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 167273));
    assert!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 167273)
    );
    assert!(
        h.app
            .world()
            .get::<shared::components::FeatherFall>(h.caster)
            .unwrap()
            .0
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura { spell_id: 167273 });
    h.advance(5);
    assert!(
        !h.app
            .world()
            .get::<shared::components::FeatherFall>(h.caster)
            .unwrap()
            .0
    );
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 167273)
    );
}

#[test]
fn toyfx2_override_toy_applies_replicates_and_reverts() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![176889]));
    assert!(!crate::spell_cast::caster_knows_spell(
        h.app.world_mut(),
        h.caster,
        225656
    ));
    learn_effect_toy(&mut h, character, 119211); // Golden Hearthstone Card,176889, override785.
    assert_eq!(activate(&mut h, 119211)[0].error, None);
    h.advance(10);
    let auras = h
        .app
        .world()
        .get::<shared::components::UnitAuras>(h.caster)
        .unwrap();
    let aura = auras.auras.iter().find(|a| a.spell_id == 176889).unwrap();
    assert!(
        aura.overrides
            .contains(&shared::components::AuraOverride::SpellSet {
                id: 785,
                spells: vec![225656, 225658, 225660, 225657, 225659, 0, 0, 0, 0, 0]
            })
    );
    assert!(crate::spell_cast::caster_knows_spell(
        h.app.world_mut(),
        h.caster,
        225656
    ));
    assert!(
        !h.app
            .world()
            .get::<crate::class_progression::KnownSpells>(h.caster)
            .unwrap()
            .contains(225656)
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 176889,
    );
    h.advance(3);
    assert!(!crate::spell_cast::caster_knows_spell(
        h.app.world_mut(),
        h.caster,
        225656
    ));
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 176889)
    );
}

#[test]
fn toyfx3_convergent_prism_is_usable_and_removes_no_actions_on_cancel() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    learn_effect_toy(&mut h, character, 193033); // Convergent Prism376088.
    assert_eq!(activate(&mut h, 193033)[0].error, None);
    let frostbolt = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(116)
        .unwrap()
        .clone();
    assert!(crate::auras::cast_blocker(h.app.world(), h.caster, &frostbolt).is_some());
    assert!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 376088)
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 376088,
    );
    assert!(crate::auras::cast_blocker(h.app.world(), h.caster, &frostbolt).is_none());
}

fn assert_linked_tent_use_refresh_cancel_and_other_owner_lifecycle(
    item: u32,
    root_spell: u32,
    child_spell: u32,
    display: u32,
) {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    learn_effect_toy(&mut h, character, item);
    assert_eq!(activate(&mut h, item)[0].error, None);
    h.advance(60); // Authentic cast time is3000ms; ToyResult only reports start.
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let summons = |world: &mut World| -> Vec<(Entity, Entity)> {
        world
            .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
            .iter(world)
            .filter(|(_, s)| s.spell_id == child_spell)
            .map(|(e, s)| (e, s.owner))
            .collect()
    };
    let first = summons(h.app.world_mut());
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].1, h.caster);
    let pos = h
        .app
        .world()
        .get::<shared::components::Position>(first[0].0)
        .unwrap();
    let owner = h
        .app
        .world()
        .get::<shared::components::Position>(h.caster)
        .unwrap();
    // TargetA18=DEST_CASTER; TargetB79=DEST_DEST_BACK; DB2 radius7=2yd.
    assert!((pos.x - (owner.x - 2.0)).abs() < 0.001);
    assert_eq!(pos.y, owner.y);
    assert!((pos.z - owner.z).abs() < 0.001);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(first[0].0)
            .unwrap()
            .display_id,
        display
    );
    let apply = |world: &mut World, unit| {
        crate::auras::apply_aura(
            world,
            &crate::auras::AuraApplication {
                caster: unit,
                target: unit,
                spell_id: root_spell,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        )
    };
    apply(h.app.world_mut(), h.caster); // Refresh does not make another tent.
    assert_eq!(summons(h.app.world_mut()), first);
    let origin = *h.app.world().get::<Position>(h.caster).unwrap();
    let other_position = Position {
        x: origin.x + 10.0,
        ..origin
    };
    let other = spawn_player(&mut h.app, 8, other_position.x);
    h.app.world_mut().entity_mut(other).insert((
        other_position,
        shared::components::Rotation {
            x: 0.0,
            y: std::f32::consts::PI,
            z: 0.0,
        },
    ));
    apply(h.app.world_mut(), other);
    let both = summons(h.app.world_mut());
    assert_eq!(both.len(), 2);
    let rotated = both.iter().find(|(_, owner)| *owner == other).unwrap().0;
    let pos = h
        .app
        .world()
        .get::<shared::components::Position>(rotated)
        .unwrap();
    assert!((pos.x - other_position.x).abs() < 0.001);
    assert!((pos.z - other_position.z - 2.0).abs() < 0.001);
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == root_spell,
    );
    let remaining = summons(h.app.world_mut());
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].1, other);
    crate::auras::remove_auras(
        h.app.world_mut(),
        other,
        crate::auras::RemoveReason::Death,
        |a| a.spell_id == root_spell,
    );
    assert!(summons(h.app.world_mut()).is_empty());
}

#[test]
fn toyfx3_linked_tent_use_refresh_cancel_and_other_owner_lifecycle() {
    assert_linked_tent_use_refresh_cancel_and_other_owner_lifecycle(193476, 398159, 398162, 111084);
}

#[test]
fn toyfx338_market_tent_use_refresh_cancel_and_other_owner_lifecycle() {
    assert_linked_tent_use_refresh_cancel_and_other_owner_lifecycle(194059, 398329, 398328, 111097);
}

#[test]
fn toyfx339_expedition_tent_use_refresh_cancel_and_other_owner_lifecycle() {
    assert_linked_tent_use_refresh_cancel_and_other_owner_lifecycle(194060, 398334, 398333, 111099);
}

#[test]
fn toyfx3_bobber_overrides_remain_explicitly_unsupported() {
    // Toy142528, Can of Worms:231291 replaces bobber35591 with241593/385542.
    let mut world = World::new();
    let data = world_db().unwrap();
    world.insert_resource(crate::spell_info::SpellStore::fixture(
        [231291, 131474]
            .into_iter()
            .map(|id| data.store.get(id).unwrap().clone())
            .collect(),
    ));
    let reasons = toy_catalog::unsupported_types(&world, 231291);
    assert!(reasons.contains("aura 258"));
    assert!(
        reasons
            .iter()
            .any(|r| r.contains("35591 -> 241593") && r.contains("transmitted-object/fishing"))
    );
}

#[test]
fn toyfx4_transmitted_objects_spawn_at_data_destinations_and_expire() {
    // DB2/TDB: Stormwind Banner45011->62736->194274 (generic),
    // Fishing Chair33223->42766->186475, MOLL-E40768->54710->191605,
    // Reliquary Banner198729->384876->380639 and sourced dragon banners.
    for (item, spell, entry, go_type, display, scale, offset, lifetime_seconds) in [
        (45011, 62736, 194274, 5, 10483, 1.0, 0.0, 180),
        (33223, 42766, 186475, 7, 7467, 1.0, 2.0, 300),
        (40768, 54710, 191605, 19, 8171, 0.5, 3.0, 600),
        (198729, 384876, 380639, 5, 76510, 0.6, 0.0, 180),
        (199768, 388089, 381318, 5, 71263, 0.5, 0.0, 180),
        (199769, 388090, 381321, 5, 77021, 0.5, 0.0, 180),
        (199770, 388091, 381319, 5, 76419, 0.5, 0.0, 180),
        (199771, 388092, 381320, 5, 77020, 0.5, 0.0, 180),
        (199900, 388267, 206590, 5, 10103, 1.0, 0.0, 300),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("toys.redb"));
        let conn = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        h.app
            .insert_resource(crate::meeting_stone::load_meeting_stone_data(&conn).unwrap());
        h.app
            .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(shared::components::Rotation {
                x: 0.0,
                y: std::f32::consts::FRAC_PI_2,
                z: 0.0,
            });
        learn_effect_toy(&mut h, character, item);
        // Banners/survey tools use target87: the clicked point, not the caster's feet.
        // Chair42766/MOLL-E54710 remain target47's authored forward offsets.
        let cursor_object = matches!(
            item,
            45011 | 198729 | 199768 | 199769 | 199770 | 199771 | 199900
        );
        let destination = cursor_object.then_some([4.0, 0.0, -3.0]);
        assert_eq!(
            activate_at(&mut h, item, destination)[0].error,
            None,
            "toy {item}"
        );
        h.advance(100); // includes real cast time for MOLL-E.
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
        let world = h.app.world_mut();
        let objects: Vec<_> = world
            .query::<(Entity, &shared::protocol::GameObjectInfo)>()
            .iter(world)
            .filter(|(_, info)| info.entry == entry)
            .map(|(e, info)| (e, info.clone()))
            .collect();
        assert_eq!(objects.len(), 1, "toy {item} must create its real object");
        let (object, info) = &objects[0];
        assert_eq!(
            (info.go_type, info.display_id, info.scale),
            (go_type, display, scale)
        );
        let pos = world.get::<shared::components::Position>(*object).unwrap();
        let expected = destination.unwrap_or([offset, 0.0, 0.0]);
        assert!((pos.x - expected[0]).abs() < 0.001, "{pos:?}");
        assert_eq!((pos.y, pos.z), (expected[1], expected[2]));
        assert_eq!(
            world.get::<crate::maps::InMap>(*object),
            world.get::<crate::maps::InMap>(h.caster)
        );
        assert_eq!(
            world
                .get::<crate::game_objects::summoned::SummonedObject>(*object)
                .unwrap()
                .owner,
            Some(h.caster)
        );
        assert_eq!(
            world
                .get::<shared::components::Rotation>(*object)
                .unwrap()
                .y,
            std::f32::consts::FRAC_PI_2
        );
        if go_type == 7 {
            assert!(world.get::<crate::chair::Chair>(*object).is_some());
        }
        h.advance(lifetime_seconds * 20 - 200); // survives normal cast completion.
        assert!(h.app.world().get_entity(*object).is_ok());
        h.advance(200); // chair's timer begins after its real3000ms cast.
        assert!(h.app.world().get_entity(*object).is_err(), "toy {item}");
    }
}

#[test]
fn toyfx4_destination_request_does_not_select_a_unit_or_use_target_facing() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&dir.path().join("toys.redb"));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::Rotation {
            x: 0.0,
            y: 1.2 + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        });
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(shared::components::Rotation {
            x: 0.0,
            y: 2.0 + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        });
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::SpellCastIntent>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::SpellCastIntent {
            spell_id: Some(62736),
            spell: String::new(),
            target_entity: Some(h.npc.to_bits()),
            destination: Some([7.0, 4.0, -2.0]),
            target_item_guid: None,
            witness: None,
        });
    h.advance(7);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let world = h.app.world_mut();
    let objects: Vec<_> = world
        .query::<(Entity, &shared::protocol::GameObjectInfo)>()
        .iter(world)
        .filter(|(_, info)| info.entry == 194274)
        .map(|(e, _)| e)
        .collect();
    assert_eq!(objects.len(), 1);
    assert_eq!(
        *world
            .get::<shared::components::Position>(objects[0])
            .unwrap(),
        shared::components::Position {
            x: 7.0,
            y: 4.0,
            z: -2.0
        }
    );
    assert!(
        (world
            .get::<shared::components::Rotation>(objects[0])
            .unwrap()
            .y
            - (1.2 + std::f32::consts::FRAC_PI_2))
            .abs()
            < 0.001
    );
    assert_eq!(
        world
            .get::<crate::game_objects::summoned::SummonedObject>(objects[0])
            .unwrap()
            .owner,
        Some(h.caster)
    );
}

#[test]
fn toyfx4_nonritual_interactive_objects_are_not_silent_success() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    // Both native spells use TARGET_DEST_DEST87: no placement click means NoTarget.
    // Type22 now has the shared native spellcaster runtime; it is no longer an
    // unsupported-type fixture. Goblin Gumbo must really spawn after placement.
    learn_effect_toy(&mut h, character, 33219);
    let error = activate(&mut h, 33219)[0].error.clone().unwrap();
    assert!(error.contains("NoTarget"), "{error}");
    assert!(!h.inbox.cooldowns.iter().any(|c| c.spell_id == 42753));
    assert_eq!(
        activate_at(&mut h, 33219, Some([2.0, 0.0, 0.0]))[0].error,
        None
    );
    h.advance(60); // Native Goblin Gumbo has a two-second cast before object creation.
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 42753));
    let world = h.app.world_mut();
    let spawned: Vec<_> = world
        .query::<&shared::protocol::GameObjectInfo>()
        .iter(world)
        .filter(|info| info.entry == 186472)
        .collect();
    assert_eq!(spawned.len(), 1);
    assert_eq!(spawned[0].go_type, 22);
    // Actual DB2 toy199767/388088 references381317, absent in local TDB1210.
    learn_effect_toy(&mut h, character, 199767);
    let error = activate_at(&mut h, 199767, Some([2.0, 0.0, 0.0]))[0]
        .error
        .clone()
        .unwrap();
    assert!(error.contains("gameobject template"), "{error}");
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == 388088));
}

#[test]
fn toyfx71_rezan_roar_real_learning_cast_event_and_source_recovery() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    const ITEM: u32 = 160509;
    const SPELL: u32 = 272071;
    const RECOVERY_MS: u32 = 90_000;
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("roar.redb"));
    let path = crate::test_data::world_db_path();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    let health_before = *h.app.world().get::<Health>(h.caster).unwrap();
    let target_health = *h.app.world().get::<Health>(h.npc).unwrap();
    let model_before = h.app.world().get::<ModelDisplay>(h.caster).cloned();
    learn_effect_toy(&mut h, character, ITEM);
    assert_eq!(
        activate(&mut h, ITEM)[0].error,
        None,
        "real Roar learned-use"
    );
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| go.spell_id == SPELL && go.caster == h.caster.to_bits())
    );
    assert!(h.inbox.cooldowns.iter().any(|cooldown| {
        cooldown.spell_id == SPELL && cooldown.duration_ms == RECOVERY_MS && !cooldown.is_gcd
    }));
    assert_eq!(
        *h.app.world().get::<Health>(h.caster).unwrap(),
        health_before
    );
    assert_eq!(*h.app.world().get::<Health>(h.npc).unwrap(), target_health);
    assert_eq!(
        h.app.world().get::<ModelDisplay>(h.caster),
        model_before.as_ref()
    );
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(h.caster)
            .is_none_or(|auras| auras.auras.iter().all(|aura| aura.spell_id != SPELL))
    );
    assert_eq!(
        h.app
            .world_mut()
            .query::<&crate::spell_cast::summon::SummonedCreature>()
            .iter(h.app.world())
            .count(),
        0
    );
    let first_cast_count = h.inbox.go.iter().filter(|go| go.spell_id == SPELL).count();
    assert!(
        activate(&mut h, ITEM)[0]
            .error
            .as_deref()
            .is_some_and(|error| error.contains("Cooldown"))
    );
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == SPELL).count(),
        first_cast_count
    );
    h.advance(1_801); // Harness advances50ms/frame: source90s recovery elapses normally.
    assert_eq!(activate(&mut h, ITEM)[0].error, None);
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == SPELL).count(),
        first_cast_count + 1
    );
}

#[test]
fn toyfx4_humanoid_model_is_viewer_aura_not_global_transform() {
    for (item, spell, payload) in [
        (158149, 264091, [3569, 53]), // Overtuned Corgi Goggles
        (166678, 289277, [4076, 155]),
        (198227, 384585, [6409, 170]),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("toys.redb"));
        learn_effect_toy(&mut h, character, item);
        assert_eq!(activate(&mut h, item)[0].error, None);
        h.advance(60); // Brynja has a real2500ms cast; goggles are instant.
        let active = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap();
        let aura = active.iter().find(|a| a.spell_id == spell).unwrap();
        assert_eq!(aura.effects[0].aura_type, 233);
        assert_eq!(aura.effects[0].misc_values, payload);
        assert!(
            h.app
                .world()
                .get::<shared::components::UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|a| a.spell_id == spell)
        );
        assert!(
            h.app
                .world()
                .get::<shared::components::ModelDisplay>(h.caster)
                .is_none()
        );
        crate::auras::remove_auras(
            h.app.world_mut(),
            h.caster,
            crate::auras::RemoveReason::Cancelled,
            |a| a.spell_id == spell,
        );
        h.advance(3);
        assert!(
            !h.app
                .world()
                .get::<shared::components::UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|a| a.spell_id == spell)
        );
    }
}

#[test]
fn toyfx5_visual_attachments_use_real_toys_and_cancel_without_gameplay_changes() {
    for (item, spell) in [
        (156871, 261981),  // Spitzy
        (165791, 286277),  // Worn Cloak
        (221964, 455023),  // Filmless Camera
        (228413, 462145),  // Lampyridae Lure
        (263198, 1269949), // Valdekar's Special
        (267456, 1280563), // Lil' Scoots' Pillow
    ] {
        let dir = tempfile::tempdir().unwrap();
        let race = if item == 165791 { 2 } else { 1 }; // Worn Cloak is Horde-only.
        let (mut h, character) = setup_for_race(&dir.path().join("toys.redb"), race);
        learn_effect_toy(&mut h, character, item);
        let health_before = *h
            .app
            .world()
            .get::<shared::components::Health>(h.caster)
            .unwrap();
        assert_eq!(activate(&mut h, item)[0].error, None, "toy {item}");
        assert!(
            h.inbox.failed.is_empty(),
            "toy {item}: {:?}",
            h.inbox.failed
        );
        assert!(
            h.inbox
                .go
                .iter()
                .any(|go| go.spell_id == spell && go.hit_targets.contains(&h.caster.to_bits())),
            "toy {item}"
        );
        let auras = h
            .app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap();
        assert!(
            auras
                .auras
                .iter()
                .any(|aura| aura.spell_id == spell && aura.caster == Some(h.caster.to_bits())),
            "toy {item}"
        );
        assert_eq!(
            *h.app
                .world()
                .get::<shared::components::Health>(h.caster)
                .unwrap(),
            health_before
        );
        assert!(
            h.app
                .world()
                .get::<shared::components::ModelDisplay>(h.caster)
                .is_none()
        );
        assert!(
            persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character)
                .items
                .is_empty()
        );
        h.app
            .world_mut()
            .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
            .unwrap()
            .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
                spell_id: spell,
            });
        h.advance(5);
        assert!(
            !h.app
                .world()
                .get::<shared::components::UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|aura| aura.spell_id == spell),
            "toy {item}"
        );
        if item == 165791 {
            h.app
                .world_mut()
                .get_mut::<shared::components::Player>(h.caster)
                .unwrap()
                .race = 1;
            assert_eq!(
                activate(&mut h, item)[0].error.as_deref(),
                Some("toy race/class restriction")
            );
        }
    }
}

#[test]
fn toyfx5_mask_affects_selected_friend_and_mortar_delivers_real_cast_time() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    let friend = spawn_player(&mut h.app, 8, 3.0);
    h.select(Some(friend));
    h.advance(2);
    learn_effect_toy(&mut h, character, 88580);
    assert_eq!(activate(&mut h, 88580)[0].error, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(friend)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == 128328 && aura.caster == Some(h.caster.to_bits()))
    );
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .is_some_and(|auras| auras.auras.iter().any(|aura| aura.spell_id == 128328))
    );
    h.advance(1200); // authentic 60000ms duration, not a permanent costume.
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(friend)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == 128328)
    );

    learn_effect_toy(&mut h, character, 204818);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        });
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.npc)
        .unwrap()
        .x = 20.0;
    h.select(Some(h.npc));
    h.advance(2);
    let health_before = *h
        .app
        .world()
        .get::<shared::components::Health>(h.npc)
        .unwrap();
    assert_eq!(activate(&mut h, 204818)[0].error, None);
    assert!(
        !h.inbox.go.iter().any(|go| go.spell_id == 406870),
        "real 2000ms cast must not finish instantly"
    );
    h.advance(80); // Includes the cast and the DB2 missile travel.
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 406870
        && go.caster == h.caster.to_bits()
        && go.hit_targets.contains(&h.npc.to_bits())));
    assert_eq!(
        *h.app
            .world()
            .get::<shared::components::Health>(h.npc)
            .unwrap(),
        health_before,
        "a squeaking rubber duck is not a damage spell"
    );
}

#[test]
fn toyfx5_attachment_descriptions_without_visual_bindings_remain_unavailable() {
    for (item, spell) in [(133997, 203533), (134007, 203820)] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("toys.redb"));
        learn_effect_toy(&mut h, character, item);
        assert!(
            !world_db()
                .unwrap()
                .store
                .get(spell)
                .unwrap()
                .has_spell_visual
        );
        let error = activate(&mut h, item)[0].error.clone().unwrap();
        assert!(
            error.contains("aura 4") && error.contains("effect 3"),
            "{error}"
        );
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == spell));
        assert!(
            !h.inbox
                .cooldowns
                .iter()
                .any(|cooldown| cooldown.spell_id == spell)
        );
    }
}

#[test]
fn toyfx5_short_attachments_expire_at_retail_durations() {
    for (item, spell, frames) in [(263198, 1269949, 70), (267456, 1280563, 230)] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("toys.redb"));
        learn_effect_toy(&mut h, character, item);
        assert_eq!(activate(&mut h, item)[0].error, None);
        assert!(
            h.app
                .world()
                .get::<shared::components::UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|aura| aura.spell_id == spell)
        );
        h.advance(frames);
        assert!(
            !h.app
                .world()
                .get::<shared::components::UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|aura| aura.spell_id == spell)
        );
    }
}

#[test]
fn toyfx6_duck_lovie_learns_uses_replicates_and_cancels_authored_nap() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    learn_effect_toy(&mut h, character, 194056);
    let health_before = *h
        .app
        .world()
        .get::<shared::components::Health>(h.caster)
        .unwrap();
    assert_eq!(activate(&mut h, 194056)[0].error, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 383065));
    assert!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == 383065)
    );
    assert_eq!(
        *h.app
            .world()
            .get::<shared::components::Health>(h.caster)
            .unwrap(),
        health_before
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura { spell_id: 383065 });
    h.advance(5);
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == 383065)
    );
}

#[test]
fn toyfx7_rabbit_hat_casts_then_expires_without_spawning_a_creature() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    learn_effect_toy(&mut h, character, 147838); // Akazamzarak's Spare Hat239560.
    assert_eq!(activate(&mut h, 147838)[0].error, None);
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == 239560));
    h.advance(50); // The authored2000ms cast precedes the5000ms hat/rabbit visual.
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 239560));
    assert!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == 239560)
    );
    assert_eq!(
        h.app
            .world_mut()
            .query::<&crate::spell_cast::summon::SummonedCreature>()
            .iter(h.app.world())
            .count(),
        0,
        "the rabbit is an authored visual model, not an NPC"
    );
    h.advance(110);
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == 239560)
    );
}

#[test]
fn toyfx7_barrier_is_a_timed_cosmetic_channel_not_a_damage_absorb() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    learn_effect_toy(&mut h, character, 153183); // Barrier Generator254513.
    let health_before = *h
        .app
        .world()
        .get::<shared::components::Health>(h.caster)
        .unwrap();
    assert_eq!(activate(&mut h, 153183)[0].error, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 254513));
    assert!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == 254513)
    );
    crate::combat::deal_direct_damage(h.app.world_mut(), h.npc, h.caster, 20.0);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Health>(h.caster)
            .unwrap()
            .current,
        health_before.current - 20.0,
        "a shield visual must not absorb damage"
    );
    h.advance(210); // Authored10000ms channel and visual aura lifetime.
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == 254513)
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Health>(h.caster)
            .unwrap()
            .max,
        health_before.max
    );
}

#[test]
fn toyfx7_drake_basket_preserves_aura_identity_and_cancellation() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("toys.redb"));
    learn_effect_toy(&mut h, character, 204675); // A Drake's Big Basket of Eggs405634.
    assert_eq!(activate(&mut h, 204675)[0].error, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 405634));
    assert!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == 405634)
    );
    assert!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(h.caster)
            .is_none()
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura { spell_id: 405634 });
    h.advance(5);
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == 405634)
    );
}

/// world.db plus `add_toy_summon_templates.py`, imported once per test process and shared
/// read-only by every toy test. A per-test copy and import of the 770 MB database cost ~2 s.
/// The file is named by PID; an import whose process has exited is removed here.
fn toyfx8_world() -> std::path::PathBuf {
    static PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        let directory = std::env::temp_dir();
        remove_exited_toyfx8_worlds(&directory);
        let path = directory.join(format!("toyfx8-world-{}.sqlite", std::process::id()));
        std::fs::copy(crate::test_data::world_db_path(), &path).unwrap();
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts/add_toy_summon_templates.py");
        let output = std::process::Command::new("python3")
            .arg(script)
            .arg("--db")
            .arg(&path)
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output);
        path
    })
    .clone()
}

fn remove_exited_toyfx8_worlds(directory: &std::path::Path) {
    for entry in std::fs::read_dir(directory).unwrap().flatten() {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|name| name.strip_prefix("toyfx8-world-"))
            .and_then(|rest| rest.strip_suffix(".sqlite"))
        else {
            continue;
        };
        if !std::path::Path::new("/proc").join(pid).exists() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// The creature and item products of the world database at `path`, loaded once per test
/// process: reloading them in every toy test took most of its ~20 s in a debug build and
/// made the toy module the slowest part of the server suite (2026-10-15).
fn world_products(
    path: &std::path::Path,
) -> &'static (
    crate::world_data::WorldData,
    crate::item_templates::ItemTemplates,
) {
    type Products = (
        crate::world_data::WorldData,
        crate::item_templates::ItemTemplates,
    );
    static LOADED: std::sync::Mutex<
        std::collections::BTreeMap<std::path::PathBuf, &'static Products>,
    > = std::sync::Mutex::new(std::collections::BTreeMap::new());
    let mut loaded = LOADED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    loaded.entry(path.to_path_buf()).or_insert_with(|| {
        Box::leak(Box::new((
            crate::world_data::load_world_data(path),
            crate::item_templates::load_item_templates(path).unwrap(),
        )))
    })
}

fn shared_world_data(path: &std::path::Path) -> crate::world_data::WorldData {
    world_products(path).0.clone()
}

fn shared_item_templates(path: &std::path::Path) -> crate::item_templates::ItemTemplates {
    world_products(path).1.clone()
}

#[test]
fn toyfx8_scenery_learns_spawns_source_models_and_expires_without_following() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{ModelDisplay, Npc, Position, UnitFactionTemplate, UnitSummonedBy};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    for (item, spell, entry, displays, lifetime_frames) in [
        (88381, 127771, 65298, vec![41007, 41008], 400),
        (88584, 128180, 65387, vec![41838], 1200),
    ] {
        let (mut h, character) = setup(&directory.path().join(format!("{item}.redb")));
        h.app.insert_resource(shared_world_data(&path));
        h.app
            .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
        learn_effect_toy(&mut h, character, item);
        let caster_position = *h.app.world().get::<Position>(h.caster).unwrap();
        // Both source effects use target87, requiring the actual clicked destination.
        let placed = Position {
            x: caster_position.x + 2.0,
            y: caster_position.y,
            z: caster_position.z + 1.0,
        };
        assert_eq!(
            activate_at(&mut h, item, Some([placed.x, placed.y, placed.z]))[0].error,
            None,
            "item {item}"
        );
        h.advance(10);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
        let world = h.app.world_mut();
        let summons: Vec<_> = world
            .query::<(Entity, &Npc, &SummonedCreature)>()
            .iter(world)
            .filter(|(_, npc, summon)| npc.template_id == entry && summon.spell_id == spell)
            .map(|(entity, _, _)| entity)
            .collect();
        assert_eq!(summons.len(), 1);
        let summon = summons[0];
        assert!(displays.contains(&world.get::<ModelDisplay>(summon).unwrap().display_id));
        assert_eq!(
            world.get::<UnitSummonedBy>(summon).unwrap().0,
            h.caster.to_bits()
        );
        assert_eq!(world.get::<Position>(summon), Some(&placed));
        assert_eq!(
            world.get::<crate::maps::InMap>(summon),
            world.get::<crate::maps::InMap>(h.caster)
        );
        assert_eq!(
            world.get::<UnitFactionTemplate>(summon),
            world.get::<UnitFactionTemplate>(h.caster)
        );
        assert!(
            world
                .get::<crate::creature_ai::FollowOwner>(summon)
                .is_none()
        );
        world.get_mut::<Position>(h.caster).unwrap().x += 5.0;
        h.advance(lifetime_frames - 30);
        assert_eq!(h.app.world().get::<Position>(summon), Some(&placed));
        h.advance(40);
        assert!(
            h.app.world().get_entity(summon).is_err(),
            "authored duration must expire"
        );
    }
}

#[test]
fn toyfx8_scenery_census() {
    let path = toyfx8_world();
    let report = report_toy_census(&path, "TOYFX8_CENSUS_REPORT");
    for item in [88381, 88584] {
        let toy = report["toys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|toy| toy["item_id"] == item)
            .unwrap();
        assert!(toy["unavailable"].is_null(), "{toy}");
    }
    for item in [88387, 88579, 193478, 198156, 219387] {
        let toy = report["toys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|toy| toy["item_id"] == item)
            .unwrap();
        assert!(
            !toy["unavailable"].is_null(),
            "interaction toys must stay blocked: {toy}"
        );
    }
}

#[test]
fn toyfx9_banners_and_sign_learn_spawn_at_source_destinations_and_expire() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{
        ModelDisplay, Npc, Position, Rotation, UnitFactionTemplate, UnitSummonedBy,
    };
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    for (item, spell, entry, display) in [
        (163986, 280827, 144025, 88125),
        (163987, 280828, 144026, 88126),
        (187834, 360123, 181826, 102568),
        (187957, 361085, 182459, 103372),
        (187958, 361090, 182461, 103384),
        (197719, 381262, 193182, 108105),
        (203716, 401253, 201798, 111503),
        (208057, 417591, 208477, 113602),
        (211424, 429364, 214220, 115934),
        (218128, 442240, 220620, 118153),
        (232301, 469612, 233144, 123650),
        (232302, 469613, 233145, 123654),
        (232303, 469614, 233146, 123655),
        (232305, 469616, 233148, 123656),
        (232306, 469617, 233149, 123658),
        (232307, 469618, 233150, 123659),
    ] {
        let race = if item == 163986 { 2 } else { 1 };
        let (mut h, character) =
            setup_for_race(&directory.path().join(format!("{item}.redb")), race);
        place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_item_templates(&path));
        h.app.insert_resource(shared_world_data(&path));
        h.app
            .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
        h.app.world_mut().entity_mut(h.caster).insert(Rotation {
            x: 0.0,
            y: std::f32::consts::PI,
            z: 0.0,
        });
        learn_effect_toy(&mut h, character, item);
        if matches!(item, 163986 | 163987) {
            let wrong_race = if race == 2 { 1 } else { 2 };
            h.app
                .world_mut()
                .get_mut::<shared::components::Player>(h.caster)
                .unwrap()
                .race = wrong_race;
            assert_eq!(
                activate(&mut h, item)[0].error.as_deref(),
                Some("toy race/class restriction")
            );
            assert!(h.inbox.go.is_empty());
            h.app
                .world_mut()
                .get_mut::<shared::components::Player>(h.caster)
                .unwrap()
                .race = race;
        }
        let caster_position = *h.app.world().get::<Position>(h.caster).unwrap();
        let is_sign = item == 197719;
        // Target47 clamps the source1yd war-banner radius to player reach1.5yd;
        // seasonal banners retain their authored2yd radius.
        let forward_distance = if is_sign {
            0.0
        } else if matches!(item, 163986 | 163987) {
            1.5
        } else {
            2.0
        };
        let expected_position = Position {
            x: caster_position.x,
            y: caster_position.y,
            z: caster_position.z - forward_distance,
        };
        assert_eq!(activate(&mut h, item)[0].error, None, "item {item}");
        if is_sign {
            assert!(
                !h.inbox.go.iter().any(|go| go.spell_id == spell),
                "sign has a 2000ms cast"
            );
        }
        h.advance(if is_sign { 45 } else { 10 });
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(
            h.inbox.go.iter().any(|go| go.spell_id == spell),
            "item {item}"
        );
        let world = h.app.world_mut();
        let summons: Vec<_> = world
            .query::<(Entity, &Npc, &SummonedCreature)>()
            .iter(world)
            .filter(|(_, npc, summon)| npc.template_id == entry && summon.spell_id == spell)
            .map(|(entity, _, _)| entity)
            .collect();
        assert_eq!(summons.len(), 1, "item {item}");
        let summon = summons[0];
        assert_eq!(
            world.get::<ModelDisplay>(summon).unwrap().display_id,
            display
        );
        assert_eq!(
            world.get::<UnitSummonedBy>(summon).unwrap().0,
            h.caster.to_bits()
        );
        let position = world.get::<Position>(summon).unwrap();
        assert!(
            (position.x - expected_position.x).abs() < 0.001,
            "item {item}: {position:?}"
        );
        assert!(
            (position.z - expected_position.z).abs() < 0.001,
            "item {item}: {position:?}"
        );
        assert_eq!(position.y, expected_position.y);
        assert_eq!(
            world.get::<crate::maps::InMap>(summon),
            world.get::<crate::maps::InMap>(h.caster)
        );
        let source_faction = if matches!(item, 163986 | 163987) {
            35
        } else {
            0
        };
        assert_eq!(
            world.get::<UnitFactionTemplate>(summon).unwrap().0,
            source_faction
        );
        assert!(
            world
                .get::<crate::creature_ai::FollowOwner>(summon)
                .is_none()
        );
        let stationary_position = *position;
        world.get_mut::<Position>(h.caster).unwrap().x += 5.0;
        let lifetime_frames = if is_sign { 6000 } else { 1200 };
        h.advance(lifetime_frames - 30);
        assert_eq!(
            h.app.world().get::<Position>(summon),
            Some(&stationary_position)
        );
        h.advance(40);
        assert!(
            h.app.world().get_entity(summon).is_err(),
            "authored lifetime: item {item}"
        );
    }
}

#[test]
fn toyfx9_prepared_census_preserves_interaction_blockers() {
    let path = toyfx8_world();
    let report = report_toy_census(&path, "TOYFX9_CENSUS_REPORT");
    let toys = report["toys"].as_array().unwrap();
    for item in [
        163986, 163987, 187834, 187957, 187958, 197719, 203716, 208057, 211424, 218128, 232301,
        232302, 232303, 232305, 232306, 232307, 238852,
    ] {
        let toy = toys.iter().find(|toy| toy["item_id"] == item).unwrap();
        assert!(toy["unavailable"].is_null(), "{toy}");
    }
    for item in [88387, 88579, 193478, 198156, 219387] {
        let toy = toys.iter().find(|toy| toy["item_id"] == item).unwrap();
        assert!(
            !toy["unavailable"].is_null(),
            "unproven interaction/scale: {toy}"
        );
    }
}

#[test]
fn toyfx13_orange_paint_targets_friend_and_expires_after_six_seconds() {
    assert_toyfx12_aura(88377, 127803, 6_000, true);
}

#[test]
fn toyfx13_shell_applies_all_three_source_auras_and_movement_removes_them() {
    let state = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&state.path().join("toys.redb"));
    let friend = spawn_player(&mut h.app, 8, 3.0);
    h.select(Some(friend));
    h.advance(2);
    let position = *h
        .app
        .world()
        .get::<shared::components::Position>(h.caster)
        .unwrap();
    let health = *h
        .app
        .world()
        .get::<shared::components::Health>(h.caster)
        .unwrap();
    learn_effect_toy(&mut h, character, 88417);
    assert_eq!(activate(&mut h, 88417)[0].error, None);
    h.advance(80);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 128150));
    // Native trigger64 children apply their auras without a separate SpellGo.
    for spell in [128150, 128151, 128152] {
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == spell)
            .unwrap();
        assert_eq!(aura.duration_ms, 120_000);
        assert_eq!(aura.caster, h.caster);
        assert!(
            h.app
                .world()
                .get::<shared::components::UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|aura| aura.spell_id == spell)
        );
        assert!(
            !h.app
                .world()
                .get::<shared::components::UnitAuras>(friend)
                .unwrap()
                .auras
                .iter()
                .any(|aura| aura.spell_id == spell)
        );
    }
    assert_eq!(
        *h.app
            .world()
            .get::<shared::components::Position>(h.caster)
            .unwrap(),
        position
    );
    assert_eq!(
        *h.app
            .world()
            .get::<shared::components::Health>(h.caster)
            .unwrap(),
        health
    );
    assert!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(h.caster)
            .is_none()
    );
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::networking::Moving(true));
    h.advance(2);
    for spell in [128150, 128151, 128152] {
        assert!(
            !h.app
                .world()
                .get::<shared::components::UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|aura| aura.spell_id == spell)
        );
    }
    let world = h.app.world_mut();
    assert_eq!(
        world
            .query::<&crate::spell_cast::summon::SummonedCreature>()
            .iter(world)
            .count(),
        0
    );
    assert_eq!(
        world
            .query::<&crate::game_objects::summoned::SummonedObject>()
            .iter(world)
            .count(),
        0
    );
}

#[test]
fn toyfx13_shell_expires_with_both_linked_visuals_after_two_minutes() {
    let state = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&state.path().join("toys.redb"));
    learn_effect_toy(&mut h, character, 88417);
    assert_eq!(activate(&mut h, 88417)[0].error, None);
    h.advance(80);
    let auras = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap();
    let expires_at = auras
        .iter()
        .filter_map(|aura| aura.expires_at)
        .fold(0.0, f64::max);
    let until_near_expiry = expires_at - crate::auras::aura_now(h.app.world()) - 1.0;
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .advance_by(std::time::Duration::from_secs_f64(until_near_expiry));
    h.advance(2);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .filter(|aura| [128150, 128151, 128152].contains(&aura.spell_id))
            .count(),
        3
    );
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .advance_by(std::time::Duration::from_secs(2));
    h.advance(2);
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| [128150, 128151, 128152].contains(&aura.spell_id))
    );
}

#[test]
fn toyfx13_audited_slots_do_not_admit_changed_amounts_or_unhandled_siblings() {
    use crate::spell_info::SpellStore;
    for item in [88377, 88417] {
        let state = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&state.path().join("toys.redb"));
        learn_effect_toy(&mut h, character, item);
        let spell_ids: &[u32] = if item == 88377 {
            &[127803]
        } else {
            &[128150, 128151, 128152]
        };
        let store = h.app.world().resource::<SpellStore>();
        let mut spells: Vec<_> = spell_ids
            .iter()
            .map(|id| store.get(*id).unwrap().clone())
            .collect();
        if item == 88377 {
            spells[0].effects[0].base_points = 2.0;
        } else {
            // The admitted root aura does not authorize a changed sibling script.
            spells[0].effects[1].effect = 77;
        }
        h.app.insert_resource(SpellStore::fixture(spells));
        let result = activate(&mut h, item);
        assert!(
            result[0]
                .error
                .as_deref()
                .unwrap()
                .contains("unsupported toy spell")
        );
        h.advance(80);
        assert!(h.inbox.go.is_empty());
        assert!(h.inbox.cooldowns.is_empty());
        assert!(
            h.app
                .world()
                .get::<shared::components::UnitAuras>(h.caster)
                .unwrap()
                .auras
                .is_empty()
        );
    }
}

#[test]
fn toyfx13_shared_census_preserves_control_refusals_and_unbound_scripts() {
    let report = report_toy_census(&crate::test_data::world_db_path(), "TOYFX13_CENSUS_REPORT");
    let toys = report["toys"].as_array().unwrap();
    for item in [88377, 88417, 1973, 115503, 64361] {
        let toy = toys.iter().find(|toy| toy["item_id"] == item).unwrap();
        assert!(toy["unavailable"].is_null(), "{toy}");
    }
    for item in [35275, 133997, 134007, 48933, 199902, 210974, 249468] {
        let toy = toys.iter().find(|toy| toy["item_id"] == item).unwrap();
        assert!(!toy["unavailable"].is_null(), "underived behavior: {toy}");
    }
}

#[test]
fn toyfx12_helheim_spirit_applies_to_caster_and_expires_after_thirty_minutes() {
    assert_toyfx12_aura(129149, 193333, 1_800_000, false);
}

#[test]
fn toyfx12_heartbreaker_applies_to_friend_and_expires_after_sixty_seconds() {
    assert_toyfx12_aura(50471, 71909, 60_000, true);
}

#[test]
fn toyfx12_fist_bump_applies_to_selected_friend_and_expires_after_one_day() {
    assert_toyfx12_aura(205045, 408067, 86_400_000, true);
}

fn assert_toyfx12_aura(item: u32, spell: u32, duration_ms: u32, on_friend: bool) {
    use shared::components::{Health, UnitAuras};
    let state = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&state.path().join("toys.redb"));
    let friend = spawn_player(&mut h.app, 8, 3.0);
    h.select(Some(friend));
    h.advance(2);
    let target = if on_friend { friend } else { h.caster };
    let other = if on_friend { h.caster } else { friend };
    let before = *h.app.world().get::<Health>(target).unwrap();
    learn_effect_toy(&mut h, character, item);
    assert_eq!(activate(&mut h, item)[0].error, None, "item {item}");
    h.advance(40);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell)
        .unwrap();
    assert_eq!(aura.duration_ms, duration_ms);
    assert_eq!(aura.caster, h.caster);
    let expires_at = aura.expires_at.unwrap();
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(target)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == spell && aura.caster == Some(h.caster.to_bits()))
    );
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(other)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == spell)
    );
    assert_eq!(*h.app.world().get::<Health>(target).unwrap(), before);
    assert!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(target)
            .is_none()
    );
    assert!(
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character)
            .items
            .is_empty()
    );
    // Advance the actual virtual clock, not an aura's expiry field. The next
    // normal frame runs production expiry and replication with a bounded delta.
    let until_near_expiry = expires_at - crate::auras::aura_now(h.app.world()) - 1.0;
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .advance_by(std::time::Duration::from_secs_f64(until_near_expiry));
    h.advance(2);
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(target)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == spell)
    );
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .advance_by(std::time::Duration::from_secs(2));
    h.advance(2);
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(target)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == spell)
    );
}

#[test]
fn toyfx12_abyss_horn_casts_visual_without_creating_gameplay_water_or_actor() {
    let state = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&state.path().join("toys.redb"));
    learn_effect_toy(&mut h, character, 211931);
    let position = *h
        .app
        .world()
        .get::<shared::components::Position>(h.caster)
        .unwrap();
    let health = *h
        .app
        .world()
        .get::<shared::components::Health>(h.caster)
        .unwrap();
    assert_eq!(activate(&mut h, 211931)[0].error, None);
    h.advance(80);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 430760));
    assert_eq!(
        *h.app
            .world()
            .get::<shared::components::Position>(h.caster)
            .unwrap(),
        position
    );
    assert_eq!(
        *h.app
            .world()
            .get::<shared::components::Health>(h.caster)
            .unwrap(),
        health
    );
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == 430760)
    );
    let world = h.app.world_mut();
    assert_eq!(
        world
            .query::<&crate::spell_cast::summon::SummonedCreature>()
            .iter(world)
            .count(),
        0
    );
    assert_eq!(
        world
            .query::<&crate::game_objects::summoned::SummonedObject>()
            .iter(world)
            .count(),
        0
    );
}

#[test]
fn toyfx12_prepared_census_retains_underivable_non_summon_scripts() {
    let path = toyfx8_world();
    let report = report_toy_census(&path, "TOYFX12_CENSUS_REPORT");
    let toys = report["toys"].as_array().unwrap();
    for item in [50471, 88377, 129149, 205045, 211931, 1973] {
        let toy = toys.iter().find(|toy| toy["item_id"] == item).unwrap();
        assert!(toy["unavailable"].is_null(), "{toy}");
    }
    for item in [35275, 133997, 134007, 48933, 199902, 210974, 249468] {
        let toy = toys.iter().find(|toy| toy["item_id"] == item).unwrap();
        assert!(!toy["unavailable"].is_null(), "underived behavior: {toy}");
    }
}

#[test]
fn toyfx10_iron_belly_spirits_affects_friend_for_sixty_seconds() {
    assert_toyfx10_cosmetic_aura(88587, 128327, 128327, 1200);
}

#[test]
fn toyfx10_darkshard_fragment_affects_caster_for_five_seconds() {
    assert_toyfx10_cosmetic_aura(131812, 196783, 196783, 100);
}

#[test]
fn toyfx10_aqir_egg_missile_triggers_fifteen_second_target_aura() {
    assert_toyfx10_cosmetic_aura(175063, 318453, 318452, 300);
}

fn assert_toyfx10_cosmetic_aura(item: u32, root: u32, aura_spell: u32, lifetime_frames: usize) {
    let path = toyfx8_world();
    let state = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&state.path().join("toys.redb"));
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_world_data(&path));
    h.app.insert_resource(shared_item_templates(&path));
    let distance = if item == 175063 { 8.0 } else { 3.0 };
    let friend = spawn_player(&mut h.app, 8, distance);
    let target = if item == 131812 { h.caster } else { friend };
    h.select(Some(friend));
    h.advance(2);
    learn_effect_toy(&mut h, character, item);
    let before = *h
        .app
        .world()
        .get::<shared::components::Health>(target)
        .unwrap();
    assert_eq!(activate(&mut h, item)[0].error, None, "item {item}");
    if item == 88587 {
        assert!(
            !h.inbox.go.iter().any(|go| go.spell_id == root),
            "1000ms cast unfinished"
        );
    }
    if item == 175063 {
        assert!(
            !h.app
                .world()
                .get::<shared::components::UnitAuras>(target)
                .is_some_and(|auras| auras.auras.iter().any(|aura| aura.spell_id == aura_spell)),
            "eight-yard missile must not apply its aura immediately"
        );
    }
    h.advance(40); // Includes the Spirits' 1000ms cast and missile flight.
    assert!(
        h.inbox.failed.is_empty(),
        "item {item}: {:?}",
        h.inbox.failed
    );
    assert!(
        h.inbox.go.iter().any(|go| go.spell_id == root),
        "item {item}"
    );
    let aura = h
        .app
        .world()
        .get::<shared::components::UnitAuras>(target)
        .unwrap()
        .auras
        .iter()
        .find(|aura| aura.spell_id == aura_spell)
        .unwrap();
    assert_eq!(aura.caster, Some(h.caster.to_bits()));
    let other = if target == h.caster { friend } else { h.caster };
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(other)
            .is_some_and(|auras| auras.auras.iter().any(|aura| aura.spell_id == aura_spell))
    );
    assert_eq!(
        *h.app
            .world()
            .get::<shared::components::Health>(target)
            .unwrap(),
        before
    );
    assert!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(target)
            .is_none()
    );
    assert!(
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character)
            .items
            .is_empty()
    );
    h.advance(lifetime_frames - 60);
    assert!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(target)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == aura_spell),
        "item {item} still active"
    );
    h.advance(80);
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(target)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == aura_spell),
        "item {item} expiry"
    );
}

#[test]
fn toyfx10_prepared_census_retains_script_and_interaction_refusals() {
    let path = toyfx8_world();
    let report = report_toy_census(&path, "TOYFX10_CENSUS_REPORT");
    let toys = report["toys"].as_array().unwrap();
    for item in [88417, 88587, 131812, 175063, 238852] {
        let toy = toys.iter().find(|toy| toy["item_id"] == item).unwrap();
        assert!(toy["unavailable"].is_null(), "{toy}");
    }
    for item in [48933, 88387, 95567, 138873, 198156, 219387] {
        let toy = toys.iter().find(|toy| toy["item_id"] == item).unwrap();
        assert!(!toy["unavailable"].is_null(), "unproved action: {toy}");
    }
}

#[test]
fn toyfx11_sweeper_and_trophy_learn_spawn_and_expire() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{
        ModelDisplay, Npc, Position, Rotation, UnitAuras, UnitFactionTemplate,
    };
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    for (item, spell, entry, display, radius) in [
        // Target47 clamps the source1yd radius to the default player's1.5yd reach.
        (147867, 243209, 121762, 76543, 1.5_f32),
        (169865, 362988, 183727, 92396, 1.5_f32),
    ] {
        let (mut h, character) = setup(&directory.path().join(format!("{item}.redb")));
        place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_world_data(&path));
        h.app
            .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
        learn_effect_toy(&mut h, character, item);
        let origin = *h.app.world().get::<Position>(h.caster).unwrap();
        let facing = 0.7_f32;
        h.app.world_mut().entity_mut(h.caster).insert(Rotation {
            x: 0.0,
            y: facing + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        });
        assert_eq!(activate(&mut h, item)[0].error, None, "item {item}");
        h.advance(5);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
        let world = h.app.world_mut();
        let actors: Vec<_> = world
            .query::<(Entity, &Npc, &SummonedCreature)>()
            .iter(world)
            .filter(|(_, npc, summon)| npc.template_id == entry && summon.spell_id == spell)
            .map(|(entity, _, _)| entity)
            .collect();
        assert_eq!(actors.len(), 1, "item {item}");
        let actor = actors[0];
        assert_eq!(
            world.get::<ModelDisplay>(actor).unwrap().display_id,
            display
        );
        let position = *world.get::<Position>(actor).unwrap();
        assert!((position.x - origin.x - radius * facing.cos()).abs() < 0.001);
        assert!((position.z - origin.z + radius * facing.sin()).abs() < 0.001);
        assert_eq!(position.y, origin.y);
        assert_eq!(
            world.get::<crate::maps::InMap>(actor),
            world.get::<crate::maps::InMap>(h.caster)
        );
        assert_eq!(
            world.get::<UnitFactionTemplate>(actor).unwrap().0,
            if item == 147867 { 35 } else { 0 }
        );
        assert_eq!(
            world.get::<SummonedCreature>(actor).unwrap().owner,
            h.caster
        );
        assert!(
            world
                .get::<crate::creature_ai::FollowOwner>(actor)
                .is_none()
        );
        let trophy_aura = world
            .get::<UnitAuras>(h.caster)
            .is_some_and(|auras| auras.auras.iter().any(|aura| aura.spell_id == 302730));
        assert_eq!(trophy_aura, item == 169865);
        world.get_mut::<Position>(h.caster).unwrap().x += 5.0;
        h.advance(5970); // Both source durations are 300000ms; check just before expiry.
        assert_eq!(h.app.world().get::<Position>(actor), Some(&position));
        if item == 169865 {
            assert!(
                h.app
                    .world()
                    .get::<UnitAuras>(h.caster)
                    .unwrap()
                    .auras
                    .iter()
                    .any(|aura| aura.spell_id == 302730)
            );
        }
        h.advance(50);
        assert!(h.app.world().get_entity(actor).is_err());
        assert!(
            !h.app
                .world()
                .get::<UnitAuras>(h.caster)
                .is_some_and(|auras| auras.auras.iter().any(|aura| aura.spell_id == 302730))
        );
        assert!(
            persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character)
                .items
                .is_empty()
        );
    }
}

#[test]
fn toyfx11_prepared_census_keeps_unbound_controls_and_actions_blocked() {
    let path = toyfx8_world();
    let report = report_toy_census(&path, "TOYFX11_CENSUS_REPORT");
    for item in [147867, 169865] {
        let toy = report["toys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|toy| toy["item_id"] == item)
            .unwrap();
        assert!(toy["unavailable"].is_null(), "{toy}");
    }
    for item in [
        37710, 54212, 198206, 198264, 210975, 88375, 88387, 144339, 198156, 219387,
    ] {
        let toy = report["toys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|toy| toy["item_id"] == item)
            .unwrap();
        assert!(
            !toy["unavailable"].is_null(),
            "unbound control/interaction must remain blocked: {toy}"
        );
    }
}

#[test]
fn toyfx14_source_item_preparation_enables_real_transform_learning_use_and_restore() {
    use shared::components::{ModelDisplay, UnitAuras};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("transform.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    let before = h.app.world().get::<ModelDisplay>(h.caster).cloned();
    learn_effect_toy(&mut h, character, 64646); // Bones of Transformation.
    assert_eq!(activate(&mut h, 64646)[0].error, None);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 91760));
    assert_eq!(
        h.app
            .world()
            .get::<ModelDisplay>(h.caster)
            .unwrap()
            .display_id,
        32473
    );
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 91760)
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura { spell_id: 91760 });
    h.advance(5);
    assert_eq!(h.app.world().get::<ModelDisplay>(h.caster), before.as_ref());
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 91760)
    );
}

#[test]
fn toyfx14_prepared_item_census_records_all_remaining_mechanics() {
    let path = toyfx8_world();
    let report = report_toy_census(&path, "TOYFX14_CENSUS_REPORT");
    for item in [64646, 109183, 113542] {
        let toy = report["toys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["item_id"] == item)
            .unwrap();
        assert!(toy["unavailable"].is_null(), "{toy}");
    }
    for item in [37710, 198156, 71259, 72231] {
        let toy = report["toys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["item_id"] == item)
            .unwrap();
        assert!(
            !toy["unavailable"].is_null(),
            "unbound remote control/wormhole/narrative/metadata: {toy}"
        );
    }
}

#[test]
fn toyfx14_orb_deception_uses_native_race_and_sex_and_restores() {
    use shared::components::{ModelDisplay, Player, UnitAuras};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    // TC a352b1fa SpellAuraEffects.cpp2110-2161, not inferred faction counterparts.
    for (race, male, female) in [
        (1, 10137, 10138),
        (2, 10139, 10140),
        (3, 10141, 10142),
        (4, 10143, 10144),
        (5, 10146, 10145),
        (6, 10136, 10147),
        (7, 10148, 10149),
        (8, 10135, 10134),
        (10, 17829, 17830),
        (11, 17827, 17828),
    ] {
        for sex in [0, 1] {
            let class = crate::race_class::playable_pairs()
                .find(|(native, _)| *native == race)
                .unwrap()
                .1;
            let (mut h, character) = setup_for_race_class(
                &directory.path().join(format!("orb-{race}-{sex}.redb")),
                race,
                class,
            );
            h.app.insert_resource(toy_catalog::load(&conn).unwrap());
            h.app.insert_resource(shared_item_templates(&path));
            h.app.insert_resource(
                crate::spell_cast::test_harness::world_db()
                    .unwrap()
                    .store
                    .clone(),
            );
            h.app
                .world_mut()
                .get_mut::<Player>(h.caster)
                .unwrap()
                .appearance
                .sex = sex;
            let native = h.app.world().get::<ModelDisplay>(h.caster).cloned();
            let other = h.app.world().get::<ModelDisplay>(h.npc).cloned();
            learn_effect_toy(&mut h, character, 1973);
            assert_eq!(
                activate(&mut h, 1973)[0].error,
                None,
                "race {race},sex {sex}"
            );
            h.advance(5);
            assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
            assert!(h.inbox.go.iter().any(|go| go.spell_id == 16739));
            let display = if sex == 1 { female } else { male };
            assert_eq!(
                h.app
                    .world()
                    .get::<ModelDisplay>(h.caster)
                    .unwrap()
                    .display_id,
                display
            );
            let player = h.app.world().get::<Player>(h.caster).unwrap();
            assert_eq!((player.race, player.appearance.sex), (race, sex));
            assert_eq!(h.app.world().get::<ModelDisplay>(h.npc), other.as_ref());
            assert!(
                h.app
                    .world()
                    .get::<UnitAuras>(h.caster)
                    .unwrap()
                    .auras
                    .iter()
                    .any(|a| a.spell_id == 16739)
            );
            h.app
                .world_mut()
                .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
                .unwrap()
                .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
                    spell_id: 16739,
                });
            h.advance(5);
            assert_eq!(h.app.world().get::<ModelDisplay>(h.caster), native.as_ref());
            assert!(
                !h.app
                    .world()
                    .get::<UnitAuras>(h.caster)
                    .unwrap()
                    .auras
                    .iter()
                    .any(|a| a.spell_id == 16739)
            );
        }
    }
    // No source mapping for Goblin9: no successful cosmetic no-op or spent cooldown.
    let (mut h, character) = setup_for_race(&directory.path().join("orb-unmapped.redb"), 9);
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(
        crate::spell_cast::test_harness::world_db()
            .unwrap()
            .store
            .clone(),
    );
    learn_effect_toy(&mut h, character, 1973);
    let result = activate(&mut h, 1973);
    assert!(
        result[0]
            .error
            .as_ref()
            .unwrap()
            .contains("no source display mapping for race 9")
    );
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == 16739));
    assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == 16739));
}

#[test]
fn toyfx14_zero_entry_costumes_use_authentic_gender_models_and_restore() {
    use shared::components::{ModelDisplay, Player, UnitAuras};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    for (item, spell, race, male, female) in [
        (33079, 42365, 1, 21723, 21723),
        (54651, 75531, 1, 31654, 31655),
        (54653, 75532, 2, 31737, 31738),
    ] {
        for sex in [0, 1] {
            let (mut h, character) = setup_for_race(
                &directory.path().join(format!("costume-{item}-{sex}.redb")),
                race,
            );
            h.app.insert_resource(toy_catalog::load(&conn).unwrap());
            h.app.insert_resource(shared_item_templates(&path));
            h.app
                .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
            h.app
                .world_mut()
                .get_mut::<Player>(h.caster)
                .unwrap()
                .appearance
                .sex = sex;
            let before = h.app.world().get::<ModelDisplay>(h.caster).cloned();
            let other_before = h.app.world().get::<ModelDisplay>(h.npc).cloned();
            learn_effect_toy(&mut h, character, item);
            assert_eq!(
                activate(&mut h, item)[0].error,
                None,
                "item {item},sex {sex}"
            );
            if spell == 42365 {
                assert!(
                    !h.inbox.go.iter().any(|go| go.spell_id == spell),
                    "Murloc's authored6000ms cast must finish first"
                );
            }
            h.advance(140);
            assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
            assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
            let display = if sex == 1 { female } else { male };
            assert_eq!(
                h.app
                    .world()
                    .get::<ModelDisplay>(h.caster)
                    .unwrap()
                    .display_id,
                display
            );
            assert_eq!(
                h.app.world().get::<ModelDisplay>(h.npc),
                other_before.as_ref()
            );
            assert!(
                h.app
                    .world()
                    .get::<UnitAuras>(h.caster)
                    .unwrap()
                    .auras
                    .iter()
                    .any(|a| a.spell_id == spell)
            );
            h.app
                .world_mut()
                .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
                .unwrap()
                .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
                    spell_id: spell,
                });
            h.advance(5);
            assert_eq!(h.app.world().get::<ModelDisplay>(h.caster), before.as_ref());
            assert!(
                !h.app
                    .world()
                    .get::<UnitAuras>(h.caster)
                    .unwrap()
                    .auras
                    .iter()
                    .any(|a| a.spell_id == spell)
            );
        }
    }
}

#[test]
fn toyfx14_authored_caster_attachments_learn_use_replicate_and_cancel_without_gameplay() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (item, spell, duration) in [
        (108743, 160688, 600000),
        (110586, 160093, 180000),
        (116115, 170869, 900000),
        (127864, 188172, 3600000),
        (137294, 212660, 300000),
        (147537, 242014, 3600000),
        (163744, 280053, 300000),
        (180947, 335276, 900000),
        (182890, 340276, 180000),
        (188699, 45416, 0), // Source duration-1: permanent until cancelled.
        (188701, 45418, 900000),
        (194056, 383065, 0), // Source duration-1: permanent until cancelled.
        (204220, 403491, 300000),
        (230727, 467285, 1800000),
        (259240, 1264653, 300000),
    ] {
        let (mut h, character) = setup(&directory.path().join(format!("attachment-{item}.redb")));
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_item_templates(&path));
        h.app.insert_resource(store.clone());
        let model_before = h.app.world().get::<ModelDisplay>(h.caster).cloned();
        let caster_health = *h.app.world().get::<Health>(h.caster).unwrap();
        let target_health = *h.app.world().get::<Health>(h.npc).unwrap();
        learn_effect_toy(&mut h, character, item);
        assert_eq!(activate(&mut h, item)[0].error, None, "item {item}");
        h.advance(60);
        assert!(
            h.inbox.failed.is_empty(),
            "item {item}: {:?}",
            h.inbox.failed
        );
        assert!(
            h.inbox.go.iter().any(|go| go.spell_id == spell),
            "item {item}"
        );
        let aura = h
            .app
            .world()
            .get::<UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .find(|a| a.spell_id == spell)
            .unwrap();
        assert_eq!(
            aura.duration_ms, duration,
            "authored lifetime of item {item}"
        );
        assert_eq!(
            h.app.world().get::<ModelDisplay>(h.caster),
            model_before.as_ref()
        );
        assert_eq!(
            *h.app.world().get::<Health>(h.caster).unwrap(),
            caster_health
        );
        assert_eq!(*h.app.world().get::<Health>(h.npc).unwrap(), target_health);
        assert_eq!(
            h.app
                .world_mut()
                .query::<&crate::spell_cast::summon::SummonedCreature>()
                .iter(h.app.world())
                .count(),
            0
        );
        h.app
            .world_mut()
            .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
            .unwrap()
            .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
                spell_id: spell,
            });
        h.advance(5);
        assert!(
            !h.app
                .world()
                .get::<UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|a| a.spell_id == spell)
        );
    }
}

#[test]
fn toyfx14_stationary_source_summons_learn_spawn_and_expire() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{ModelDisplay, Npc, Position, UnitSummonedBy};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (item, spell, entry, display, duration) in [
        (170197, 438546, 218978, 117639, 60000),
        (104331, 148627, 73781, 51525, 60000),
        (119039, 176310, 88280, 51525, 60000),
        (170196, 304505, 156052, 92597, 60000),
        (89869, 131171, 66768, 45436, 150000),
        (141299, 225824, 114846, 73493, 60000),
        (184415, 345799, 175582, 99948, 180000),
        (201931, 395720, 199047, 112284, 300000),
        (202261, 398246, 200350, 114420, 300000),
        (202851, 398245, 200961, 112316, 300000),
        (202856, 399246, 200966, 112319, 300000),
        (202859, 399250, 200969, 112322, 300000),
        (202862, 399253, 200972, 112325, 300000),
        (202865, 399257, 200975, 112328, 300000),
        (272339, 1290957, 263908, 143906, 60000),
        (279590, 1306978, 270689, 146432, 60000),
    ] {
        let (mut h, character) = setup(&directory.path().join(format!("scenery-{item}.redb")));
        place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_item_templates(&path));
        h.app.insert_resource(shared_world_data(&path));
        h.app
            .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
        h.app.insert_resource(store.clone());
        learn_effect_toy(&mut h, character, item);
        // Keep each mini's source-speed missile in flight during the first pump.
        let (missile_root, cursor_distance) = match item {
            201931 => (Some(398485), 12.0), // Maiev: speed 12, one-second flight.
            202261 => (Some(398491), 18.0), // Sneed: speed 30, 0.6-second flight.
            202851 => (Some(399232), 18.0), // Huntress: speed 30, 0.6-second flight.
            202856 => (Some(399247), 18.0), // Tauren: speed 30, 0.6-second flight.
            202859 => (Some(399262), 18.0), // Ghoul: speed 30, 0.6-second flight.
            202862 => (Some(399265), 18.0), // Murloc: speed 30, 0.6-second flight.
            202865 => (Some(399268), 18.0), // Whelp Egg: speed 30, 0.6-second flight.
            _ => (None, 2.0),
        };
        let destination = if matches!(item, 89869 | 184415) || missile_root.is_some() {
            let origin = h.app.world().get::<Position>(h.caster).unwrap();
            Some([origin.x + cursor_distance, origin.y, origin.z])
        } else {
            None
        };
        assert_eq!(
            activate_at(&mut h, item, destination)[0].error,
            None,
            "item {item}"
        );
        if let Some(root_spell) = missile_root {
            assert!(h.inbox.go.iter().any(|go| go.spell_id == root_spell));
            let world = h.app.world_mut();
            let early_actors = world
                .query::<(&Npc, &SummonedCreature)>()
                .iter(world)
                .filter(|(npc, summon)| npc.template_id == entry && summon.spell_id == spell)
                .count();
            assert_eq!(
                early_actors, 0,
                "item {item}: actor appeared before source flight"
            );
        }
        h.advance(60); // Includes the globe's authored 3000ms cast.
        assert!(
            h.inbox.failed.is_empty(),
            "item {item}: {:?}",
            h.inbox.failed
        );
        if missile_root.is_none() {
            assert!(
                h.inbox.go.iter().any(|go| go.spell_id == spell),
                "item {item}"
            );
        }
        let world = h.app.world_mut();
        let actors: Vec<_> = world
            .query::<(Entity, &Npc, &SummonedCreature)>()
            .iter(world)
            .filter(|(_, npc, summon)| npc.template_id == entry && summon.spell_id == spell)
            .map(|(entity, _, _)| entity)
            .collect();
        assert_eq!(actors.len(), 1, "item {item}");
        let actor = actors[0];
        assert_eq!(
            world.get::<ModelDisplay>(actor).unwrap().display_id,
            display
        );
        let expected_scale = if matches!(item, 170196 | 170197) {
            0.6
        } else {
            1.0
        };
        let scale = world
            .get::<shared::components::UnitScale>(actor)
            .copied()
            .unwrap_or_default()
            .0;
        assert!((scale - expected_scale).abs() < 0.00001, "item {item}");
        assert_eq!(
            world.get::<UnitSummonedBy>(actor).unwrap().0,
            h.caster.to_bits()
        );
        assert_eq!(
            world.get::<crate::maps::InMap>(actor),
            world.get::<crate::maps::InMap>(h.caster)
        );
        assert!(
            world
                .get::<crate::creature_ai::FollowOwner>(actor)
                .is_none()
        );
        let is_pet = world
            .get::<shared::components::UnitIsPet>(actor)
            .is_some_and(|pet| pet.0);
        assert!(!is_pet, "item {item}");
        let placed = *world.get::<Position>(actor).unwrap();
        if let Some(destination) = destination {
            assert!((placed.x - destination[0]).abs() < 0.001, "item {item}");
            assert!((placed.z - destination[2]).abs() < 0.001, "item {item}");
        }
        world.get_mut::<Position>(h.caster).unwrap().x += 5.0;
        let expires = world
            .get::<SummonedCreature>(actor)
            .unwrap()
            .expires_at
            .unwrap();
        let now = world.resource::<Time>().elapsed_secs_f64();
        let remaining = expires - now;
        assert!(
            remaining > duration as f64 / 1000.0 - 4.0 && remaining <= duration as f64 / 1000.0
        );
        h.app
            .world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(std::time::Duration::from_secs_f64(remaining));
        h.app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f64(remaining - 0.1),
            ));
        h.advance(1);
        assert_eq!(h.app.world().get::<Position>(actor), Some(&placed));
        h.app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(200),
            ));
        h.advance(1);
        assert!(
            h.app.world().get_entity(actor).is_err(),
            "authored expiry of item {item}"
        );
    }
}

#[test]
fn toyfx14_stationary_source_summon_census() {
    let path = toyfx8_world();
    let report = report_toy_census(&path, "TOYFX14_SUMMON_CENSUS_REPORT");
    for item in [
        104331, 119039, 170196, 170197, 89869, 141299, 184415, 272339, 279590,
    ] {
        let toy = report["toys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|toy| toy["item_id"] == item)
            .unwrap();
        assert!(toy["unavailable"].is_null(), "{toy}");
    }
}

#[test]
fn toyfx14_area_cosmetics_preserve_authored_recipients_and_lifetimes() {
    use shared::components::{
        Health, ModelDisplay, Position, Rotation, UnitAuras, UnitFactionTemplate,
    };
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (item, spell, duration) in [(141300, 225826, 10000), (141301, 225828, 30000)] {
        let (mut h, character) = setup(&directory.path().join(format!("area-{item}.redb")));
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_item_templates(&path));
        h.app.insert_resource(store.clone());
        h.app
            .insert_resource(crate::conditions::load_spell_target_conditions(&path).unwrap());
        let origin = *h.app.world().get::<Position>(h.caster).unwrap();
        let map = *h.app.world().get::<crate::maps::InMap>(h.caster).unwrap();
        let faction = *h.app.world().get::<UnitFactionTemplate>(h.caster).unwrap();
        h.app.world_mut().entity_mut(h.caster).insert(Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        });
        h.app.world_mut().entity_mut(h.npc).insert((
            faction,
            Position {
                x: origin.x + 2.0,
                ..origin
            },
        ));
        let outside = h
            .app
            .world_mut()
            .spawn((
                Position {
                    x: origin.x - 50.0,
                    ..origin
                },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                map,
                faction,
                UnitAuras::default(),
            ))
            .id();
        let model = h.app.world().get::<ModelDisplay>(h.npc).cloned();
        let health = *h.app.world().get::<Health>(h.npc).unwrap();
        learn_effect_toy(&mut h, character, item);
        assert_eq!(activate(&mut h, item)[0].error, None, "item {item}");
        h.advance(40);
        assert!(
            h.inbox.failed.is_empty(),
            "item {item}: {:?}",
            h.inbox.failed
        );
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
        let aura = h
            .app
            .world()
            .get::<UnitAuras>(h.npc)
            .unwrap()
            .auras
            .iter()
            .find(|a| a.spell_id == spell)
            .unwrap();
        assert_eq!(aura.duration_ms, duration);
        assert!(
            !h.app
                .world()
                .get::<UnitAuras>(outside)
                .unwrap()
                .auras
                .iter()
                .any(|a| a.spell_id == spell)
        );
        assert_eq!(h.app.world().get::<ModelDisplay>(h.npc), model.as_ref());
        assert_eq!(*h.app.world().get::<Health>(h.npc).unwrap(), health);
        h.advance((duration / 50) as usize + 10);
        assert!(
            !h.app
                .world()
                .get::<UnitAuras>(h.npc)
                .unwrap()
                .auras
                .iter()
                .any(|a| a.spell_id == spell)
        );
    }
}

#[test]
fn toyfx14_area_cosmetics_census() {
    let path = toyfx8_world();
    let report = report_toy_census(&path, "TOYFX14_AREA_CENSUS_REPORT");
    for item in [141300, 141301] {
        let toy = report["toys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|toy| toy["item_id"] == item)
            .unwrap();
        assert!(toy["unavailable"].is_null(), "{toy}");
    }
}

#[test]
fn toyfx14_ground_toy_waits_for_a_clicked_destination() {
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("ground.redb"));
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    learn_effect_toy(&mut h, character, 89869);
    assert_eq!(
        activate(&mut h, 89869)[0].error.as_deref(),
        Some("NoTarget")
    );
    assert!(
        h.inbox.go.is_empty(),
        "a cursor-only request must not cast at the player's feet"
    );
    let origin = *h
        .app
        .world()
        .get::<shared::components::Position>(h.caster)
        .unwrap();
    for (destination, error) in [
        ([f32::NAN, origin.y, origin.z], "InvalidTarget"),
        ([origin.x + 11.0, origin.y, origin.z], "OutOfRange"),
    ] {
        assert_eq!(
            activate_at(&mut h, 89869, Some(destination))[0]
                .error
                .as_deref(),
            Some(error)
        );
        assert!(h.inbox.go.is_empty());
    }
    let destination = [origin.x + 3.0, origin.y, origin.z];
    assert_eq!(activate_at(&mut h, 89869, Some(destination))[0].error, None);
    h.advance(5);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 131171));
    let world = h.app.world_mut();
    let positions: Vec<_> = world
        .query::<(&shared::components::Npc, &shared::components::Position)>()
        .iter(world)
        .filter(|(npc, _)| npc.template_id == 66768)
        .map(|(_, position)| *position)
        .collect();
    assert_eq!(
        positions,
        vec![shared::components::Position {
            x: destination[0],
            y: destination[1],
            z: destination[2]
        }]
    );
}

#[test]
fn toyfx14_target_and_sound_cosmetics_preserve_source_ownership() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (item, spell, duration) in [(122120, 179869, 60000), (168824, 298869, 180000)] {
        let (mut h, character) = setup(&directory.path().join(format!("target-sound-{item}.redb")));
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_item_templates(&path));
        h.app.insert_resource(store.clone());
        let target = if item == 122120 { h.npc } else { h.caster };
        let other = if item == 122120 { h.caster } else { h.npc };
        h.app
            .world_mut()
            .get_mut::<crate::networking::CombatTarget>(h.caster)
            .unwrap()
            .0 = Some(h.npc);
        let health = *h.app.world().get::<Health>(target).unwrap();
        let model = h.app.world().get::<ModelDisplay>(target).cloned();
        learn_effect_toy(&mut h, character, item);
        assert_eq!(activate(&mut h, item)[0].error, None, "item {item}");
        h.advance(50);
        assert!(
            h.inbox.failed.is_empty(),
            "item {item}: {:?}",
            h.inbox.failed
        );
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
        let aura = h
            .app
            .world()
            .get::<UnitAuras>(target)
            .unwrap()
            .auras
            .iter()
            .find(|a| a.spell_id == spell)
            .unwrap();
        assert_eq!(aura.duration_ms, duration);
        assert!(
            !h.app
                .world()
                .get::<UnitAuras>(other)
                .unwrap()
                .auras
                .iter()
                .any(|a| a.spell_id == spell)
        );
        assert_eq!(*h.app.world().get::<Health>(target).unwrap(), health);
        assert_eq!(h.app.world().get::<ModelDisplay>(target), model.as_ref());
    }
}

#[test]
fn toyfx14_target_and_sound_cosmetics_census() {
    let path = toyfx8_world();
    let report = report_toy_census(&path, "TOYFX14_SOUND_CENSUS_REPORT");
    for item in [122120, 168824] {
        let toy = report["toys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|toy| toy["item_id"] == item)
            .unwrap();
        assert!(toy["unavailable"].is_null(), "{toy}");
    }
}

#[test]
fn toyfx14_gaze_uses_authoritative_baked_sight() {
    use shared::components::{ModelDisplay, Position, Rotation, UnitAuras};
    use shared::protocol::WitnessRay;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("witness.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    let dir = crate::ground::ground_data_dir().join("los");
    crate::test_data::require(dir.join(los::eye::EYE_TABLE_FILE));
    h.app
        .insert_resource(crate::line_of_sight::LineOfSight::load(&dir).unwrap());
    // Actual Northshire west-door bake: source spell range30yd, clear door opening.
    h.app.world_mut().entity_mut(h.caster).insert((
        Position {
            x: -8908.2,
            y: 81.95,
            z: 146.6,
        },
        Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    ));
    h.app.world_mut().entity_mut(h.npc).insert((
        Position {
            x: -8896.5,
            y: 81.58,
            z: 173.8,
        },
        Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        ModelDisplay { display_id: 10913 },
    ));
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = Some(h.npc);
    learn_effect_toy(&mut h, character, 122120);
    let catalog = snapshot(h.app.world(), 1);
    assert!(
        catalog
            .toys
            .iter()
            .find(|toy| toy.item_id == 122120)
            .unwrap()
            .requires_target_witness
    );
    assert!(
        !catalog
            .toys
            .iter()
            .find(|toy| toy.item_id == 168824)
            .unwrap()
            .requires_target_witness
    );
    let world = h.app.world();
    let los = world.resource::<crate::line_of_sight::LineOfSight>();
    let body = |entity| {
        let unit = world.entity(entity);
        los.body((
            entity,
            unit.get::<shared::components::Npc>(),
            unit.get::<Position>().unwrap(),
            unit.get::<Rotation>(),
            unit.get::<ModelDisplay>(),
            None,
            unit.get::<shared::components::Player>(),
            unit.get::<shared::components::PlayerStandState>(),
        ))
        .unwrap()
    };
    let caster = body(h.caster);
    let target = body(h.npc);
    let end = los
        .witness(crate::maps::InMap::continent(0), &caster, &target)
        .expect("authored doorway has a clear ray");
    let witness = WitnessRay {
        start: caster.eyes.to_array(),
        end: end.to_array(),
    };
    // The sampled endpoint may age; authoritative clear sight decides the toy cast.
    let stale = WitnessRay {
        end: [witness.end[0] + 5.0, witness.end[1], witness.end[2]],
        ..witness
    };
    assert_eq!(
        activate_witnessed(&mut h, 122120, None, Some(stale))[0].error,
        None
    );
    h.advance(3);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 179869));
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(h.npc)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 179869 && a.duration_ms == 60000)
    );
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 179869)
    );
}

#[test]
fn toyfx14_scaled_scenery_preserves_source_display_scale_and_aura_object_scale() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{ModelDisplay, Npc, Position, UnitScale};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (item, spell, entry, display, scale) in [
        (170476, 304620, 156086, 92605, 0.35),
        (238852, 1227383, 241494, 127931, 0.75),
        (246227, 1240408, 246171, 131280, 0.5),
        (276229, 1299034, 267634, 145135, 1.5),
    ] {
        let (mut h, character) = setup(&directory.path().join(format!("scaled-{item}.redb")));
        place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_item_templates(&path));
        h.app.insert_resource(shared_world_data(&path));
        h.app
            .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
        h.app.insert_resource(store.clone());
        let origin = *h.app.world().get::<Position>(h.caster).unwrap();
        let destination = (item == 246227).then_some([origin.x + 2.0, origin.y, origin.z]);
        learn_effect_toy(&mut h, character, item);
        assert_eq!(
            activate_at(&mut h, item, destination)[0].error,
            None,
            "item {item}"
        );
        h.advance(50);
        assert!(
            h.inbox.failed.is_empty(),
            "item {item}: {:?}",
            h.inbox.failed
        );
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
        let world = h.app.world_mut();
        let actors: Vec<_> = world
            .query::<(Entity, &Npc, &SummonedCreature)>()
            .iter(world)
            .filter(|(_, npc, summon)| npc.template_id == entry && summon.spell_id == spell)
            .map(|(entity, _, _)| entity)
            .collect();
        assert_eq!(actors.len(), 1);
        let actor = actors[0];
        assert_eq!(
            world.get::<ModelDisplay>(actor).unwrap().display_id,
            display
        );
        assert_eq!(
            world.get::<UnitScale>(actor).copied().unwrap_or_default().0,
            scale
        );
        // A real scale aura must compose with display scale, not overwrite it.
        let growth = store.get(156132).unwrap(); // World Shrinker: authored +100% object scale.
        assert_eq!(growth.effects[0].aura, crate::auras::handlers::MOD_SCALE);
        let effect = &growth.effects[0];
        crate::auras::apply_aura(
            world,
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: actor,
                spell_id: 156132,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
        assert!(
            (world.get::<UnitScale>(actor).unwrap().0 - scale * (1.0 + effect.base_points / 100.0))
                .abs()
                < 0.00001
        );
        crate::auras::remove_auras(world, actor, crate::auras::RemoveReason::Cancelled, |a| {
            a.spell_id == 156132
        });
        assert_eq!(world.get::<UnitScale>(actor).unwrap().0, scale);
    }
}

#[test]
fn toyfx14_scaled_scenery_census() {
    let path = toyfx8_world();
    let report = report_toy_census(&path, "TOYFX14_SCALE_CENSUS_REPORT");
    for item in [170476, 238852, 246227, 276229] {
        let toy = report["toys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|toy| toy["item_id"] == item)
            .unwrap();
        assert!(toy["unavailable"].is_null(), "{toy}");
    }
}

#[test]
fn toyfx18_placed_statues_spawn_source_models_stationary_for_authored_lifetime() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{ModelDisplay, Npc, Position, UnitScale};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (item, spell, entry, displays, scale) in [
        (198646, 384618, 194793, vec![108683], 0.7),
        (204405, 405204, 203158, vec![111821], 1.0),
    ] {
        let (mut h, character) = setup(&directory.path().join(format!("statue-{item}.redb")));
        place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_item_templates(&path));
        h.app.insert_resource(shared_world_data(&path));
        h.app
            .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
        h.app.insert_resource(store.clone());
        learn_effect_toy(&mut h, character, item);
        assert_eq!(
            activate_at(&mut h, item, None)[0].error,
            None,
            "item {item}"
        );
        h.advance(50);
        assert!(
            h.inbox.failed.is_empty(),
            "item {item}: {:?}",
            h.inbox.failed
        );
        let now = crate::auras::aura_now(h.app.world());
        let world = h.app.world_mut();
        let actors: Vec<_> = world
            .query::<(Entity, &Npc, &SummonedCreature)>()
            .iter(world)
            .filter(|(_, npc, summon)| npc.template_id == entry && summon.spell_id == spell)
            .map(|(entity, _, summon)| (entity, summon.expires_at))
            .collect();
        assert_eq!(actors.len(), 1, "item {item}");
        let (actor, expires_at) = actors[0];
        // Authored SpellDuration 300000ms; 50 frames already elapsed.
        let remaining = expires_at.unwrap() - now;
        assert!(
            (297.0..=300.0).contains(&remaining),
            "item {item}: {remaining}"
        );
        assert!(displays.contains(&world.get::<ModelDisplay>(actor).unwrap().display_id));
        assert_eq!(
            world.get::<UnitScale>(actor).copied().unwrap_or_default().0,
            scale
        );
        assert!(
            world
                .get::<crate::creature_ai::FollowOwner>(actor)
                .is_none()
        );
        let placed = *world.get::<Position>(actor).unwrap();
        world.get_mut::<Position>(h.caster).unwrap().x += 5.0;
        h.advance(100);
        assert_eq!(h.app.world().get::<Position>(actor), Some(&placed));
    }
}

#[test]
fn toyfx21_required_area_refuses_northshire_without_cast_or_item_cooldown() {
    let directory = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&directory.path().join("outside-source-area.redb"));
    // Actual Human source start, Area9; source1103 permits Goldshire87, not Northshire.
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    learn(&mut h, 2);
    assert_eq!(
        activate(&mut h, 188698)[0].error.as_deref(),
        Some("Requires area group1103")
    );
    h.advance(5);
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == 61810));
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|cooldown| cooldown.spell_id == 61810)
    );
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .all(|aura| aura.spell_id != 61810)
    );
}

#[test]
fn toyfx21_allowed_area_exit_interrupts_aura_preserves_category_and_rejection_does_not_start_it() {
    let directory = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&directory.path().join("area-lifecycle.redb"));
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    learn(&mut h, 2);
    assert_eq!(
        activate(&mut h, 188698)[0].error.as_deref(),
        Some("Requires area group1103")
    );
    // Exact content_creature20280725 Cow2442, verified Goldshire MCNK Area87.
    place_toy_caster_on_source_area(&mut h, 778032, [-9480.94, -38.5858], 87);
    assert_eq!(activate(&mut h, 188698)[0].error, None);
    h.advance(2);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 61810)
    );
    // TDB1210 creature305516/41478, actual ADT777997 Kharanos Area131.
    place_toy_caster_on_source_area(&mut h, 777997, [-5437.77, -437.216], 131);
    h.advance(2);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 61810),
        "moving between permitted group members keeps the aura"
    );
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    h.advance(2);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .all(|a| a.spell_id != 61810)
    );
    place_toy_caster_on_source_area(&mut h, 778032, [-9480.94, -38.5858], 87);
    assert_eq!(
        activate(&mut h, 188698)[0].error.as_deref(),
        Some("OnCooldown"),
        "area exit must not clear item category history"
    );
    h.advance(201);
    assert_eq!(
        activate(&mut h, 188698)[0].error,
        None,
        "natural category expiry permits reuse"
    );
}

#[test]
fn toyfx21_all_69_source_roots_reject_real_northshire_and_absent_terrain_before_history() {
    let directory = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&directory.path().join("all-root-areas.redb"));
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/toy_required_areas.json")).unwrap();
    let spells = manifest["spells"].as_object().unwrap();
    assert_eq!(spells.len(), 69);
    h.app
        .insert_resource(crate::spell_info::SpellStore::fixture(
            spells
                .keys()
                .map(|spell| {
                    world_db()
                        .unwrap()
                        .store
                        .get(spell.parse().unwrap())
                        .unwrap()
                        .clone()
                })
                .collect(),
        ));
    for (spell, group) in spells {
        let spell: u32 = spell.parse().unwrap();
        for missing_location in 0..3 {
            if missing_location == 1 {
                h.app.world_mut().entity_mut(h.caster).remove::<Position>();
            }
            if missing_location == 2 {
                h.app.insert_resource(crate::ground::GroundMap::default());
            }
            let failure = crate::spell_cast::cast_granted(h.app.world_mut(), h.caster, spell, None)
                .unwrap_err();
            assert_eq!(
                failure.to_contract(),
                (
                    shared::spell_data::CastFailReason::IncorrectArea,
                    Some(format!("Requires area group{}", group.as_u64().unwrap()))
                ),
                "spell{spell}, missing_location={missing_location}"
            );
            if missing_location != 0 {
                place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
            }
        }
    }
    h.advance(2);
    assert!(h.inbox.go.is_empty());
    assert!(h.inbox.cooldowns.is_empty());
}

fn facing25_fixture(path: &std::path::Path, item: u32) -> (Harness, Entity) {
    let (mut h, character) = setup(path);
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        });
    let target = spawn_player(&mut h.app, 8, -10.0);
    h.app
        .world_mut()
        .entity_mut(target)
        .insert(shared::components::UnitFactionTemplate(25));
    learn_effect_toy(&mut h, character, item);
    h.select(Some(target));
    h.advance(2);
    (h, target)
}

#[test]
fn toyfx25_facing_actual_use_rejects_rear_selected_unit_before_cast_and_recovery() {
    let directory = tempfile::tempdir().unwrap();
    for item in [141300, 204818] {
        let (mut h, _) = facing25_fixture(&directory.path().join(format!("rear{item}.redb")), item);
        assert_eq!(
            activate(&mut h, item)[0].error.as_deref(),
            Some("NotInFront")
        );
        assert!(
            h.inbox
                .failed
                .iter()
                .any(|failed| failed.reason == shared::spell_data::CastFailReason::NotInFront)
        );
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(h.caster)
                .is_none()
        );
        assert!(h.inbox.go.is_empty());
        assert!(h.inbox.cooldowns.is_empty());
        h.app
            .world_mut()
            .get_mut::<shared::components::Rotation>(h.caster)
            .unwrap()
            .y += std::f32::consts::PI;
        assert_eq!(
            activate(&mut h, item)[0].error,
            None,
            "refusal must not spend recovery"
        );
    }
}

#[test]
fn toyfx25_facing_uses_inclusive_half_arc_and_strict_scaled_exact3d_boundary() {
    let directory = tempfile::tempdir().unwrap();
    // WoW forward +X; Bevy Y is altitude. Target scale, not caster reach,
    // controls the boundary. Exact2yd is outside TC Position::IsInDist.
    let cases = [
        ([-1.999, 0.0, 0.0], 1.0, true),
        ([-2.0, 0.0, 0.0], 1.0, false),
        ([-1.5, 2.0, 0.0], 1.0, false),
        ([-3.88, 0.0, 0.0], 10.0, true),
        ([-3.90, 0.0, 0.0], 10.0, false),
        ([0.0, 0.0, -3.0], 1.0, true),
        ([0.0, 0.0, 3.0], 1.0, true),
        ([-0.01, 0.0, 3.0], 1.0, false),
        ([0.01, 0.0, 3.0], 1.0, true),
    ];
    for (index, (position, scale, permitted)) in cases.into_iter().enumerate() {
        let (mut h, target) = facing25_fixture(
            &directory.path().join(format!("boundary{index}.redb")),
            141300,
        );
        h.app.world_mut().entity_mut(target).insert((
            Position {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            shared::components::UnitScale(scale),
        ));
        // An enormous caster must not contribute reach to the exemption.
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(shared::components::UnitScale(100.0));
        let expected = if permitted { None } else { Some("NotInFront") };
        assert_eq!(
            activate(&mut h, 141300)[0].error.as_deref(),
            expected,
            "case{index}"
        );
    }
}

#[test]
fn toyfx25_facing_uses_source_creature_radius_not_its_combat_reach() {
    use shared::components::{ModelDisplay, Npc, Player, UnitScale};
    let directory = tempfile::tempdir().unwrap();
    // Source display42247: bounding radius6, CombatReach24. UnitScale is the
    // native display/object scale already replicated by the creature spawner.
    for (index, (distance, scale, permitted)) in [
        (5.9, 1.0, true),
        (6.0, 1.0, false),
        (11.9, 2.0, true),
        (12.0, 2.0, false),
    ]
    .into_iter()
    .enumerate()
    {
        let (mut h, target) =
            facing25_fixture(&directory.path().join(format!("model{index}.redb")), 141300);
        h.app
            .world_mut()
            .entity_mut(target)
            .remove::<Player>()
            .insert((
                Npc {
                    template_id: 61463,
                    name: "Slagmaw".into(),
                },
                ModelDisplay { display_id: 42247 },
                Position {
                    x: -distance,
                    y: 0.0,
                    z: 0.0,
                },
                UnitScale(scale),
            ));
        let expected = if permitted { None } else { Some("NotInFront") };
        assert_eq!(
            activate(&mut h, 141300)[0].error.as_deref(),
            expected,
            "case{index}"
        );
    }
}

#[test]
fn toyfx25_facing_rechecks_noninstant_completion_without_spending_recovery() {
    let directory = tempfile::tempdir().unwrap();
    for (item, spell) in [(141300, 225826), (204818, 406870)] {
        let (mut h, _) = facing25_fixture(
            &directory.path().join(format!("completion{item}.redb")),
            item,
        );
        h.app
            .world_mut()
            .get_mut::<shared::components::Rotation>(h.caster)
            .unwrap()
            .y += std::f32::consts::PI;
        assert_eq!(activate(&mut h, item)[0].error, None);
        h.app
            .world_mut()
            .get_mut::<shared::components::Rotation>(h.caster)
            .unwrap()
            .y -= std::f32::consts::PI;
        h.advance(45);
        assert!(h.inbox.failed.iter().any(|failed| failed.spell_id == spell
            && failed.reason == shared::spell_data::CastFailReason::NotInFront));
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == spell));
        assert!(
            h.inbox.cooldowns.iter().all(|cooldown| cooldown.is_gcd),
            "start GCD is legitimate; item/category recovery must remain unspent"
        );
        h.app
            .world_mut()
            .get_mut::<shared::components::Rotation>(h.caster)
            .unwrap()
            .y += std::f32::consts::PI;
        assert_eq!(activate(&mut h, item)[0].error, None);
    }
}

#[test]
fn toyfx26_bauble_actual_learning_source_lifetime_and_retained_recovery() {
    use shared::components::{Health, ModelDisplay};
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("bauble-real-use.redb"));
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    let health = *h.app.world().get::<Health>(h.caster).unwrap();
    let position = *h.app.world().get::<Position>(h.caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(h.caster).copied();
    learn_effect_toy(&mut h, character, 140309);
    assert_eq!(activate(&mut h, 140309)[0].error, None);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 223143));
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 223143)
        .unwrap();
    assert_eq!(aura.duration_ms, 20_000);
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cooldown| cooldown.spell_id == 223143
                && !cooldown.is_gcd
                && cooldown.duration_ms == 120_000)
    );
    h.advance(370);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 223143)
    );
    h.advance(40);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .all(|aura| aura.spell_id != 223143)
    );
    assert_eq!(h.app.world().get::<Health>(h.caster), Some(&health));
    assert_eq!(h.app.world().get::<Position>(h.caster), Some(&position));
    assert_eq!(
        h.app.world().get::<ModelDisplay>(h.caster).copied(),
        display
    );
    assert_eq!(
        activate(&mut h, 140309)[0].error.as_deref(),
        Some("OnCooldown")
    );
}

#[test]
fn toyfx24_tome_actual_learning_five_minute_lifetime_and_retained_recovery() {
    use shared::components::{Health, ModelDisplay};
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("tome-real-use.redb"));
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&source).unwrap().0);
    let health = *h.app.world().get::<Health>(h.caster).unwrap();
    let position = *h.app.world().get::<Position>(h.caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(h.caster).copied();
    learn_effect_toy(&mut h, character, 127670);
    assert_eq!(activate(&mut h, 127670)[0].error, None);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 187146));
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 187146)
        .unwrap();
    assert_eq!(aura.duration_ms, 300_000);
    assert_eq!(aura.caster, h.caster);
    h.advance(5900);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 187146)
    );
    h.advance(110);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .all(|aura| aura.spell_id != 187146)
    );
    assert_eq!(h.app.world().get::<Health>(h.caster), Some(&health));
    assert_eq!(h.app.world().get::<Position>(h.caster), Some(&position));
    assert_eq!(
        h.app.world().get::<ModelDisplay>(h.caster).copied(),
        display
    );
    assert_eq!(
        activate(&mut h, 127670)[0].error.as_deref(),
        Some("OnCooldown")
    );
}

#[test]
fn toyfx23_fountain_actual_learning_source_duration_and_retained_recovery() {
    use shared::components::{Health, ModelDisplay};
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("fountain-real-use.redb"));
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&source).unwrap().0);
    let health = *h.app.world().get::<Health>(h.caster).unwrap();
    let position = *h.app.world().get::<Position>(h.caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(h.caster).copied();
    learn_effect_toy(&mut h, character, 64361);
    assert_eq!(activate(&mut h, 64361)[0].error, None);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 91743));
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 91743)
        .unwrap();
    assert_eq!(aura.duration_ms, 10_000);
    assert_eq!(aura.caster, h.caster);
    h.advance(160);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 91743)
    );
    h.advance(50);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .all(|aura| aura.spell_id != 91743)
    );
    assert_eq!(h.app.world().get::<Health>(h.caster), Some(&health));
    assert_eq!(h.app.world().get::<Position>(h.caster), Some(&position));
    assert_eq!(
        h.app.world().get::<ModelDisplay>(h.caster).copied(),
        display
    );
    assert_eq!(
        activate(&mut h, 64361)[0].error.as_deref(),
        Some("OnCooldown")
    );
    // Another complete-zero aura contract remains unavailable, not admitted by shape.
    learn_effect_toy(&mut h, character, 90067);
    assert!(
        activate(&mut h, 90067)[0]
            .error
            .as_deref()
            .is_some_and(|reason| reason.contains("aura 4"))
    );
}

#[test]
fn toyfx22_pendant_actual_learning_cast_completion_aura_exit_and_cooldown_preservation() {
    use shared::components::{Health, ModelDisplay};
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("pendant-real-use.redb"));
    learn_effect_toy(&mut h, character, 115503);
    assert_eq!(
        snapshot(h.app.world(), 1)
            .toys
            .into_iter()
            .find(|toy| toy.item_id == 115503)
            .unwrap()
            .unavailable_reason,
        None
    );
    let health = h.app.world().get::<Health>(h.caster).unwrap().current;
    let model = h.app.world().get::<ModelDisplay>(h.caster).copied();
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    assert_eq!(
        activate(&mut h, 115503)[0].error.as_deref(),
        Some("Requires area group4031")
    );
    assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == 171352));
    place_toy_caster_on_source_map_area(&mut h, 1116, 843253, [1433.39, 1081.63], 6797);
    assert_eq!(activate(&mut h, 115503)[0].error, None);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::CastState>(h.caster)
            .unwrap()
            .duration,
        5.0
    );
    h.advance(10);
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == 171352));
    // Same-map source Ashran site (TDB371343/86408):7332→6941, outside4031.
    place_toy_caster_on_source_map_area(&mut h, 1116, 843551, [3594.38, -3928.24], 7332);
    h.advance(101);
    assert!(h.inbox.failed.iter().any(|failed| failed.spell_id == 171352
        && failed.reason == shared::spell_data::CastFailReason::IncorrectArea));
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == 171352 && !cd.is_gcd)
    );
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == 171352));
    place_toy_caster_on_source_map_area(&mut h, 1116, 843253, [1433.39, 1081.63], 6797);
    assert_eq!(activate(&mut h, 115503)[0].error, None);
    h.advance(101);
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 171352)
        .unwrap();
    assert_eq!(aura.duration_ms, 120_000);
    assert_eq!(aura.caster, h.caster);
    assert_eq!(
        h.app.world().get::<Health>(h.caster).unwrap().current,
        health
    );
    assert_eq!(h.app.world().get::<ModelDisplay>(h.caster).copied(), model);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 171352));
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == 171352 && !cd.is_gcd)
    );
    place_toy_caster_on_source_map_area(&mut h, 1116, 843551, [3594.38, -3928.24], 7332);
    h.advance(2);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .all(|a| a.spell_id != 171352)
    );
    place_toy_caster_on_source_map_area(&mut h, 1116, 843253, [1433.39, 1081.63], 6797);
    assert_eq!(
        activate(&mut h, 115503)[0].error.as_deref(),
        Some("OnCooldown")
    );
}

#[test]
fn toyfx22_pendant_expires_after_source_120_seconds_without_ending_its_item_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("pendant-natural-expiry.redb"));
    place_toy_caster_on_source_map_area(&mut h, 1116, 843253, [1433.39, 1081.63], 6797);
    learn_effect_toy(&mut h, character, 115503);
    assert_eq!(activate(&mut h, 115503)[0].error, None);
    h.advance(101);
    h.advance(2300);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 171352)
    );
    h.advance(100);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .all(|a| a.spell_id != 171352)
    );
    assert_eq!(
        activate(&mut h, 115503)[0].error.as_deref(),
        Some("OnCooldown")
    );
}

#[test]
fn toyfx21_pendant_parent_area_starts_real_five_second_cast_then_rechecks_completion() {
    let directory = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&directory.path().join("pendant-completion.redb"));
    // TDB1210 creature459999/77488, ADT843253 Area6797 whose parent is6719.
    place_toy_caster_on_source_map_area(&mut h, 1116, 843253, [1433.39, 1081.63], 6797);
    let caster = h.caster;
    crate::spell_cast::cast_granted(h.app.world_mut(), caster, 171352, None).unwrap();
    let cast = h
        .app
        .world()
        .get::<shared::components::CastState>(caster)
        .unwrap();
    assert_eq!(cast.duration, 5.0);
    h.advance(10);
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == 171352));
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    h.advance(101);
    assert!(
        h.inbox
            .failed
            .iter()
            .any(|failure| failure.spell_id == 171352
                && failure.reason == shared::spell_data::CastFailReason::IncorrectArea)
    );
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == 171352));
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|cooldown| cooldown.spell_id == 171352 && !cooldown.is_gcd)
    );
}

#[test]
fn toyfx20_war_banners_learn_apply_source_aura_and_remove_on_real_combat_entry() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    let directory = tempfile::tempdir().unwrap();
    for (item, spell, race) in [(169275, 300104, 1), (169276, 300144, 2)] {
        let (mut h, character) =
            setup_for_race(&directory.path().join(format!("war-{item}.redb")), race);
        learn_effect_toy(&mut h, character, item);
        let before = *h.app.world().get::<Health>(h.caster).unwrap();
        assert_eq!(activate(&mut h, item)[0].error, None, "item {item}");
        h.advance(40);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == spell)
            .unwrap();
        assert_eq!(aura.expires_at, None);
        assert_eq!(aura.interrupt_flags, [0x1000_1002, 0]);
        assert_eq!(*h.app.world().get::<Health>(h.caster).unwrap(), before);
        assert!(h.app.world().get::<ModelDisplay>(h.caster).is_none());
        let present = |h: &Harness| {
            h.app
                .world()
                .get::<UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|aura| aura.spell_id == spell)
        };
        assert!(present(&h));
        let (caster, npc) = (h.caster, h.npc);
        crate::combat_refs::set_in_combat_with(h.app.world_mut(), caster, npc);
        h.advance(2);
        assert!(h.app.world().get::<CombatStatus>(caster).unwrap().0);
        assert!(!present(&h), "entering actual combat removes spell {spell}");
        assert!(
            h.app
                .world()
                .get::<crate::auras::ActiveAuras>(caster)
                .unwrap()
                .iter()
                .all(|aura| aura.spell_id != spell)
        );
        assert_eq!(*h.app.world().get::<Health>(caster).unwrap(), before);
        assert!(h.app.world().get::<ModelDisplay>(caster).is_none());
    }
}

#[test]
fn toyfx19_gladiator_banners_respect_faction_and_cancel_persistent_cosmetic_aura() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    let directory = tempfile::tempdir().unwrap();
    for (item, spell, race, wrong_race) in [(119220, 176899, 1, 2), (119221, 176900, 2, 1)] {
        let (mut h, character) = setup_for_race(
            &directory.path().join(format!("gladiator-{item}.redb")),
            race,
        );
        learn_effect_toy(&mut h, character, item);
        let before = *h.app.world().get::<Health>(h.caster).unwrap();
        assert_eq!(activate(&mut h, item)[0].error, None, "item {item}");
        h.advance(40);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == spell)
            .unwrap();
        assert_eq!(aura.expires_at, None);
        assert_eq!(*h.app.world().get::<Health>(h.caster).unwrap(), before);
        assert!(h.app.world().get::<ModelDisplay>(h.caster).is_none());
        assert!(
            h.app
                .world()
                .get::<UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|aura| aura.spell_id == spell)
        );
        h.app
            .world_mut()
            .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
            .unwrap()
            .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
                spell_id: spell,
            });
        h.advance(5);
        assert!(
            !h.app
                .world()
                .get::<UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|aura| aura.spell_id == spell)
        );

        h.app
            .world_mut()
            .get_mut::<shared::components::Player>(h.caster)
            .unwrap()
            .race = wrong_race;
        h.inbox.go.clear();
        assert_eq!(
            activate(&mut h, item)[0].error.as_deref(),
            Some("toy race/class restriction")
        );
        h.advance(40);
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == spell));
    }
}

#[test]
fn toyfx18_placed_statues_census() {
    let path = toyfx8_world();
    let report = report_toy_census(&path, "TOYFX18_STATUE_CENSUS_REPORT");
    let toys = report["toys"].as_array().unwrap();
    let find = |item: u32| toys.iter().find(|toy| toy["item_id"] == item).unwrap();
    for item in [198646, 204405] {
        assert!(find(item)["unavailable"].is_null(), "{}", find(item));
    }
    // Seats, target dummies and wormholes keep their interaction gap; Attraction
    // Sign's only weighted display is an invisible trigger.
    for item in [116689, 88375, 172924, 122126] {
        assert!(!find(item)["unavailable"].is_null(), "{}", find(item));
    }
}

#[derive(Resource, Default)]
struct ToyChatCapture(std::collections::HashMap<Entity, Vec<ChatMessage>>);

fn capture_toy_chat(
    mut receivers: Query<(Entity, &mut MessageReceiver<ChatMessage>)>,
    mut captured: ResMut<ToyChatCapture>,
) {
    for (connection, mut receiver) in &mut receivers {
        captured
            .0
            .entry(connection)
            .or_default()
            .extend(receiver.receive());
    }
}

#[test]
fn toyfx14_dice_learn_emote_and_roll_to_only_the_authored_audience() {
    use crate::group::{GroupRegistry, OnlineMember, OnlineView};
    use crate::persistence::ClientId;
    use shared::components::{Health, Player, Position};
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = crate::spell_info::load_spell_data(&source).unwrap().0;
    for (item, spell, text_id, maximum, count, sex, in_party) in [
        (36863, 47770, 26147, 100, 1, 0, false),
        (36863, 47770, 26147, 100, 1, 1, true),
        (36862, 47776, 26152, 6, 2, 0, true),
        (36862, 47776, 26152, 6, 2, 1, false),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&directory.path().join("dice.redb"));
        h.app.insert_resource(store.clone());
        h.app.init_resource::<GroupRegistry>();
        h.app.init_resource::<ToyChatCapture>();
        h.app.add_systems(PostUpdate, capture_toy_chat);
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(ClientId(character));
        h.app
            .world_mut()
            .get_mut::<Player>(h.caster)
            .unwrap()
            .appearance
            .sex = sex;
        h.app.world_mut().entity_mut(h.connection).insert((
            MessageSender::<ChatMessage>::default(),
            MessageReceiver::<ChatMessage>::default(),
        ));
        let mut peers = Vec::new();
        for (peer, x, map, name) in [
            (2, 35.0, 0, "Near"),
            (3, 500.0, 0, "Far"),
            (4, 3.0, 1, "OtherMap"),
            (5, 500.0, 1, "PartyFar"),
        ] {
            let player = spawn_player(&mut h.app, 8, x);
            h.app.world_mut().get_mut::<Player>(player).unwrap().name = name.into();
            h.app
                .world_mut()
                .entity_mut(player)
                .insert((ClientId(100 + peer), crate::maps::InMap::continent(map)));
            let connection = spawn_peer_connection(&mut h.app, player, peer);
            h.app.world_mut().entity_mut(connection).insert((
                MessageSender::<ChatMessage>::default(),
                MessageReceiver::<ChatMessage>::default(),
            ));
            peers.push((player, connection));
        }
        if in_party {
            let online = OnlineView::from([
                (
                    character,
                    OnlineMember {
                        entity: h.caster.to_bits(),
                        name: "Caster".into(),
                        class: 8,
                        level: 80,
                        portrait: default(),
                        dungeon_difficulty: 1,
                        raid_difficulty: default(),
                    },
                ),
                (
                    105,
                    OnlineMember {
                        entity: peers[3].0.to_bits(),
                        name: "PartyFar".into(),
                        class: 8,
                        level: 80,
                        portrait: default(),
                        dungeon_difficulty: 1,
                        raid_difficulty: default(),
                    },
                ),
            ]);
            let mut groups = h.app.world_mut().resource_mut::<GroupRegistry>();
            groups.invite(character, "PartyFar", &online);
            groups.respond_invite(105, true);
            assert_eq!(groups.group_members(character).len(), 2);
        }
        let health = h.app.world().get::<Health>(h.caster).unwrap().current;
        let origin = *h.app.world().get::<Position>(h.caster).unwrap();
        learn_effect_toy(&mut h, character, item);
        assert_eq!(activate(&mut h, item)[0].error, None, "item {item}");
        h.advance(20);
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
        assert_eq!(
            h.app.world().get::<Health>(h.caster).unwrap().current,
            health
        );
        assert_eq!(h.app.world().get::<Position>(h.caster), Some(&origin));
        let text: (String, String) = source
            .query_row(
                "SELECT MaleText, FemaleText FROM content_broadcast_text WHERE ID=?",
                [text_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let expected_emote = if sex == 1 { text.1 } else { text.0 };
        let captured = &h.app.world().resource::<ToyChatCapture>().0;
        let caster_chat = captured.get(&h.connection).unwrap();
        let emotes: Vec<_> = caster_chat
            .iter()
            .filter(|message| message.channel == ChatType::Emote)
            .collect();
        assert_eq!(emotes.len(), 1);
        assert_eq!(emotes[0].sender, "Caster");
        assert_eq!(emotes[0].content, expected_emote);
        let rolls: Vec<_> = caster_chat
            .iter()
            .filter(|message| message.channel == ChatType::System)
            .collect();
        assert_eq!(rolls.len(), count);
        for roll in &rolls {
            let value = roll
                .content
                .strip_prefix("Caster rolls ")
                .unwrap()
                .strip_suffix(&format!(" (1-{maximum})"))
                .unwrap()
                .parse::<u32>()
                .unwrap();
            assert!((1..=maximum).contains(&value));
        }
        let near = captured.get(&peers[0].1).unwrap();
        assert_eq!(near.len(), 1);
        assert_eq!(near[0].sender, emotes[0].sender);
        assert_eq!(near[0].content, emotes[0].content);
        assert_eq!(near[0].channel, emotes[0].channel);
        assert!(captured.get(&peers[1].1).is_none_or(Vec::is_empty));
        assert!(captured.get(&peers[2].1).is_none_or(Vec::is_empty));
        let party_chat = captured.get(&peers[3].1);
        if in_party {
            let party_chat = party_chat.unwrap();
            assert_eq!(party_chat.len(), count);
            for (received, expected) in party_chat.iter().zip(&rolls) {
                assert_eq!(received.sender, expected.sender);
                assert_eq!(received.content, expected.content);
                assert_eq!(received.channel, expected.channel);
            }
        } else {
            assert!(party_chat.is_none_or(Vec::is_empty));
        }
        let before: usize = captured.values().map(Vec::len).sum();
        assert!(
            activate(&mut h, item)[0].error.is_some(),
            "source10s cooldown"
        );
        h.advance(5);
        let after: usize = h
            .app
            .world()
            .resource::<ToyChatCapture>()
            .0
            .values()
            .map(Vec::len)
            .sum();
        assert_eq!(after, before, "rejected repeat must not emote or roll");
    }
}

#[test]
fn toyfx14_dice_refuses_missing_source_or_caster_context_and_payload_drift() {
    use crate::persistence::ClientId;
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for missing in ["text", "group", "identity", "amount", "extra effect"] {
        let directory = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&directory.path().join("dice-refusal.redb"));
        let mut altered = store.get(47770).unwrap().clone();
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(ClientId(character));
        match missing {
            "text" => {
                h.app
                    .world_mut()
                    .remove_resource::<crate::spell_cast::toy_scripts::ToyTexts>();
            }
            "group" => {
                h.app
                    .world_mut()
                    .remove_resource::<crate::group::GroupRegistry>();
            }
            "identity" => {
                h.app.world_mut().entity_mut(h.caster).remove::<ClientId>();
            }
            "amount" => {
                altered.effects[0].base_points = 1.0;
            }
            "extra effect" => {
                altered.effects.push(crate::spell_info::SpellEffect {
                    index: 1,
                    effect: 10,
                    implicit_targets: [1, 0],
                    base_points: 100.0,
                    ..default()
                });
            }
            _ => unreachable!(),
        }
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(vec![altered]));
        h.app.world_mut().entity_mut(h.connection).insert((
            MessageSender::<ChatMessage>::default(),
            MessageReceiver::<ChatMessage>::default(),
        ));
        h.app.init_resource::<ToyChatCapture>();
        h.app.add_systems(PostUpdate, capture_toy_chat);
        learn_effect_toy(&mut h, character, 36863);
        assert!(activate(&mut h, 36863)[0].error.is_some(), "{missing}");
        h.advance(5);
        assert!(h.inbox.go.is_empty(), "{missing} must not deliver SpellGo");
        assert!(
            h.inbox.cooldowns.is_empty(),
            "{missing} must not start cooldown"
        );
        assert!(
            h.app
                .world()
                .resource::<ToyChatCapture>()
                .0
                .values()
                .all(Vec::is_empty),
            "{missing} must not emit an emote or roll"
        );
    }
}

fn coin_texts(source: &rusqlite::Connection, sex: u8) -> [String; 3] {
    [32638, 32663, 32664].map(|id| {
        source
            .query_row(
                "SELECT MaleText, FemaleText FROM content_broadcast_text WHERE ID=?",
                [id],
                |row| row.get::<_, String>(if sex == 1 { 1 } else { 0 }),
            )
            .unwrap()
    })
}

fn emotes_for(h: &Harness, connection: Entity) -> Vec<String> {
    h.app
        .world()
        .resource::<ToyChatCapture>()
        .0
        .get(&connection)
        .map_or(Vec::new(), |chat| {
            chat.iter()
                .filter(|message| message.channel == ChatType::Emote)
                .map(|message| message.content.clone())
                .collect()
        })
}

#[test]
fn toyfx18_coin_tosses_then_catches_heads_or_tails_after_source_flight() {
    use crate::persistence::ClientId;
    use shared::components::{Health, Player, Position};
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = crate::spell_info::load_spell_data(&source).unwrap().0;
    for sex in [0, 1] {
        let directory = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&directory.path().join("coin.redb"));
        h.app.insert_resource(store.clone());
        h.app.init_resource::<ToyChatCapture>();
        h.app.add_systems(PostUpdate, capture_toy_chat);
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(ClientId(character));
        h.app
            .world_mut()
            .get_mut::<Player>(h.caster)
            .unwrap()
            .appearance
            .sex = sex;
        h.app.world_mut().entity_mut(h.connection).insert((
            MessageSender::<ChatMessage>::default(),
            MessageReceiver::<ChatMessage>::default(),
        ));
        let mut peers = Vec::new();
        for (peer, x, name) in [(2, 35.0, "Near"), (3, 500.0, "Far")] {
            let player = spawn_player(&mut h.app, 8, x);
            h.app.world_mut().get_mut::<Player>(player).unwrap().name = name.into();
            h.app
                .world_mut()
                .entity_mut(player)
                .insert((ClientId(100 + peer), crate::maps::InMap::continent(0)));
            let connection = spawn_peer_connection(&mut h.app, player, peer);
            h.app.world_mut().entity_mut(connection).insert((
                MessageSender::<ChatMessage>::default(),
                MessageReceiver::<ChatMessage>::default(),
            ));
            peers.push(connection);
        }
        let [toss, heads, tails] = coin_texts(&source, sex);
        let health = *h.app.world().get::<Health>(h.caster).unwrap();
        let origin = *h.app.world().get::<Position>(h.caster).unwrap();
        learn_effect_toy(&mut h, character, 44430);
        assert_eq!(activate(&mut h, 44430)[0].error, None);
        assert!(h.inbox.go.iter().any(|go| go.spell_id == 60458));
        assert_eq!(emotes_for(&h, h.connection), vec![toss.clone()]);
        assert_eq!(emotes_for(&h, peers[0]), vec![toss.clone()]);
        // Source missile: 20yd above the caster at speed10 lands after 2000ms.
        h.advance(30);
        assert_eq!(
            emotes_for(&h, h.connection).len(),
            1,
            "caught before source flight"
        );
        h.advance(14);
        let caster_emotes = emotes_for(&h, h.connection);
        assert_eq!(caster_emotes.len(), 2);
        assert!(
            caster_emotes[1] == heads || caster_emotes[1] == tails,
            "{caster_emotes:?}"
        );
        assert_eq!(emotes_for(&h, peers[0]), caster_emotes);
        assert!(emotes_for(&h, peers[1]).is_empty());
        assert_eq!(h.app.world().get::<Health>(h.caster), Some(&health));
        assert_eq!(h.app.world().get::<Position>(h.caster), Some(&origin));
        h.advance(20);
        assert_eq!(emotes_for(&h, h.connection).len(), 2, "one catch per toss");
    }
}

#[test]
fn toyfx18_coin_refuses_missing_text_or_source_shape_drift() {
    use crate::persistence::ClientId;
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for missing in ["text", "trigger", "catch target", "extra effect"] {
        let directory = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&directory.path().join("coin-refusal.redb"));
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(ClientId(character));
        let mut toss = store.get(60458).unwrap().clone();
        let mut catch = store.get(60476).unwrap().clone();
        match missing {
            "text" => {
                h.app
                    .world_mut()
                    .remove_resource::<crate::spell_cast::toy_scripts::ToyTexts>();
            }
            "trigger" => toss.effects[0].trigger_spell = 116,
            "catch target" => catch.effects[0].implicit_targets = [6, 0],
            "extra effect" => toss.effects.push(crate::spell_info::SpellEffect {
                index: 1,
                effect: 10,
                implicit_targets: [1, 0],
                base_points: 100.0,
                ..default()
            }),
            _ => unreachable!(),
        }
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(vec![toss, catch]));
        h.app.world_mut().entity_mut(h.connection).insert((
            MessageSender::<ChatMessage>::default(),
            MessageReceiver::<ChatMessage>::default(),
        ));
        h.app.init_resource::<ToyChatCapture>();
        h.app.add_systems(PostUpdate, capture_toy_chat);
        learn_effect_toy(&mut h, character, 44430);
        assert!(activate(&mut h, 44430)[0].error.is_some(), "{missing}");
        h.advance(50);
        assert!(h.inbox.go.is_empty(), "{missing}");
        assert!(h.inbox.cooldowns.is_empty(), "{missing}");
        assert!(emotes_for(&h, h.connection).is_empty(), "{missing}");
    }
}

const PERSONALIZED_WEATHER: [u32; 4] = [46740, 46739, 46738, 46736];

fn weather_auras(h: &Harness) -> Vec<(u32, u32, Entity)> {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .map_or(Vec::new(), |auras| {
            auras
                .iter()
                .filter(|aura| PERSONALIZED_WEATHER.contains(&aura.spell_id))
                .map(|aura| (aura.spell_id, aura.duration_ms, aura.caster))
                .collect()
        })
}

#[test]
fn toyfx18_weather_machine_applies_one_authored_personal_weather() {
    use crate::persistence::ClientId;
    use shared::components::{Health, Position};
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = crate::spell_info::load_spell_data(&source).unwrap().0;
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("weather.redb"));
    h.app.insert_resource(store);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ClientId(character));
    let health = *h.app.world().get::<Health>(h.caster).unwrap();
    let origin = *h.app.world().get::<Position>(h.caster).unwrap();
    learn_effect_toy(&mut h, character, 35227);
    assert_eq!(activate(&mut h, 35227)[0].error, None);
    h.advance(5);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 46203));
    let applied = weather_auras(&h);
    assert_eq!(applied.len(), 1, "{applied:?}");
    let (weather, duration, caster) = applied[0];
    // SpellDuration9: 30000ms; the caster weathers itself (target->CastSpell(target)).
    assert_eq!((duration, caster), (30000, h.caster));
    assert!(PERSONALIZED_WEATHER.contains(&weather));
    // 46736's 5000ms periodic dummy has no TDB script: its ticks change nothing.
    h.advance(140);
    assert_eq!(weather_auras(&h).len(), 1);
    assert_eq!(h.app.world().get::<Health>(h.caster), Some(&health));
    assert_eq!(h.app.world().get::<Position>(h.caster), Some(&origin));
    h.advance(470);
    assert!(weather_auras(&h).is_empty(), "authored 30s weather expires");
}

#[test]
fn toyfx18_weather_machine_refuses_unaudited_weather_shape() {
    use crate::persistence::ClientId;
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for drift in [
        "missing weather",
        "weather trigger",
        "no visual",
        "extra effect",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&directory.path().join("weather-refusal.redb"));
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(ClientId(character));
        let mut machine = store.get(46203).unwrap().clone();
        let mut weathers: Vec<_> = PERSONALIZED_WEATHER
            .iter()
            .map(|id| store.get(*id).unwrap().clone())
            .collect();
        match drift {
            "missing weather" => {
                weathers.pop();
            }
            "weather trigger" => weathers[3].effects[0].trigger_spell = 116,
            "no visual" => weathers[0].has_spell_visual = false,
            "extra effect" => machine.effects.push(crate::spell_info::SpellEffect {
                index: 1,
                effect: 10,
                implicit_targets: [1, 0],
                base_points: 100.0,
                ..default()
            }),
            _ => unreachable!(),
        }
        weathers.push(machine);
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(weathers));
        learn_effect_toy(&mut h, character, 35227);
        assert!(activate(&mut h, 35227)[0].error.is_some(), "{drift}");
        h.advance(5);
        assert!(h.inbox.go.is_empty(), "{drift}");
        assert!(h.inbox.cooldowns.is_empty(), "{drift}");
        assert!(weather_auras(&h).is_empty(), "{drift}");
    }
}

/// Cast-event caster kits with no aura, script binding or gameplay payload.
const CAST_KIT_TOYS: [(u32, u32); 3] = [
    (122119, 103740), // Everlasting Darkmoon Firework
    (166877, 290627), // Azerite Firework Launcher
    (127668, 187150), // Jewel of Hellfire
];

#[test]
fn toyfx18_cast_kit_visual_toys_cast_without_gameplay() {
    use shared::components::{Health, ModelDisplay, Position, UnitAuras};
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (item, spell) in CAST_KIT_TOYS {
        let directory = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&directory.path().join("cast-kit.redb"));
        h.app.insert_resource(store.clone());
        // Azerite's TARGET_DEST_CASTER_GROUND needs authored ground under the caster.
        h.app
            .insert_resource(crate::terrain_height::test_helpers::flat_ground(
                33.0, 33.0, 2.0,
            ));
        h.app.world_mut().entity_mut(h.caster).insert(Position {
            x: 10.0,
            y: 2.0,
            z: -10.0,
        });
        let model = h.app.world().get::<ModelDisplay>(h.caster).cloned();
        let health = *h.app.world().get::<Health>(h.caster).unwrap();
        let position = *h.app.world().get::<Position>(h.caster).unwrap();
        learn_effect_toy(&mut h, character, item);
        assert_eq!(activate(&mut h, item)[0].error, None, "item {item}");
        h.advance(5);
        assert!(
            h.inbox.failed.is_empty(),
            "item {item}: {:?}",
            h.inbox.failed
        );
        assert!(
            h.inbox.go.iter().any(|go| go.spell_id == spell),
            "item {item}"
        );
        assert_eq!(h.app.world().get::<ModelDisplay>(h.caster), model.as_ref());
        assert_eq!(*h.app.world().get::<Health>(h.caster).unwrap(), health);
        assert_eq!(*h.app.world().get::<Position>(h.caster).unwrap(), position);
        assert!(
            h.app
                .world()
                .get::<UnitAuras>(h.caster)
                .is_none_or(|auras| auras.auras.iter().all(|aura| aura.spell_id != spell)),
            "item {item} has no aura"
        );
    }
}

#[test]
fn toyfx18_cast_kit_visual_toys_refuse_lost_visual_or_payload() {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for drift in ["no visual", "payload"] {
        let directory = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&directory.path().join("cast-kit-drift.redb"));
        let mut spell = store.get(103740).unwrap().clone();
        match drift {
            "no visual" => spell.has_spell_visual = false,
            "payload" => spell.effects[0].base_points = 50.0,
            _ => unreachable!(),
        }
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(vec![spell]));
        learn_effect_toy(&mut h, character, 122119);
        assert!(activate(&mut h, 122119)[0].error.is_some(), "{drift}");
        h.advance(5);
        assert!(h.inbox.go.is_empty(), "{drift}");
    }
}

/// TC spell_gen_upper_deck_create_foam_sword item choices.
const FOAM_SWORDS: [u32; 5] = [45061, 45176, 45177, 45178, 45179];

fn foam_swords(h: &Harness, character: u64) -> u32 {
    let db = &h.app.world().resource::<GameDatabase>().db;
    persistence::load_inventory(db, character)
        .items
        .iter()
        .filter(|item| FOAM_SWORDS.contains(&item.item_id))
        .map(|item| item.stack_count)
        .sum()
}

fn place_foam_sword_rack(h: &mut Harness, character: u64) -> Entity {
    use crate::persistence::ClientId;
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::merchant::load_vendor_catalog(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ClientId(character));
    learn_effect_toy(h, character, 45063);
    assert_eq!(activate_at(h, 45063, Some([2.0, 0.0, 0.0]))[0].error, None);
    h.advance(70); // Source CastingTimeIndex14: 3000ms; harness frames are 50ms.
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 62985));
    let world = h.app.world_mut();
    let racks: Vec<_> = world
        .query::<(Entity, &shared::protocol::GameObjectInfo)>()
        .iter(world)
        .filter(|(_, info)| info.entry == 194342)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(racks.len(), 1);
    racks[0]
}

#[test]
fn toyfx18_foam_sword_rack_grants_one_source_sword_until_it_expires() {
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("foam.redb"));
    let rack = place_foam_sword_rack(&mut h, character);
    assert_eq!(foam_swords(&h, character), 0);
    crate::game_objects::spellcaster::use_object(h.app.world_mut(), h.caster, rack).unwrap();
    h.advance(3);
    assert_eq!(foam_swords(&h, character), 1);
    // "player can only have one of these items": a second use creates nothing.
    crate::game_objects::spellcaster::use_object(h.app.world_mut(), h.caster, rack).unwrap();
    h.advance(3);
    assert_eq!(foam_swords(&h, character), 1);
    // SpellDuration 180000ms: the wild rack despawns on its own (virtual clock).
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(std::time::Duration::from_secs(7200));
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs(180),
        ));
    h.advance(2);
    assert!(h.app.world().get_entity(rack).is_err());
}

#[test]
fn toyfx18_summoned_objects_replicate_ready_collision_state() {
    // TC EffectTransmitted/EffectSummonObjectWild create GO_STATE_READY objects; the
    // client draws no game object without its replicated collision state.
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("collision.redb"));
    let rack = place_foam_sword_rack(&mut h, character);
    assert_eq!(
        h.app
            .world()
            .get::<shared::protocol::GameObjectCollision>(rack),
        Some(&shared::protocol::GameObjectCollision(true))
    );
    learn_effect_toy(&mut h, character, 45011);
    assert_eq!(
        activate_at(&mut h, 45011, Some([4.0, 0.0, 0.0]))[0].error,
        None
    );
    h.advance(70);
    let world = h.app.world_mut();
    let banners: Vec<_> = world
        .query::<(
            &shared::protocol::GameObjectInfo,
            Option<&shared::protocol::GameObjectCollision>,
        )>()
        .iter(world)
        .filter(|(info, _)| info.entry == 194274)
        .map(|(_, collision)| collision.copied())
        .collect();
    assert_eq!(
        banners,
        vec![Some(shared::protocol::GameObjectCollision(true))]
    );
}

#[test]
fn toyfx18_foam_sword_rack_counts_a_banked_sword() {
    use crate::persistence::banks::{BankData, BankOwner, save_bank_move};
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("foam-bank.redb"));
    let rack = place_foam_sword_rack(&mut h, character);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let mut tab = crate::bank::new_tab(1);
    tab.slots[0] = Some(InventoryItemData {
        item_guid: persistence::next_id(db, "item_instance"),
        item_id: 45178,
        name: "Foam Sword".into(),
        quality: 0,
        required_level: 1,
        stack_count: 1,
        vendor_sell_price: 0,
        unique_auction: false,
    });
    let bank = BankData {
        tabs: vec![tab],
        money: 0,
    };
    save_bank_move(db, character, BankOwner::Character(character, &bank), None);
    crate::game_objects::spellcaster::use_object(h.app.world_mut(), h.caster, rack).unwrap();
    h.advance(3);
    assert_eq!(foam_swords(&h, character), 0);
}

#[test]
fn toyfx18_transmitted_spellcaster_with_unscripted_object_spell_is_unavailable() {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&directory.path().join("costume-chest.redb"));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    // Costume Chest 252775 casts 222360, a caster dummy no TC/TDB script implements.
    let reasons = toy_catalog::unsupported_types(h.app.world(), 222367);
    assert!(reasons.contains("effect 3"), "{reasons:?}");
    // Audited object spells keep their banners: 424023 and 1215373 are cosmetic kits.
    for root in [424022, 1215364] {
        let reasons = toy_catalog::unsupported_types(h.app.world(), root);
        assert!(reasons.is_empty(), "{root}: {reasons:?}");
    }
}

#[test]
fn toyfx18_celebration_banner_use_applies_its_cosmetic_aura() {
    use crate::persistence::ClientId;
    use shared::components::{Health, ModelDisplay, UnitAuras};
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("banner.redb"));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ClientId(character));
    learn_effect_toy(&mut h, character, 210042);
    assert_eq!(
        activate_at(&mut h, 210042, Some([2.0, 0.0, 0.0]))[0].error,
        None
    );
    h.advance(70);
    let banner = {
        let world = h.app.world_mut();
        world
            .query::<(Entity, &shared::protocol::GameObjectInfo)>()
            .iter(world)
            .find(|(_, info)| info.entry == 409869)
            .unwrap()
            .0
    };
    let model = h.app.world().get::<ModelDisplay>(h.caster).cloned();
    let health = *h.app.world().get::<Health>(h.caster).unwrap();
    crate::game_objects::spellcaster::use_object(h.app.world_mut(), h.caster, banner).unwrap();
    h.advance(5);
    let aura = h
        .app
        .world()
        .get::<UnitAuras>(h.caster)
        .unwrap()
        .auras
        .iter()
        .find(|aura| aura.spell_id == 424023)
        .cloned()
        .unwrap();
    assert_eq!(aura.duration_ms, 3_600_000);
    assert_eq!(h.app.world().get::<ModelDisplay>(h.caster), model.as_ref());
    assert_eq!(*h.app.world().get::<Health>(h.caster).unwrap(), health);
}

#[test]
fn toyfx18_wild_display_with_unscripted_object_spell_stays_unavailable() {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&directory.path().join("blaster.redb"));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::merchant::load_vendor_catalog(&conn).unwrap());
    // 441724 casts dummy 445930, which TDB1210 binds to no script.
    let reasons = toy_catalog::unsupported_types(h.app.world(), 445938);
    assert!(reasons.contains("effect 3"), "{reasons:?}");
    let rack = toy_catalog::unsupported_types(h.app.world(), 62985);
    assert!(rack.is_empty(), "{rack:?}");
}

/// TC spell_item_wormhole_pandaria WormholeTargetLocations.
const WORMHOLE_PANDARIA_DESTINATIONS: [u32; 9] = [
    126756, 126757, 126758, 126759, 126760, 126761, 126762, 126763, 126764,
];

fn prepare_world_port(h: &mut Harness, conn: &rusqlite::Connection, character: u64) {
    use lightyear::prelude::*;
    use shared::protocol::{NewWorld, WorldPortAck};
    h.app
        .insert_resource(crate::spell_cast::teleport_positions::load_positions(conn).unwrap());
    h.app
        .insert_resource(crate::maps::store::load_map_store(conn).unwrap());
    h.app.init_resource::<crate::maps::InstanceManager>();
    h.app
        .add_systems(Update, crate::maps::transfer::receive_world_port_acks);
    h.app.world_mut().entity_mut(h.caster).insert((
        crate::persistence::ClientId(character),
        shared::components::CombatStatus(false),
        shared::components::MovementControl::default(),
        shared::components::Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    ));
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<NewWorld>::default(),
        MessageReceiver::<NewWorld>::default(),
        MessageSender::<WorldPortAck>::default(),
        MessageReceiver::<WorldPortAck>::default(),
    ));
}

#[test]
fn toyfx18_wormhole_pandaria_transfers_to_one_authored_pandaria_point() {
    use lightyear::prelude::*;
    use shared::components::Position;
    use shared::protocol::{NewWorld, TransferChannel, WorldPortAck};
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("wormhole.redb"));
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    prepare_world_port(&mut h, &conn, character);
    let positions = crate::spell_cast::teleport_positions::load_positions(&conn).unwrap();
    let destinations: Vec<_> = WORMHOLE_PANDARIA_DESTINATIONS
        .iter()
        .map(|spell| positions.0[&(*spell, 0)])
        .collect();
    assert!(destinations.iter().all(|point| point.map_id == 870));
    learn_effect_toy(&mut h, character, 87215);
    assert_eq!(activate(&mut h, 87215)[0].error, None);
    let mut worlds = Vec::new();
    for _ in 0..250 {
        h.advance(1);
        worlds.extend(drain::<NewWorld>(&mut h.app, h.connection));
    }
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 126755));
    // Triggered teleports send no SpellGo; the transfer names the chosen point.
    let destination = h
        .app
        .world()
        .get::<crate::maps::MapTransfer>(h.caster)
        .unwrap()
        .destination;
    assert!(destinations.contains(&destination), "{destination:?}");
    assert_eq!(worlds.len(), 1);
    assert_eq!(worlds[0].map_id, 870);
    h.app
        .world_mut()
        .get_mut::<MessageSender<WorldPortAck>>(h.connection)
        .unwrap()
        .send::<TransferChannel>(WorldPortAck);
    h.advance(4);
    assert_eq!(
        *h.app.world().get::<crate::maps::InMap>(h.caster).unwrap(),
        crate::maps::InMap::continent(870)
    );
    assert_eq!(
        *h.app.world().get::<Position>(h.caster).unwrap(),
        destination.bevy_position()
    );
}

#[test]
fn toyfx18_wormhole_refuses_missing_destination_or_source_drift() {
    use shared::components::Position;
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for drift in [
        "missing point",
        "missing teleport",
        "teleport target",
        "extra effect",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&directory.path().join("wormhole-refusal.redb"));
        prepare_world_port(&mut h, &conn, character);
        let mut spells: Vec<_> = std::iter::once(126755)
            .chain(WORMHOLE_PANDARIA_DESTINATIONS)
            .map(|id| store.get(id).unwrap().clone())
            .collect();
        match drift {
            "missing point" => {
                h.app
                    .world_mut()
                    .resource_mut::<crate::spell_cast::teleport_positions::SpellTargetPositions>()
                    .0
                    .remove(&(126760, 0));
            }
            "missing teleport" => {
                spells.pop();
            }
            "teleport target" => spells[3].effects[0].implicit_targets = [25, 0],
            "extra effect" => spells[0].effects.push(crate::spell_info::SpellEffect {
                index: 1,
                effect: 10,
                implicit_targets: [1, 0],
                base_points: 100.0,
                ..default()
            }),
            _ => unreachable!(),
        }
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(spells));
        let position = *h.app.world().get::<Position>(h.caster).unwrap();
        learn_effect_toy(&mut h, character, 87215);
        assert!(activate(&mut h, 87215)[0].error.is_some(), "{drift}");
        h.advance(5);
        assert!(h.inbox.go.is_empty(), "{drift}");
        assert!(h.inbox.cooldowns.is_empty(), "{drift}");
        assert_eq!(h.app.world().get::<Position>(h.caster), Some(&position));
    }
}

#[test]
fn toyfx18_compass_learn_and_use_changes_only_source_facing() {
    use shared::components::{Health, Position, Rotation};
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("compass.redb"));
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app.world_mut().entity_mut(h.caster).insert(Rotation {
        x: 0.0,
        y: -10.0,
        z: 0.0,
    });
    let position = *h.app.world().get::<Position>(h.caster).unwrap();
    let health = *h.app.world().get::<Health>(h.caster).unwrap();
    let other_rotation = h.app.world().get::<Rotation>(h.npc).copied();
    learn_effect_toy(&mut h, character, 45984);
    assert_eq!(activate(&mut h, 45984)[0].error, None);
    h.advance(50); // Source cast time is 2000ms; harness frames are 50ms.
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 64385));
    let rotation = h.app.world().get::<Rotation>(h.caster).unwrap();
    let source_orientation = rotation.y - std::f32::consts::FRAC_PI_2;
    assert!((0.0..=std::f32::consts::TAU).contains(&source_orientation));
    let correction = h
        .app
        .world()
        .get::<shared::components::ForcedFacing>(h.caster)
        .unwrap();
    assert_eq!(correction.yaw, rotation.y);
    assert_eq!(
        correction.epoch,
        h.app
            .world()
            .get::<shared::components::MovementControl>(h.caster)
            .unwrap()
            .epoch
    );
    assert_eq!((rotation.x, rotation.z), (0.0, 0.0));
    assert_eq!(h.app.world().get::<Position>(h.caster), Some(&position));
    assert_eq!(h.app.world().get::<Health>(h.caster), Some(&health));
    assert_eq!(
        h.app.world().get::<Rotation>(h.npc).copied(),
        other_rotation
    );
}

#[test]
fn toyfx18_compass_refuses_missing_rotation_or_changed_source_payload() {
    use shared::components::{Position, Rotation};
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for missing in [
        "rotation",
        "control",
        "amount",
        "trigger",
        "target",
        "extra effect",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&directory.path().join("compass-refusal.redb"));
        h.app.world_mut().entity_mut(h.caster).insert(Rotation {
            x: 0.0,
            y: 0.7,
            z: 0.0,
        });
        let mut spell = store.get(64385).unwrap().clone();
        match missing {
            "rotation" => {
                h.app.world_mut().entity_mut(h.caster).remove::<Rotation>();
            }
            "control" => {
                h.app
                    .world_mut()
                    .entity_mut(h.caster)
                    .remove::<shared::components::MovementControl>();
            }
            "amount" => spell.effects[0].base_points = 1.0,
            "trigger" => spell.effects[0].trigger_spell = 116,
            "target" => spell.effects[0].implicit_targets = [6, 0],
            "extra effect" => spell.effects.push(crate::spell_info::SpellEffect {
                index: 1,
                effect: 10,
                implicit_targets: [1, 0],
                base_points: 100.0,
                ..default()
            }),
            _ => unreachable!(),
        }
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(vec![spell]));
        let position = *h.app.world().get::<Position>(h.caster).unwrap();
        let rotation = h.app.world().get::<Rotation>(h.caster).copied();
        learn_effect_toy(&mut h, character, 45984);
        assert!(activate(&mut h, 45984)[0].error.is_some(), "{missing}");
        h.advance(5);
        assert!(h.inbox.go.is_empty(), "{missing}");
        assert!(h.inbox.cooldowns.is_empty(), "{missing}");
        assert_eq!(h.app.world().get::<Position>(h.caster), Some(&position));
        assert_eq!(h.app.world().get::<Rotation>(h.caster).copied(), rotation);
    }
}

#[test]
fn toyfx17_linked_cooking_fires_spawn_real_traps_and_refresh_cozy_fire() {
    use crate::auras::ActiveAuras;
    use shared::components::{Position, Rotation, UnitScale};
    use shared::protocol::GameObjectInfo;
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (item, spell, entry, display, scale) in [
        (70722, 99767, 208935, 10726, 0.25),
        (104309, 148553, 223229, 14358, 0.5),
        (116435, 171549, 236014, 30040, 0.5),
        (116757, 171760, 236110, 7385, 1.0),
        (182780, 340241, 357706, 67291, 0.5),
        (184404, 345745, 364811, 67976, 0.5),
        (199892, 388258, 381360, 75335, 0.7),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("linked-fire.redb"));
        place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
        h.app.insert_resource(store.clone());
        h.app.world_mut().entity_mut(h.caster).insert((
            UnitScale(1.0),
            Rotation {
                x: 0.0,
                y: std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
        ));
        learn_effect_toy(&mut h, character, item);
        let cursor = store
            .get(spell)
            .unwrap()
            .effects
            .iter()
            .any(|effect| effect.implicit_targets.contains(&87));
        let origin = *h.app.world().get::<Position>(h.caster).unwrap();
        let destination = cursor.then_some([origin.x + 3.0, origin.y, origin.z]);
        assert_eq!(
            activate_at(&mut h, item, destination)[0].error,
            None,
            "toy {item}"
        );
        h.advance(100);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
        let world = h.app.world_mut();
        let fire = world
            .query::<(Entity, &GameObjectInfo)>()
            .iter(world)
            .find(|(_, info)| info.entry == entry)
            .unwrap()
            .0;
        let info = world.get::<GameObjectInfo>(fire).unwrap();
        assert_eq!(
            (info.go_type, info.display_id, info.scale),
            (8, display, scale)
        );
        let lifetime = world
            .get::<crate::game_objects::summoned::SummonedObject>(fire)
            .unwrap();
        assert_eq!((lifetime.owner, lifetime.spell_id), (Some(h.caster), spell));
        let source_ms = store
            .get(spell)
            .unwrap()
            .misc
            .as_ref()
            .unwrap()
            .duration
            .unwrap()
            .duration_ms;
        let remaining = lifetime.expires_at.unwrap() - crate::spell_cast::now(world);
        assert!(remaining > f64::from(source_ms / 1000) - 10.0);
        let traps: Vec<_> = world
            .query::<(Entity, &GameObjectInfo)>()
            .iter(world)
            .filter(|(_, info)| info.entry == 31442)
            .map(|(entity, info)| (entity, info.clone()))
            .collect();
        assert_eq!(traps.len(), 1);
        let (trap, info) = &traps[0];
        assert_eq!((info.go_type, info.display_id), (6, 0));
        assert_eq!(world.get::<Position>(fire), world.get::<Position>(*trap));
        assert_eq!(
            world.get::<crate::maps::InMap>(fire),
            world.get::<crate::maps::InMap>(*trap)
        );
        let aura = world
            .get::<ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 7353)
            .unwrap();
        assert_eq!(
            aura.caster, *trap,
            "linked trap is not owned by the toy caster"
        );
        assert_eq!(aura.effects[0].amount, 1.0);
        assert_eq!(
            world
                .get::<crate::stats::StatModifiers>(h.caster)
                .unwrap()
                .ratings_flat
                .versatility,
            1.0
        );
        let fire_position = *world.get::<Position>(fire).unwrap();
        // Once triggered, the 60s buff survives leaving the 10yd spell area.
        world.entity_mut(h.caster).insert(Position {
            x: fire_position.x + 30.0,
            ..fire_position
        });
        h.advance(100);
        assert!(
            h.app
                .world()
                .get::<ActiveAuras>(h.caster)
                .unwrap()
                .iter()
                .any(|aura| aura.spell_id == 7353)
        );
        // Removing the focus despawns its linked trap; the already-cast buff is not an area lease.
        h.app.world_mut().despawn(fire);
        h.advance(2);
        assert!(h.app.world().get_entity(*trap).is_err());
        assert!(
            h.app
                .world()
                .get::<ActiveAuras>(h.caster)
                .unwrap()
                .iter()
                .any(|aura| aura.spell_id == 7353)
        );
        h.advance(1300);
        assert!(
            !h.app
                .world()
                .get::<ActiveAuras>(h.caster)
                .unwrap()
                .iter()
                .any(|aura| aura.spell_id == 7353)
        );
        assert_eq!(
            h.app
                .world()
                .get::<crate::stats::StatModifiers>(h.caster)
                .unwrap()
                .ratings_flat
                .versatility,
            0.0
        );
    }
}

#[test]
fn toyfx17_linked_fire_requires_a_live_player_and_repeats_on_source_cooldown() {
    use crate::auras::{ActiveAuras, RemoveReason};
    use shared::components::{Health, Position, Rotation, UnitScale};
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("trigger.redb"));
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app.world_mut().entity_mut(h.caster).insert((
        UnitScale(1.0),
        Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    learn_effect_toy(&mut h, character, 70722);
    assert_eq!(activate(&mut h, 70722)[0].error, None);
    h.advance(100);
    let fire = {
        let world = h.app.world_mut();
        world
            .query::<(&shared::protocol::GameObjectInfo, &Position)>()
            .iter(world)
            .find(|(info, _)| info.entry == 208935)
            .map(|(_, position)| *position)
            .unwrap()
    };
    h.app.world_mut().entity_mut(h.caster).insert(Position {
        x: fire.x + 30.0,
        ..fire
    });
    crate::auras::remove_auras(h.app.world_mut(), h.caster, RemoveReason::Cancelled, |a| {
        a.spell_id == 7353
    });
    let bystander = spawn_player(&mut h.app, 8, fire.x + 9.0);
    let other_map = spawn_player(&mut h.app, 8, fire.x + 2.0);
    let dead = spawn_player(&mut h.app, 8, fire.x + 2.0);
    for entity in [bystander, other_map, dead] {
        h.app.world_mut().entity_mut(entity).insert(UnitScale(1.0));
    }
    h.app
        .world_mut()
        .entity_mut(other_map)
        .insert(crate::maps::InMap::continent(1));
    h.app.world_mut().get_mut::<Health>(dead).unwrap().current = 0.0;
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: fire.x + 2.0,
        ..fire
    });
    h.advance(100);
    let has_buff = |world: &World, entity| {
        world
            .get::<ActiveAuras>(entity)
            .is_some_and(|auras| auras.iter().any(|a| a.spell_id == 7353))
    };
    assert!(
        !has_buff(h.app.world(), bystander),
        "9yd is inside the spell area, outside the trigger"
    );
    assert!(
        !has_buff(h.app.world(), h.npc),
        "NPC alone cannot activate this environmental trap"
    );
    assert!(!has_buff(h.app.world(), other_map));
    assert!(!has_buff(h.app.world(), dead));
    h.app.world_mut().entity_mut(h.caster).insert(Position {
        x: fire.x + 6.6,
        ..fire
    });
    h.advance(2);
    assert!(
        !has_buff(h.app.world(), bystander),
        "beyond diameter/2 + scaled1.5yd reach"
    );
    h.app.world_mut().entity_mut(h.caster).insert(Position {
        x: fire.x + 6.4,
        ..fire
    });
    h.advance(2);
    assert!(
        has_buff(h.app.world(), bystander),
        "source10yd area is not the5yd trigger radius"
    );
    assert!(!has_buff(h.app.world(), other_map));
    assert!(!has_buff(h.app.world(), dead));
    let deadline = |world: &World| {
        world
            .get::<ActiveAuras>(bystander)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 7353)
            .unwrap()
            .expires_at
    };
    let first = deadline(h.app.world());
    h.advance(40);
    assert_eq!(
        deadline(h.app.world()),
        first,
        "no per-frame refresh before source4s cooldown"
    );
    h.advance(45);
    assert!(
        deadline(h.app.world()) > first,
        "zero charges repeat instead of consuming the trap"
    );
}

#[test]
fn toyfx17_cozy_fire_adds_source_versatility_rating_and_restores_it() {
    use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
    use shared::components::{CombatRatings, Position};
    let mut h = harness_with(Vec::new(), &[7353], 8, true).unwrap();
    h.advance(2);
    let before = *h.app.world().get::<CombatRatings>(h.caster).unwrap();
    // TC creates the linked trap without an owner: its caster is the GO, not the player.
    let trap = h
        .app
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            crate::maps::InMap::continent(0),
        ))
        .id();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: trap,
            target: h.caster,
            spell_id: 7353,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    h.advance(2);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 7353)
        .unwrap();
    assert_eq!(aura.effects[0].misc_values[0], 0x7000_0000);
    assert_eq!(
        aura.effects[0].amount, 1.0,
        "source scaling max level1/coefficient.8"
    );
    let after = *h.app.world().get::<CombatRatings>(h.caster).unwrap();
    assert_eq!(after.versatility, before.versatility + 1.0);
    assert_eq!(after.crit, before.crit);
    assert_eq!(after.haste, before.haste);
    assert_eq!(after.mastery, before.mastery);
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == 7353,
    );
    h.advance(2);
    assert_eq!(
        *h.app.world().get::<CombatRatings>(h.caster).unwrap(),
        before
    );
}

#[test]
fn toyfx16_no_trap_cooking_fires_spawn_with_authored_identity_and_expiry() {
    use crate::game_objects::summoned::SummonedObject;
    use shared::components::{Position, Rotation};
    use shared::protocol::GameObjectInfo;
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (item, spell, entry, display, scale) in [
        (67097, 93636, 207461, 396, 1.0),
        (153039, 254240, 277297, 45521, 0.75),
        (198402, 383081, 379147, 75339, 0.75),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("fire.redb"));
        h.app.insert_resource(store.clone());
        h.app.world_mut().entity_mut(h.caster).insert(Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        });
        learn_effect_toy(&mut h, character, item);
        assert_eq!(activate(&mut h, item)[0].error, None, "toy {item}");
        h.advance(100);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
        let world = h.app.world_mut();
        let objects: Vec<_> = world
            .query::<(Entity, &GameObjectInfo)>()
            .iter(world)
            .filter(|(_, info)| info.entry == entry)
            .map(|(entity, info)| (entity, info.clone()))
            .collect();
        assert_eq!(objects.len(), 1);
        let (object, info) = &objects[0];
        assert_eq!(
            (info.go_type, info.display_id, info.scale),
            (8, display, scale)
        );
        assert_eq!(
            world.get::<crate::maps::InMap>(*object),
            world.get::<crate::maps::InMap>(h.caster)
        );
        let lifetime = world.get::<SummonedObject>(*object).unwrap();
        assert_eq!(lifetime.owner, Some(h.caster));
        assert_eq!(lifetime.spell_id, spell);
        let source_duration = store
            .get(spell)
            .unwrap()
            .misc
            .as_ref()
            .unwrap()
            .duration
            .unwrap()
            .duration_ms;
        assert!(source_duration > 0);
        let remaining = lifetime.expires_at.unwrap() - crate::spell_cast::now(world);
        assert!(remaining > f64::from(source_duration / 1000) - 10.0);
        let placed = *world.get::<Position>(*object).unwrap();
        assert!(
            placed.x > 0.0,
            "authored caster-front destination: {placed:?}"
        );
        // Same real lifecycle boundary, accelerated only by the fixture's virtual clock.
        h.app
            .world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(std::time::Duration::from_secs(7200));
        h.app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs(u64::try_from(source_duration / 1000).unwrap()),
            ));
        h.advance(2);
        assert!(h.app.world().get_entity(*object).is_err());
    }
}

#[test]
fn toyfx16_cooking_recipe_needs_a_live_same_map_focus_before_spending_reagents() {
    use crate::persistence::ClientId;
    use shared::components::Position;
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("cooking.redb"));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::profession::load_profession_data(&conn).unwrap());
    h.app
        .insert_resource(crate::merchant::load_vendor_catalog(&conn).unwrap());
    h.app.world_mut().entity_mut(h.caster).insert((
        ClientId(character),
        shared::components::UnitScale(1.0),
        crate::profession::PlayerProfessions {
            lines: vec![],
            spells: vec![2538],
        },
    ));
    learn_effect_toy(&mut h, character, 67097);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let inventory = InventoryData {
        gold: 0,
        items: vec![InventoryItemData {
            item_guid: persistence::next_id(db, "item_instance"),
            item_id: 2672,
            name: "Stringy Wolf Meat".into(),
            quality: 1,
            required_level: 1,
            stack_count: 2,
            vendor_sell_price: 0,
            unique_auction: false,
        }],
    };
    persistence::save_inventory(db, character, &inventory);
    let before = persistence::load_inventory(db, character);
    let refused = crate::spell_cast::cast_for_player(h.app.world_mut(), h.caster, 2538);
    assert!(
        matches!(refused, Err(crate::spell_cast::CastFailure::Unsupported(ref reason)) if reason == "Requires Cooking Fire"),
        "real cooking incorrectly succeeded without a Cooking Fire: {refused:?}"
    );
    assert_eq!(
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character),
        before
    );
    assert_eq!(activate(&mut h, 67097)[0].error, None);
    h.advance(100);
    let object = {
        let world = h.app.world_mut();
        world
            .query::<(Entity, &shared::protocol::GameObjectInfo)>()
            .iter(world)
            .find(|(_, info)| info.entry == 207461)
            .unwrap()
            .0
    };
    let fire = *h.app.world().get::<Position>(object).unwrap();
    for (distance, map) in [(12.0, 0), (3.0, 1), (0.0, 0)] {
        h.app.world_mut().entity_mut(h.caster).insert((
            Position {
                x: fire.x + distance,
                ..fire
            },
            crate::maps::InMap::continent(map),
        ));
        if distance > 0.0 {
            assert!(crate::spell_cast::cast_for_player(h.app.world_mut(), h.caster, 2538).is_err());
            assert_eq!(
                persistence::load_inventory(
                    &h.app.world().resource::<GameDatabase>().db,
                    character
                ),
                before
            );
        } else {
            assert!(crate::spell_cast::cast_for_player(h.app.world_mut(), h.caster, 2538).is_ok());
            h.advance(100);
            let after = persistence::load_inventory(
                &h.app.world().resource::<GameDatabase>().db,
                character,
            );
            assert_eq!(
                after
                    .items
                    .iter()
                    .find(|item| item.item_id == 2672)
                    .unwrap()
                    .stack_count,
                1
            );
            assert_eq!(
                after
                    .items
                    .iter()
                    .find(|item| item.item_id == 2679)
                    .unwrap()
                    .stack_count,
                1
            );
        }
    }
    let before_interruption =
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character);
    assert!(crate::spell_cast::cast_for_player(h.app.world_mut(), h.caster, 2538).is_ok());
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_some()
    );
    h.app.world_mut().despawn(object);
    h.advance(100);
    assert_eq!(
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character,),
        before_interruption,
        "lost focus during the source cooking cast consumed reagents"
    );
    assert!(
        h.inbox
            .failed
            .iter()
            .any(|failed| failed.detail.as_deref() == Some("Requires Cooking Fire"))
    );
    assert!(crate::spell_cast::cast_for_player(h.app.world_mut(), h.caster, 2538).is_err());
}

#[test]
fn toys_effect_census() {
    report_toy_census(&crate::test_data::world_db_path(), "TOY_CENSUS_REPORT");
}

fn report_toy_census(path: &std::path::Path, report_variable: &str) -> serde_json::Value {
    use std::collections::{BTreeMap, BTreeSet};
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let catalog = toy_catalog::load(&conn).unwrap();
    let (store, _) = crate::spell_info::load_spell_data(&conn).unwrap();
    let mut world = World::new();
    world.insert_resource(store);
    world.insert_resource(crate::spell_cast::toy_scripts::load_toy_texts(&conn).unwrap());
    world.init_resource::<crate::group::GroupRegistry>();
    world.insert_resource(crate::game_objects::summoned::load_templates(&conn).unwrap());
    world.insert_resource(crate::meeting_stone::load_meeting_stone_data(&conn).unwrap());
    world.insert_resource(crate::auras::override_spell_data::load(&conn).unwrap());
    world.insert_resource(shared_world_data(path));
    world.insert_resource(crate::spell_cast::teleport_positions::load_positions(&conn).unwrap());
    world.insert_resource(crate::merchant::load_vendor_catalog(&conn).unwrap());
    world.insert_resource(shared_item_templates(path));
    let has_summon_properties: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='summon_properties')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    if has_summon_properties {
        world.insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    } // Absent local reference data means effect 28 is explicitly unavailable.
    let store = world.resource::<crate::spell_info::SpellStore>();
    let mut unsupported: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    let mut supported_auras: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut effects: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut auras: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut rows = Vec::new();
    for toy in catalog.0.values() {
        let (types, supported) = toy
            .snapshot
            .spell_id
            .map(|spell| toy_catalog::classify_types(&world, spell))
            .unwrap_or_default();
        for aura_type in supported {
            supported_auras
                .entry(aura_type)
                .or_default()
                .insert(toy.snapshot.item_id);
        }
        for reason in &types {
            unsupported
                .entry(reason.clone())
                .or_default()
                .insert(toy.snapshot.item_id);
        }
        if let Some(spell) = toy.snapshot.spell_id.and_then(|id| store.get(id)) {
            for effect in &spell.effects {
                effects
                    .entry(effect.effect)
                    .or_default()
                    .insert(toy.snapshot.item_id);
                if crate::auras::is_aura_effect(effect) {
                    auras
                        .entry(effect.aura)
                        .or_default()
                        .insert(toy.snapshot.item_id);
                }
            }
        }
        rows.push(serde_json::json!({"item_id":toy.snapshot.item_id,"spell_id":toy.snapshot.spell_id,"unsupported":types,"unavailable":toy_catalog::unavailable(toy,&world)}));
    }
    assert_eq!(catalog.0.len(), 1166);
    let candidate_usable = rows
        .iter()
        .filter(|row| row["unavailable"].is_null())
        .count();
    let report = serde_json::json!({"candidate_usable":candidate_usable,"toys":rows,"unsupported":unsupported,
        "supported_aura_toys":supported_auras,"root_effects":effects,"root_auras":auras});
    println!(
        "candidate-usable toys: {candidate_usable}/{}",
        catalog.0.len()
    );
    if let Ok(path) = std::env::var(report_variable) {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    println!("toy types: {}", report["unsupported"]);
    report
}
