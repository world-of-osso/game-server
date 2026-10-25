use super::*;
use crate::owned_commerce_source_tests::{Catalog, assert_owned};
use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};
use shared::protocol::CloseInteraction;

#[test]
fn owned_commerce_sources_buyback_keeps_both_catalogs_across_vendor_and_storage_reopen() {
    let catalog = Catalog::load();
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = harness().expect("real merchant fixture");
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    h.character_id = crate::owned_commerce_source_tests::character(&db, "Shopper");
    h.app.world_mut().entity_mut(h.connection).insert(InWorld {
        character_id: h.character_id,
    });
    h.app.insert_resource(GameDatabase::new(db));
    h.app.insert_resource(catalog.templates.clone());
    let guids = catalog.seed(
        &h.app.world().resource::<GameDatabase>().db,
        h.character_id,
        &[(Retail, 2101, 1), (Forever70205, 2101, 1)],
    );
    let expected = [
        (guids[0], 2101, Retail, 1),
        (guids[1], 2101, Forever70205, 1),
    ];
    h.app.update();
    let npc = h.open(GODRIC_ROTHGAR);
    h.take::<BuybackList>();
    for &guid in &guids {
        h.sell(npc, guid, 0);
    }
    assert!(h.failures().is_empty());
    assert_owned(
        &h.app.world().resource::<GameDatabase>().db,
        &catalog,
        h.character_id,
        &[],
    );
    assert_eq!(h.inventory().gold, 10_002);
    let listed = h.take::<BuybackList>().pop().unwrap();
    let sources: Vec<_> = listed
        .items
        .iter()
        .map(|item| (item.item_id, item.definition_source, item.count, item.price))
        .collect();
    assert_eq!(
        sources,
        vec![(2101, Retail, 1, 1), (2101, Forever70205, 1, 1)]
    );

    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert(pair::<CloseInteraction>());
    h.send::<_, InteractionChannel>(CloseInteraction { npc: npc.to_bits() });
    assert!(
        h.app
            .world()
            .get::<ActiveInteraction>(h.connection)
            .is_none()
    );
    // Buyback lives on the connection, not disk. Keep that production session alive.
    drop(h.app.world_mut().remove_resource::<GameDatabase>().unwrap());
    h.app.insert_resource(GameDatabase::new(
        redb::Database::open(file.path()).unwrap(),
    ));
    h.send::<_, InteractionChannel>(InteractNpc { npc: npc.to_bits() });
    assert_eq!(h.take::<BuybackList>(), vec![listed]);
    for slot in 0..2 {
        h.send::<_, MerchantChannel>(BuybackItemRequest {
            npc: npc.to_bits(),
            slot,
        });
    }
    assert!(h.failures().is_empty());
    assert_eq!(h.inventory().gold, 10_000);
    assert!(h.take::<BuybackList>().last().unwrap().items.is_empty());
    assert_owned(
        &h.app.world().resource::<GameDatabase>().db,
        &catalog,
        h.character_id,
        &expected,
    );
    let owner = h.character_id;
    drop(h);

    let db = redb::Database::open(file.path()).unwrap();
    assert_owned(&db, &catalog, owner, &expected);
    assert_eq!(persistence::load_inventory(&db, owner).gold, 10_000);
}
