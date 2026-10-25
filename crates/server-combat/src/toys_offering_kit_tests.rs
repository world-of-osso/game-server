//! Source279 and419: actual learned item creation and source recovery.
use super::*;

fn load_created_item_count(h: &Harness, character: u64, item_id: u32) -> u32 {
    let db = &h.app.world().resource::<GameDatabase>().db;
    persistence::load_inventory(db, character)
        .items
        .iter()
        .filter(|item| item.item_id == item_id)
        .map(|item| item.stack_count)
        .sum()
}

#[test]
fn toyfx279_offering_kit_maker_creates_one_persisted_kit_and_preserves_recovery() {
    const TOY: u32 = 187344;
    const SPELL: u32 = 357020;
    const PRODUCT: u32 = 187342;
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("offering.redb"));
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
        .insert(crate::persistence::ClientId(character));
    learn_effect_toy(&mut h, character, TOY);
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 0);
    assert_eq!(activate(&mut h, TOY)[0].error, None);
    h.advance(20); // Only one second: the authored two-second cast is pending.
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 0);
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == SPELL));
    h.advance(30);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == SPELL));
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 1);
    let cooldown = h
        .inbox
        .cooldowns
        .iter()
        .find(|cooldown| cooldown.spell_id == SPELL && !cooldown.is_gcd)
        .unwrap();
    assert_eq!(cooldown.duration_ms, 30_000);
    assert_eq!(
        activate(&mut h, TOY)[0].error.as_deref(),
        Some("OnCooldown")
    );
    h.advance(500); // Ordinary 25 seconds, still inside source item recovery.
    assert_eq!(
        activate(&mut h, TOY)[0].error.as_deref(),
        Some("OnCooldown")
    );
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 1);
    h.advance(140); // Ordinary seven seconds, beyond the original recovery.
    assert_eq!(activate(&mut h, TOY)[0].error, None);
    h.advance(50);
    let rejections: Vec<_> = h
        .inbox
        .failed
        .iter()
        .map(|failure| (failure.spell_id, failure.reason))
        .collect();
    assert_eq!(
        rejections,
        vec![(SPELL, shared::spell_data::CastFailReason::OnCooldown); 2]
    );
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 2);
}

#[test]
fn toyfx419_doomsday_repeat_after_ordinary_recovery_preserves_unique_product() {
    const TOY: u32 = 202309;
    const SPELL: u32 = 398572;
    const PRODUCT: u32 = 202310;
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("doomsday-repeat.redb"));
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::profession::load_profession_data(&conn).unwrap());
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::merchant::load_vendor_catalog(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(character));
    learn_effect_toy(&mut h, character, TOY);
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 0);
    assert_eq!(activate(&mut h, TOY)[0].error, None);
    h.advance(70);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 1);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let first_inventory = persistence::load_inventory(db, character);
    let first_product = first_inventory
        .items
        .iter()
        .find(|item| item.item_id == PRODUCT)
        .unwrap();
    let first_guid = first_product.item_guid;
    let first_completed = crate::auras::aura_now(h.app.world());

    // Keep the real first product. Do not clear bags or alter the300s cooldown.
    h.advance(5900);
    assert!(crate::auras::aura_now(h.app.world()) - first_completed < 300.0);
    assert_eq!(
        activate(&mut h, TOY)[0].error.as_deref(),
        Some("OnCooldown")
    );
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == SPELL).count(),
        1
    );
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 1);
    h.advance(120);
    assert!(crate::auras::aura_now(h.app.world()) - first_completed > 300.0);
    assert_eq!(
        activate(&mut h, TOY)[0].error.as_deref(),
        Some("You can't carry any more of those items.")
    );
    h.advance(70);
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == SPELL).count(),
        1
    );
    assert_eq!(h.inbox.inventory_errors.len(), 1);
    assert_eq!(
        h.inbox.inventory_errors[0].reason,
        shared::protocol::InventoryErrorReason::ItemMaxCount
    );
    assert!(
        !h.inbox
            .failed
            .iter()
            .any(|failure| failure.reason == shared::spell_data::CastFailReason::InventoryFull)
    );
    let db = &h.app.world().resource::<GameDatabase>().db;
    let final_inventory = persistence::load_inventory(db, character);
    let products: Vec<_> = final_inventory
        .items
        .iter()
        .filter(|item| item.item_id == PRODUCT)
        .map(|item| (item.item_guid, item.stack_count))
        .collect();
    assert_eq!(
        products,
        vec![(first_guid, 1)],
        "source product202310 MaxCount1 must not duplicate or replace the first instance"
    );
    assert_eq!(
        load_created_item_count(&h, character, TOY),
        0,
        "learning consumed toy202309; creation must not mint the same-name toy"
    );
}

