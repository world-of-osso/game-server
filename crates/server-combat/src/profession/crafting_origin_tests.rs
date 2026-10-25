use super::*;

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

fn origin_rows(h: &Harness, item_id: u32) -> Vec<(u64, shared::protocol::CraftingOrigin)> {
    persistence::load_inventory(h.db(), h.character_id)
        .items
        .iter()
        .filter(|item| item.item_id == item_id)
        .filter_map(|item| {
            persistence::inventory_store::load_item_crafting_origin(h.db(), item.item_guid)
                .map(|origin| (item.item_guid, origin))
        })
        .collect()
}

#[test]
fn crafting_origin_stackable_craft_output_merges_with_looted_stack() {
    let mut h = harness(10).unwrap();
    assert!(
        h.app
            .world()
            .resource::<ProfessionData>()
            .crafting_origin(212205)
            .is_none(),
        "Rogue Create:CrimsonVial is a class ability, not a profession recipe"
    );
    h.learn_tailoring();
    h.give(2996, "Bolt of Linen Cloth", 1);
    let looted = persistence::load_inventory(h.db(), h.character_id)
        .items
        .into_iter()
        .find(|item| item.item_id == 2996)
        .unwrap()
        .item_guid;
    h.give(2589, "Linen Cloth", 4);
    h.craft(2963, 2);
    h.finish_crafting();
    assert!(h.take::<CastFailed>().is_empty());
    let stacks: Vec<_> = persistence::load_inventory(h.db(), h.character_id)
        .items
        .into_iter()
        .filter(|item| item.item_id == 2996)
        .map(|item| (item.item_guid, item.stack_count))
        .collect();
    assert_eq!(
        stacks,
        vec![(looted, 3)],
        "crafted Bolt of Linen Cloth merges into the looted stack"
    );
    assert_eq!(
        origin_rows(&h, 2996),
        vec![],
        "stackable output carries no origin"
    );
}

#[test]
fn crafting_origin_persisted_stackable_row_is_dropped_at_load() {
    let mut h = harness(10).unwrap();
    h.give(2318, "Light Leather", 10);
    let templates = h
        .app
        .world()
        .resource::<crate::item_templates::ItemTemplates>();
    let mut model = crate::inventory::load_model(h.db(), templates, h.character_id).unwrap();
    let legacy = persistence::load_inventory(h.db(), h.character_id).items[0].item_guid;
    // A row written before the policy: Light Leather crafted by 2881 (ability 1708).
    model.record_crafting_origin(
        legacy,
        shared::protocol::CraftingOrigin {
            skill_line_ability_id: 1708,
            recipe_spell_id: 2881,
            crafting_quality_id: None,
        },
    );
    persistence::inventory_store::save_inventory_record(
        h.db(),
        h.character_id,
        &model.into_record(),
    );
    let reloaded = crate::inventory::load_model(h.db(), templates, h.character_id).unwrap();
    let legacy_view = reloaded
        .view()
        .into_values()
        .find(|item| item.item_guid == legacy)
        .unwrap();
    assert_eq!(
        legacy_view.crafting_origin, None,
        "stackable origin is dropped at load"
    );
    // `give` appends a raw row; the real store path is what merges stacks.
    store_created_item(h.app.world_mut(), h.player, 2318, 1, None).unwrap();
    let stacks: Vec<_> = persistence::load_inventory(h.db(), h.character_id)
        .items
        .into_iter()
        .filter(|item| item.item_id == 2318)
        .map(|item| (item.item_guid, item.stack_count))
        .collect();
    assert_eq!(
        stacks,
        vec![(legacy, 11)],
        "looted Light Leather merges into the legacy stack"
    );
}
