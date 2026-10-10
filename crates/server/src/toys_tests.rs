use super::*;
use crate::persistence::{self, InventoryData, InventoryItemData};
use crate::spell_cast::test_harness::*;
use shared::components::{CombatStatus, Level};
use shared::protocol::{InventoryChannel, InventoryError, UseItem};

const FIGURINE: u32 = 32782;
const TRANSFORM: u32 = 41301;
const FLAG_OWNERSHIP: u32 = 38578; // The Flag of Ownership: effect 3, spell 51640.

fn setup(path: &std::path::Path) -> (Harness, u64) {
    let mut h = harness(Vec::new(), &[TRANSFORM, 51640], 8).unwrap();
    let db = redb::Database::create(path).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "ToyMage", 1, 8).unwrap();
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
            items: vec![item(FIGURINE), item(FLAG_OWNERSHIP)],
        },
    );
    let path = crate::test_data::world_db_path();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app
        .insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    h.app
        .insert_resource(crate::world_data::load_world_data(&path));
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
    h.app
        .world_mut()
        .get_mut::<MessageSender<UseToy>>(h.connection)
        .unwrap()
        .send::<CollectionChannel>(UseToy { item_id });
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

#[test]
fn toys_effect_census() {
    use std::collections::{BTreeMap, BTreeSet};
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = toy_catalog::load(&conn).unwrap();
    let (store, _) = crate::spell_info::load_spell_data(&conn).unwrap();
    let mut world = World::new();
    world.insert_resource(store);
    world.insert_resource(crate::world_data::load_world_data(
        &crate::test_data::world_db_path(),
    ));
    world.insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    let store = world.resource::<crate::spell_info::SpellStore>();
    let mut unsupported: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    let mut effects: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut auras: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut rows = Vec::new();
    for toy in catalog.0.values() {
        let types = toy
            .snapshot
            .spell_id
            .map(|spell| toy_catalog::unsupported_types(&world, spell))
            .unwrap_or_default();
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
    let report = serde_json::json!({"toys":rows,"unsupported":unsupported,"root_effects":effects,"root_auras":auras});
    if let Ok(path) = std::env::var("TOY_CENSUS_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    println!("toy types: {}", report["unsupported"]);
}
