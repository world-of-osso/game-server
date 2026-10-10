use super::*;
use crate::persistence::{self, InventoryData, InventoryItemData};
use crate::spell_cast::test_harness::*;
use shared::components::{CombatStatus, Level};
use shared::protocol::{InventoryChannel, InventoryError, UseItem};

const FIGURINE: u32 = 32782;
const TRANSFORM: u32 = 41301;
const FLAG_OWNERSHIP: u32 = 38578; // The Flag of Ownership: effect 3, spell 51640.

fn setup(path: &std::path::Path) -> (Harness, u64) {
    let mut h = harness(
        Vec::new(),
        &[
            TRANSFORM, 51640, 61810, 45426, 176594, 176457, 185712, 302273, 98444, 141917, 156132,
            99104, 304370, 23126, 167273, 65451, 176889, 225656, 225658, 225660, 225657, 225659,
            376088, 116, 376092, 376155, 376203, 397775, 376172, 376113,
        ],
        8,
    )
    .unwrap();
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
            items: vec![item(FIGURINE), item(FLAG_OWNERSHIP), item(188698)],
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
fn toys_category_override_expires_authoritatively() {
    // Eagger Basket: ItemEffect category1203/10s; SpellCooldowns recovery0,
    // spell category44 must not replace the item category.
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&dir.path().join("toys.redb"));
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
    assert_eq!(learn(h, 0)[0].error, None);
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
    // Shadescale170187 has no runtime content_item, so its actual UseToy must
    // remain unavailable. The authentic on-use spell proves the aura mechanism.
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&dir.path().join("toys.redb"));
    assert!(
        snapshot(h.app.world(), 1)
            .toys
            .iter()
            .find(|t| t.item_id == 170187)
            .unwrap()
            .unavailable_reason
            .as_deref()
            .unwrap()
            .contains("content_item")
    );
    assert!(toy_catalog::unsupported_types(h.app.world(), 304370).is_empty());
    h.cast_and_deliver(304370, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let auras = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap();
    let aura = auras.iter().find(|a| a.spell_id == 304370).unwrap();
    assert_eq!(aura.effects[0].aura_type, 260);
    assert_eq!(aura.effects[0].misc_values, [1720, 7]);
    assert!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 304370)
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura { spell_id: 304370 });
    h.advance(5);
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 304370)
    );
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
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE summon_properties (ID INTEGER, Control INTEGER, Faction INTEGER, Title INTEGER, Slot INTEGER); INSERT INTO summon_properties VALUES (64,0,0,0,0);").unwrap();
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
    let (mut h, _) = setup(&dir.path().join("toys.redb"));
    //113542 has no runtime item: do not invent one or count it as usable.
    assert!(
        snapshot(h.app.world(), 1)
            .toys
            .iter()
            .find(|t| t.item_id == 113542)
            .unwrap()
            .unavailable_reason
            .as_deref()
            .unwrap()
            .contains("content_item")
    );
    assert!(toy_catalog::unsupported_types(h.app.world(), 167273).is_empty());
    h.cast_and_deliver(167273, None);
    h.advance(40); // Authentic one-second cast, not an immediate aura fixture.
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
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

#[test]
fn toys_effect_census() {
    use std::collections::{BTreeMap, BTreeSet};
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = toy_catalog::load(&conn).unwrap();
    let (store, _) = crate::spell_info::load_spell_data(&conn).unwrap();
    let mut world = World::new();
    world.insert_resource(store);
    world.insert_resource(crate::auras::override_spell_data::load(&conn).unwrap());
    world.insert_resource(crate::world_data::load_world_data(
        &crate::test_data::world_db_path(),
    ));
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
    if let Ok(path) = std::env::var("TOY_CENSUS_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    println!("toy types: {}", report["unsupported"]);
}
