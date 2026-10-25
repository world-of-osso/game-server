//! Native1804 + Retail4638/Lock3240, through owned bag GUID cast input.
use super::test_harness::*;
use crate::persistence::{self, ClientId, GameDatabase};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Level, UnitLevel};
use shared::item_data::ItemDefinitionSource::Retail;
use shared::protocol::{CombatChannel, SpellCastIntent};

fn fixture(item_id: u32) -> (Harness, tempfile::TempDir, u64) {
    let path = crate::test_data::world_db_path();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let mut h = harness(Vec::new(), &[1804], 4).unwrap();
    h.app
        .insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    h.app
        .insert_resource(crate::merchant::load_vendor_catalog(&conn).unwrap());
    h.app.init_resource::<crate::gold::GoldUpdateQueue>();
    h.app
        .insert_resource(super::open_lock::load_data(path.to_str().unwrap()).unwrap());
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("lockbox.redb")).unwrap();
    persistence::create_tables(&db);
    let (id, _) = persistence::create_character(&db, 1, "Lockpicker", 1, 4).unwrap();
    let templates = h
        .app
        .world()
        .resource::<crate::item_templates::ItemTemplates>();
    assert!(templates.get(Retail, item_id).is_some());
    crate::inventory::grant_item(
        &db,
        templates,
        h.app.world().resource::<crate::merchant::VendorCatalog>(),
        id,
        (item_id, 1),
    )
    .unwrap();
    let model = crate::inventory::load_model(&db, templates, id).unwrap();
    let guid = model
        .bag_items()
        .find(|item| item.item_id == item_id)
        .unwrap()
        .item_guid;
    h.app.insert_resource(GameDatabase::new(db));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((ClientId(id), Level(29), UnitLevel(29)));
    (h, directory, guid)
}

fn send_item_cast(h: &mut Harness, guid: u64) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<SpellCastIntent>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(SpellCastIntent {
            spell_id: Some(1804),
            spell: String::new(),
            target_entity: None,
            target_item_guid: Some(guid),
            destination: None,
            witness: None,
        });
}

fn cast_item(h: &mut Harness, guid: u64) {
    send_item_cast(h, guid);
    h.advance(150);
}

#[test]
fn class_completeness_native_pick_lock_accepts_owned_bag_item_without_consuming_it() {
    let (mut h, directory, guid) = fixture(4638);
    cast_item(&mut h, guid);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 1804));
    let model = crate::inventory::load_model(
        &h.app.world().resource::<GameDatabase>().db,
        h.app
            .world()
            .resource::<crate::item_templates::ItemTemplates>(),
        1,
    )
    .unwrap();
    assert_eq!(model.bag_item(guid).unwrap().stack_count, 1);
    // UNLOCKED is an item-instance flag, not transient caster state.
    let db = h.app.world_mut().remove_resource::<GameDatabase>().unwrap();
    drop(db);
    h.app.insert_resource(GameDatabase::new(
        redb::Database::open(directory.path().join("lockbox.redb")).unwrap(),
    ));
    h.inbox = Inbox::default();
    cast_item(&mut h, guid);
    assert!(
        h.inbox.go.is_empty(),
        "already-unlocked item must reject recast"
    );
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::InvalidTarget
    );
}

#[test]
fn class_completeness_native_pick_lock_rechecks_owned_item_at_completion() {
    let (mut h, _directory, guid) = fixture(4638);
    send_item_cast(&mut h, guid);
    h.advance(3);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_some()
    );
    let db = &h.app.world().resource::<GameDatabase>().db;
    let mut inventory = persistence::load_inventory(db, 1);
    inventory.items.retain(|item| item.item_guid != guid);
    persistence::save_inventory(db, 1, &inventory);
    h.advance(150);
    assert!(h.inbox.go.is_empty());
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::InvalidTarget
    );
    assert!(!persistence::inventory_store::is_item_unlocked(
        &h.app.world().resource::<GameDatabase>().db,
        guid
    ));
}

#[test]
fn class_completeness_native_pick_lock_rejects_insufficient_level_points() {
    let (mut h, _directory, guid) = fixture(4638);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(28), UnitLevel(28)));
    cast_item(&mut h, guid);
    assert!(h.inbox.go.is_empty());
    assert_eq!(
        h.inbox.failed[0].detail.as_deref(),
        Some("SPELL_FAILED_LOW_CASTLEVEL")
    );
}

#[test]
fn class_completeness_native_pick_lock_rejects_foreign_missing_and_unlocked_template() {
    let (mut h, _directory, guid) = fixture(727); // Native LockID0 is not locked.
    cast_item(&mut h, guid);
    assert!(h.inbox.go.is_empty());
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::InvalidTarget
    );
    for target in [0, u64::MAX] {
        let (mut h, _directory, _guid) = fixture(4638);
        cast_item(&mut h, target);
        assert!(h.inbox.go.is_empty());
        assert_eq!(
            h.inbox.failed[0].reason,
            shared::spell_data::CastFailReason::InvalidTarget
        );
    }
    let (mut h, _directory, guid) = fixture(4638);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let (other, _) = persistence::create_character(db, 1, "Other", 1, 4).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ClientId(other));
    cast_item(&mut h, guid);
    assert!(h.inbox.go.is_empty());
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::InvalidTarget
    );
}
