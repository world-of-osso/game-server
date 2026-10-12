//! Native1804 + Retail4638/Lock3240, through owned bag GUID cast input.
use super::test_harness::*;
use crate::persistence::{self, ClientId, GameDatabase};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Level, UnitLevel};
use shared::item_data::ItemDefinitionSource::Retail;
use shared::protocol::{CombatChannel, SpellCastIntent};

fn fixture() -> (Harness, tempfile::TempDir, u64) {
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
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("lockbox.redb")).unwrap();
    persistence::create_tables(&db);
    let (id, _) = persistence::create_character(&db, 1, "Lockpicker", 1, 4).unwrap();
    let templates = h
        .app
        .world()
        .resource::<crate::item_templates::ItemTemplates>();
    assert!(templates.get(Retail, 4638).is_some());
    crate::inventory::grant_item(
        &db,
        templates,
        h.app.world().resource::<crate::merchant::VendorCatalog>(),
        id,
        (4638, 1),
    )
    .unwrap();
    let model = crate::inventory::load_model(&db, templates, id).unwrap();
    let guid = model
        .bag_items()
        .find(|item| item.item_id == 4638)
        .unwrap()
        .item_guid;
    h.app.insert_resource(GameDatabase { db });
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((ClientId(id), Level(29), UnitLevel(29)));
    (h, directory, guid)
}

fn cast_item(h: &mut Harness, guid: u64) {
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
    h.advance(150);
}

#[test]
fn class_completeness_native_pick_lock_accepts_owned_bag_item_without_consuming_it() {
    let (mut h, _directory, guid) = fixture();
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
    assert_eq!(model.bag_item(guid).unwrap().count, 1);
}
