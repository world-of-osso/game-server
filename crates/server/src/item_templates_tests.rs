//! Real-DB test against data/world.db `content_item`.

use super::*;

#[test]
fn mount_journal_chestnut_mare_item_retains_authentic_learn_spell() {
    let templates = load_item_templates(&crate::test_data::world_db_path()).unwrap();
    let mare = templates.retail.get(&5655).expect("Chestnut Mare item");
    assert_eq!(mare.on_use.as_ref().map(|spell| spell.spell_id), Some(6648));
}

#[test]
fn mount_journal_teaching_items_never_collect_transmog_even_with_appearance_metadata() {
    let conn = Connection::open_in_memory().unwrap();
    let path = crate::test_data::world_db_path();
    conn.execute(
        "ATTACH DATABASE ?1 AS authentic",
        [format!("file:{}?mode=ro", path.display())],
    )
    .unwrap();
    for table in ["content_item", "item_sparse"] {
        conn.execute_batch(&format!(
            "CREATE TABLE {table} AS SELECT * FROM authentic.{table} WHERE ID IN (25,5655)"
        ))
        .unwrap();
    }
    conn.execute_batch("CREATE TABLE item_x_item_effect AS SELECT * FROM authentic.item_x_item_effect WHERE ItemID=5655;
        CREATE TABLE item_effect AS SELECT * FROM authentic.item_effect WHERE ID IN (SELECT ItemEffectID FROM item_x_item_effect);
        CREATE TABLE mount AS SELECT * FROM authentic.mount WHERE SourceSpellID=6648;
        CREATE TABLE spell_effect AS SELECT * FROM authentic.spell_effect WHERE SpellID=6648;").unwrap();
    conn.execute_batch("CREATE TABLE item_modified_appearance AS SELECT * FROM authentic.item_modified_appearance WHERE ItemID=25; UPDATE item_modified_appearance SET ItemID=5655;").unwrap();
    let templates = load_item_template_rows(&conn, ITEM_QUERY).unwrap();
    assert_eq!(templates[&5655].on_use.unwrap().spell_id, 6648);
    assert_eq!(
        templates[&5655].appearance_id, None,
        "Learning/destroying a mount item must not collect transmog"
    );
    conn.execute("UPDATE item_modified_appearance SET ItemID=25", [])
        .unwrap();
    let templates = load_item_template_rows(&conn, ITEM_QUERY).unwrap();
    assert_eq!(
        templates[&25].appearance_id,
        Some(154),
        "Ordinary equipment still collects its appearance"
    );
}

#[test]
fn content_item_rows_load_inventory_attributes() {
    let path = crate::test_data::world_db_path();
    let templates = load_item_templates(&path).unwrap();

    assert!(templates.retail.len() > 30_000);
    let sword = templates
        .get(shared::item_data::ItemDefinitionSource::Retail, 25)
        .unwrap();
    assert_eq!(
        *sword,
        ItemTemplate {
            inventory_type: 21,
            max_stack: 1,
            max_count: 0,
            bonding: BIND_ON_EQUIP,
            required_level: 1,
            allowable_class: -1,
            allowable_race: u64::MAX,
            scaling: sword.scaling,
            appearance_id: Some(154),
            on_use: None,
            container_slots: 0,
            bag_family: 0,
            unique_equipped: false,
            equip_limit: None,
        }
    );
    assert!(sword.allows(1, 1));
    let linen = templates
        .get(shared::item_data::ItemDefinitionSource::Retail, 2589)
        .unwrap();
    assert_eq!((linen.inventory_type, linen.max_stack), (0, 1000));
    assert_eq!(linen.appearance_id, None, "cloth teaches no appearance");
    // Verigan's Fist: Paladin-only, bind on pickup.
    let fist = templates
        .get(shared::item_data::ItemDefinitionSource::Retail, 6953)
        .unwrap();
    assert!(fist.allows(2, 1));
    assert!(!fist.allows(1, 1));
    assert!(fist.binds_on_pickup());
    assert_eq!(fist.appearance_id, Some(2288));
    // Milly's Fire Extinguisher (Extinguishing Hope) casts Spray Water.
    assert_eq!(
        templates
            .get(shared::item_data::ItemDefinitionSource::Retail, 58362)
            .unwrap()
            .on_use
            .map(|on_use| on_use.spell_id),
        Some(80199)
    );
    assert_eq!(linen.on_use, None);
    // Tough Hunk of Bread: expendable Food in the 1 s "Item - Food" category 11.
    assert_eq!(
        templates
            .get(shared::item_data::ItemDefinitionSource::Retail, 4540)
            .unwrap()
            .on_use,
        Some(OnUseSpell {
            spell_id: 1_291_822,
            charges: -1,
            cooldown: crate::spell_cast::ItemCooldown {
                category: 11,
                category_ms: 1000,
            },
        })
    );
    // Wildgrove Haranir Longbow, Hunter-only: AllowableRaces_0 1572864 holds the
    // PlayableRaceBits of Haranir Horde 91 (19) and Alliance 86 (20).
    let longbow = templates
        .get(shared::item_data::ItemDefinitionSource::Retail, 266263)
        .unwrap();
    assert!(longbow.allows(3, 86));
    assert!(longbow.allows(3, 91));
    assert!(!longbow.allows(3, 1));
}

/// TrinityCore `ItemCanGoIntoBag` on real containers: Small Brown Pouch (plain,
/// 6 slots), Simply Stitched Reagent Bag (32) and Herb Pouch (12, herbs).
#[test]
fn containers_take_only_the_items_their_type_allows() {
    let path = crate::test_data::world_db_path();
    let templates = load_item_templates(&path).unwrap();
    let item = |id| {
        *templates
            .get(shared::item_data::ItemDefinitionSource::Retail, id)
            .unwrap()
    };
    let (pouch, reagent_bag, herb_pouch) = (item(4496), item(194019), item(22250));
    let (sword, linen, peacebloom) = (item(25), item(2589), item(2447));

    assert_eq!(
        (
            pouch.container_slots,
            reagent_bag.container_slots,
            herb_pouch.container_slots
        ),
        (6, 32, 12)
    );
    assert!(!pouch.is_reagent_bag());
    assert!(reagent_bag.is_reagent_bag());
    assert!(sword.can_go_into_bag(&pouch));
    assert!(linen.can_go_into_bag(&reagent_bag));
    assert!(!sword.can_go_into_bag(&reagent_bag));
    assert!(peacebloom.can_go_into_bag(&herb_pouch));
    assert!(!linen.can_go_into_bag(&herb_pouch));
    assert!(!linen.can_go_into_bag(&sword), "a sword is no container");
}
