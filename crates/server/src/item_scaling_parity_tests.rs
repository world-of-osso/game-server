use super::*;
use crate::item_templates::load_item_templates;
use shared::item_data::ItemDefinitionSource;

#[test]
fn local_rows_for_tooltip_parity() {
    let path = crate::test_data::world_db_path();
    let templates = load_item_templates(&path).unwrap();
    let tables = load_item_scaling_tables(&path).unwrap();
    for id in [1273, 6268, 937] {
        let template = templates.get(ItemDefinitionSource::Retail, id).unwrap();
        let result = item_stats(template, &tables);
        let values: Vec<_> =
            shared::item_scaling::stat_values(&tables, &template.scaling, template.inventory_type)
                .collect();
        println!(
            "PARITY item={id} input={:?} slot={} result={result:?} allocations={values:?}",
            template.scaling, template.inventory_type
        );
        if id == 1273 {
            assert_eq!(result.block.primary.stamina, 4.0);
            assert_eq!(result.block.secondary.armor, 10.0);
        }
    }
}
