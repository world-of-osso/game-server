//! Real-DB test against data/world.db `content_item`; skips when the DB is absent.

use std::path::PathBuf;

use super::*;

#[test]
fn content_item_rows_load_inventory_attributes() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
    if !path.exists() {
        eprintln!("skipping: {} not found", path.display());
        return;
    }
    let templates = load_item_templates(&path).unwrap();

    assert!(templates.0.len() > 30_000);
    let sword = templates.get(25).unwrap();
    assert_eq!(
        *sword,
        ItemTemplate {
            inventory_type: 21,
            max_stack: 1,
            bonding: BIND_ON_EQUIP,
            required_level: 1,
            allowable_class: -1,
            allowable_race: u64::MAX,
            scaling: sword.scaling,
            appearance_id: Some(154),
            on_use: None,
        }
    );
    assert!(sword.allows(1, 1));
    let linen = templates.get(2589).unwrap();
    assert_eq!((linen.inventory_type, linen.max_stack), (0, 1000));
    assert_eq!(linen.appearance_id, None, "cloth teaches no appearance");
    // Verigan's Fist: Paladin-only, bind on pickup.
    let fist = templates.get(6953).unwrap();
    assert!(fist.allows(2, 1));
    assert!(!fist.allows(1, 1));
    assert!(fist.binds_on_pickup());
    assert_eq!(fist.appearance_id, Some(2288));
    // Milly's Fire Extinguisher (Extinguishing Hope) casts Spray Water.
    assert_eq!(
        templates
            .get(58362)
            .unwrap()
            .on_use
            .map(|on_use| on_use.spell_id),
        Some(80199)
    );
    assert_eq!(linen.on_use, None);
    // Tough Hunk of Bread: expendable Food in the 1 s "Item - Food" category 11.
    assert_eq!(
        templates.get(4540).unwrap().on_use,
        Some(OnUseSpell {
            spell_id: 1_291_822,
            charges: -1,
            cooldown: crate::spell_cast::ItemCooldown {
                category: 11,
                category_ms: 1000,
            },
        })
    );
}
