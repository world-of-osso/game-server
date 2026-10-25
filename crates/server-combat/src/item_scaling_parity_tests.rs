use super::*;
use crate::item_templates::load_item_templates;
use shared::item_data::ItemDefinitionSource;

#[test]
fn local_rows_for_tooltip_parity() {
    let path = crate::test_data::world_db_path();
    let templates = load_item_templates(&path).unwrap();
    let tables = load_item_scaling_tables(&path).unwrap();
    for id in [1273, 6268, 937, 25, 3191, 1925, 11287] {
        let template = templates.get(ItemDefinitionSource::Retail, id).unwrap();
        let result = item_stats(template, &tables);
        let values: Vec<_> =
            shared::item_scaling::stat_values(&tables, &template.scaling, template.inventory_type)
                .collect();
        println!(
            "PARITY item={id} input={:?} slot={} result={result:?} allocations={values:?}",
            template.scaling, template.inventory_type
        );
        match id {
            1273 => {
                assert_eq!(result.item_level, 11);
                assert_eq!(values, [(7, 4.0), (40, 7.0)]);
                assert_eq!(result.block.primary.stamina, 4.0);
                assert_eq!(result.block.secondary.armor, 10.0);
            }
            6268 => {
                assert_eq!(result.item_level, 6);
                assert_eq!(values, [(73, 1.0), (7, 2.0)]);
                assert_eq!(result.block.primary.agility, 1.0);
                assert_eq!(result.block.primary.intellect, 1.0);
                assert_eq!(result.block.primary.stamina, 2.0);
                assert_eq!(result.block.secondary.armor, 5.0);
            }
            937 => {
                assert_eq!(result.item_level, 12);
                assert!(values.is_empty());
                assert_eq!(result.block, ItemStatBlock::default());
                let weapon = result.weapon.unwrap();
                assert_eq!((weapon.min, weapon.max, weapon.speed), (6.0, 9.0, 3.6));
            }
            // Additional rows used by the existing client tooltip tests.
            _ => {}
        }
    }
}
