use super::*;
use shared::profession::{PlayerProfessions, Profession};

fn learned_professions() -> PlayerProfessions {
    let mut profs = PlayerProfessions::default();
    profs.learn(Profession::Mining).unwrap();
    profs.learn(Profession::Blacksmithing).unwrap();
    profs
}

#[test]
fn build_profession_snapshot_includes_skills_and_craftable_recipe() {
    let profs = learned_professions();
    let inventory = InventoryData {
        gold: 0,
        items: vec![InventoryItemData {
            item_guid: 1,
            item_id: 2770,
            name: "Copper Ore".into(),
            quality: 1,
            required_level: 1,
            stack_count: 4,
            vendor_sell_price: 25,
            unique_auction: false,
        }],
    };

    let snapshot = build_profession_snapshot(&profs, &inventory);

    assert_eq!(snapshot.skills.len(), 2);
    assert!(
        snapshot
            .skills
            .iter()
            .any(|skill| skill.profession == "Mining")
    );
    assert!(
        snapshot
            .recipes
            .iter()
            .any(|recipe| recipe.name == "Copper Bracers" && recipe.craftable)
    );
}

#[test]
fn craft_recipe_consumes_materials_adds_output_and_skillup() {
    let db = test_db();
    let mut profs = learned_professions();
    let mut inventory = InventoryData {
        gold: 0,
        items: vec![InventoryItemData {
            item_guid: 1,
            item_id: 2770,
            name: "Copper Ore".into(),
            quality: 1,
            required_level: 1,
            stack_count: 4,
            vendor_sell_price: 25,
            unique_auction: false,
        }],
    };

    let result = craft_recipe(
        RECIPE_COPPER_BRACERS,
        &mut profs,
        &mut inventory,
        &db.db,
        0.0,
    )
    .expect("craft succeeds");

    assert!(result.message.contains("crafted Copper Bracers"));
    assert_eq!(
        result.skill_up.as_ref().expect("skill-up").profession,
        "Blacksmithing"
    );
    assert_eq!(count_inventory_item(&inventory, 2770), 2);
    assert!(
        inventory
            .items
            .iter()
            .any(|item| item.item_id == 2851 && item.name == "Copper Bracers")
    );
}

#[test]
fn gather_node_adds_items_and_reports_skillup() {
    let db = test_db();
    let mut profs = learned_professions();
    let mut inventory = InventoryData {
        gold: 0,
        items: Vec::new(),
    };

    let result = complete_gather(
        NODE_COPPER_VEIN,
        &mut profs,
        &mut inventory,
        &db.db,
        0.0,
        &[0.0],
    )
    .expect("gather succeeds");

    assert!(result.message.contains("gathered Copper Vein"));
    assert_eq!(
        result.skill_up.as_ref().expect("skill-up").profession,
        "Mining"
    );
    assert!(count_inventory_item(&inventory, 2770) >= 1);
}