#[test]
fn toyfx419_doomsday_triggered_effect_cap_preserves_original_product_guid() {
    const SPELL: u32 = 398572;
    const PRODUCT: u32 = 202310;
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("doomsday-triggered.redb"));
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
        .insert_resource(crate::profession::load_profession_data(&conn).unwrap());
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::persistence::ClientId(character));
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        caster,
        SPELL,
        Some(caster),
        1
    ));
    h.advance(5);
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 1);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let before = persistence::load_inventory(db, character);
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        caster,
        SPELL,
        Some(caster),
        1
    ));
    h.advance(5);
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(persistence::load_inventory(db, character), before);
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 1);
}

#[test]
fn toyfx419_doomsday_full_bags_keep_inventory_full_not_item_cap_error() {
    const TOY: u32 = 202309;
    const SPELL: u32 = 398572;
    const PRODUCT: u32 = 202310;
    const FILLER: u32 = 25; // Worn Shortsword: stack1, MaxCount0.
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("doomsday-full.redb"));
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
        .insert_resource(crate::profession::load_profession_data(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(character));
    learn_effect_toy(&mut h, character, TOY);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let templates = h
        .app
        .world()
        .resource::<crate::item_templates::ItemTemplates>();
    let slots = crate::inventory::free_bag_cells(db, templates, character) as u32;
    assert!(slots > 0);
    crate::inventory::grant_item(
        db,
        templates,
        h.app.world().resource::<crate::merchant::VendorCatalog>(),
        character,
        (FILLER, slots),
    )
    .unwrap();
    assert_eq!(
        crate::inventory::free_bag_cells(db, templates, character),
        0
    );
    let before = persistence::load_inventory(db, character);
    assert_eq!(
        activate(&mut h, TOY)[0].error.as_deref(),
        Some("InventoryFull")
    );
    assert!(h.inbox.inventory_errors.is_empty());
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::InventoryFull
    );
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == SPELL));
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(persistence::load_inventory(db, character), before);
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 0);
}

#[test]
fn toyfx419_defective_doomsday_creates_one_persisted_product_after_source_cast() {
    const TOY: u32 = 202309;
    const SPELL: u32 = 398572;
    const PRODUCT: u32 = 202310;
    const RECOVERY_MS: u32 = 300_000;
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("doomsday.redb"));
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
        .insert(crate::persistence::ClientId(character));
    learn_effect_toy(&mut h, character, TOY);
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 0);
    assert_eq!(activate(&mut h, TOY)[0].error, None);
    h.advance(20); // The source three-second cast must not create its item early.
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 0);
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == SPELL));
    h.advance(50);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == SPELL).count(),
        1
    );
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 1);
    assert_eq!(
        h.inbox
            .cooldowns
            .iter()
            .find(|cooldown| cooldown.spell_id == SPELL && !cooldown.is_gcd)
            .unwrap()
            .duration_ms,
        RECOVERY_MS
    );
    assert_eq!(
        activate(&mut h, TOY)[0].error.as_deref(),
        Some("OnCooldown")
    );
    assert_eq!(load_created_item_count(&h, character, PRODUCT), 1);
    let rejections: Vec<_> = h
        .inbox
        .failed
        .iter()
        .map(|failure| (failure.spell_id, failure.reason))
        .collect();
    assert_eq!(
        rejections,
        vec![(SPELL, shared::spell_data::CastFailReason::OnCooldown)]
    );
}
