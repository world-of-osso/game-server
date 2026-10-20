use super::*;

#[test]
#[ignore = "RED: crafted stacks merge into unknown-origin instances"]
fn crafting_origin_stack_preserves_unknown_item_and_final_surviving_crafted_guid() {
    let mut h = harness(10).unwrap();
    h.learn_tailoring();
    h.give(2996, "Bolt of Linen Cloth", 1);
    let unknown_guid = persistence::load_inventory(h.db(), h.character_id)
        .items
        .into_iter()
        .find(|item| item.item_id == 2996)
        .unwrap()
        .item_guid;
    h.give(2589, "Linen Cloth", 4);
    h.craft(2963, 2);
    h.finish_crafting();
    assert!(h.take::<CastFailed>().is_empty());
    assert_eq!(h.bag_count(2589), 0);
    assert_eq!(h.bag_count(2996), 3);
    let model = crate::inventory::load_model(
        h.db(),
        h.app
            .world()
            .resource::<crate::item_templates::ItemTemplates>(),
        h.character_id,
    )
    .unwrap();
    let wire = serde_json::to_value(model.inventory_snapshot()).unwrap();
    let items: Vec<_> = wire["bags"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|bag| bag["items"].as_array().unwrap())
        .map(|slot| &slot["item"])
        .filter(|item| item["item_id"] == 2996)
        .collect();
    let unknown = items
        .iter()
        .find(|item| item["item_guid"] == unknown_guid)
        .unwrap();
    assert_eq!(
        unknown["count"], 1,
        "actual crafts cannot relabel/merge into an identical unknown-origin stack"
    );
    assert!(
        unknown
            .get("crafting_origin")
            .is_none_or(serde_json::Value::is_null)
    );
    let crafted = items
        .iter()
        .find(|item| item["item_guid"] != unknown_guid)
        .unwrap();
    assert_eq!(
        crafted["count"], 2,
        "same-origin craft output must retain its final surviving stack GUID"
    );
    assert_eq!(
        crafted["crafting_origin"],
        serde_json::json!({"skill_line_ability_id":2359,"recipe_spell_id":2963,"crafting_quality_id":null})
    );
}

#[test]
fn crafting_origin_actual_cast_survives_database_reopen_in_owner_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crafting-origin.redb");
    let db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let mut h = harness_with(10, |app| {
        app.insert_resource(GameDatabase::new(db));
    })
    .unwrap();
    let ability: u32 = h
        .conn
        .query_row(
            "SELECT ID FROM skill_line_ability WHERE Spell=2153",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        ability, 1355,
        "TCmodifier39 identity is the ability row, not spell2153"
    );
    h.set_gold(1000);
    let npc = h.open_trainer(5564);
    h.buy(npc, 264578);
    assert!(h.take::<TrainerBuyFailed>().is_empty());
    h.set_rank(2532, 15);
    h.buy(npc, 2153);
    assert!(h.take::<TrainerBuyFailed>().is_empty());
    assert!(h.professions().knows(2153));
    h.give(2303, "Handstitched Leather Pants", 1);
    let original = persistence::load_inventory(h.db(), h.character_id)
        .items
        .into_iter()
        .find(|item| item.item_id == 2303)
        .unwrap();
    h.give(2318, "Light Leather", 4);
    h.give(2320, "Coarse Thread", 1);
    h.take::<CastFailed>();
    h.craft(2153, 1);
    assert!(h.casting());
    h.finish_crafting();
    assert!(h.take::<CastFailed>().is_empty());
    assert_eq!(h.bag_count(2318), 0);
    assert_eq!(h.bag_count(2320), 0);
    assert_eq!(h.bag_count(2303), 2);
    let inventory = persistence::load_inventory(h.db(), h.character_id);
    let crafted = inventory
        .items
        .iter()
        .find(|item| item.item_id == 2303 && item.item_guid != original.item_guid)
        .unwrap()
        .clone();
    let owner = h.character_id;
    assert!(h.app.world_mut().despawn(h.connection));
    assert!(h.app.world_mut().despawn(h.player));
    drop(
        h.app
            .world_mut()
            .remove_resource::<GameDatabase>()
            .unwrap()
            .into_database(),
    );
    let reopened = redb::Database::open(&path).unwrap();
    assert_eq!(persistence::load_inventory(&reopened, owner), inventory);
    let model = crate::inventory::load_model(
        &reopened,
        h.app
            .world()
            .resource::<crate::item_templates::ItemTemplates>(),
        owner,
    )
    .unwrap();
    let wire = serde_json::to_value(model.inventory_snapshot()).unwrap();
    let items: Vec<_> = wire["bags"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|bag| bag["items"].as_array().unwrap())
        .map(|slot| &slot["item"])
        .collect();
    let crafted_wire = items
        .iter()
        .find(|item| item["item_guid"] == crafted.item_guid)
        .unwrap();
    assert_eq!(
        crafted_wire.get("crafting_origin"),
        Some(
            &serde_json::json!({"skill_line_ability_id": ability,"recipe_spell_id": 2153,"crafting_quality_id": null})
        ),
        "fresh DB/owner snapshot must carry authoritative origin on the final surviving crafted GUID"
    );
    let original_wire = items
        .iter()
        .find(|item| item["item_guid"] == original.item_guid)
        .unwrap();
    assert!(
        original_wire
            .get("crafting_origin")
            .is_none_or(serde_json::Value::is_null),
        "identical non-crafted item must stay unknown"
    );
}
