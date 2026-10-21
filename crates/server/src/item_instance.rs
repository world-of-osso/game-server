//! Per-GUID equipment modifiers, independent of the legacy inventory and wire ABI.
pub mod bonuses;
pub mod enchants;
pub mod load;
pub mod store;
use serde::{Deserialize, Serialize};
use shared::item_scaling::{ItemScaling, ItemScalingTables, ItemStats};

#[derive(
    Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, bitcode::Encode, bitcode::Decode,
)]
#[serde(default, deny_unknown_fields)]
pub struct ItemInstanceModifiers {
    pub bonus_list: Vec<u32>,
    pub item_level_override: Option<u16>,
    pub enchant: u32,
    pub gems: Vec<u32>,
    pub crafted_stats: Vec<i8>,
}

pub fn item_instance_stats(
    item: &ItemScaling,
    inventory_type: u8,
    instance: &ItemInstanceModifiers,
    tables: &ItemScalingTables,
) -> Result<ItemStats, String> {
    let mut resolved = *item;
    if let Some(level) = instance.item_level_override {
        if level == 0 || !tables.rand_prop_points.contains_key(&level) {
            return Err(format!("item level {level} has no native stat budget"));
        }
        resolved.item_level = level;
        if let Some((era, _)) = &tables.squish {
            resolved.squish_era = *era;
        }
    }
    Ok(shared::item_scaling::item_stats(
        &resolved,
        inventory_type,
        tables,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::item_scaling::ItemStatAllocation;

    fn weapon() -> ItemScaling {
        let mut item = ItemScaling {
            class_id: 2,
            quality: 4,
            item_level: 100,
            squish_era: 2,
            delay_ms: 2000,
            ..Default::default()
        };
        item.stats[0] = ItemStatAllocation {
            stat: 4,
            allocation: 10000,
            socket_multiplier: 0.0,
        };
        item
    }

    #[test]
    fn item_instance_explicit_level_changes_budget_and_weapon_damage_not_catalog() {
        let item = weapon();
        let mut tables = ItemScalingTables::default();
        tables.rand_prop_points.insert(100, [[100.0; 5]; 3]);
        tables.rand_prop_points.insert(200, [[400.0; 5]; 3]);
        tables.damage[0].insert(100, [10.0; 7]);
        tables.damage[0].insert(200, [40.0; 7]);
        let instance = ItemInstanceModifiers {
            item_level_override: Some(200),
            ..Default::default()
        };
        let base = shared::item_scaling::item_stats(&item, 13, &tables);
        let scaled = item_instance_stats(&item, 13, &instance, &tables).unwrap();
        assert_eq!(scaled.item_level, 200);
        assert_eq!(scaled.block.primary.strength, 400.0);
        assert_eq!(scaled.weapon.unwrap().min, base.weapon.unwrap().min * 4.0);
        assert_eq!(item.item_level, 100);
        assert_eq!(base.block.primary.strength, 100.0);
    }

    #[test]
    fn item_instance_profile_crafted_stats_replace_native_placeholders_in_order() {
        let mut item = weapon();
        item.stats[0] = ItemStatAllocation {
            stat: 24,
            allocation: 3500,
            socket_multiplier: 0.0,
        };
        item.stats[1] = ItemStatAllocation {
            stat: 25,
            allocation: 3500,
            socket_multiplier: 0.0,
        };
        let mut tables = ItemScalingTables::default();
        tables.rand_prop_points.insert(100, [[100.0; 5]; 3]);
        let instance = ItemInstanceModifiers {
            crafted_stats: vec![36, 32],
            ..Default::default()
        };
        let stats = item_instance_stats(&item, 13, &instance, &tables).unwrap();
        assert_eq!(stats.block.secondary.haste, 35.0);
        assert_eq!(stats.block.secondary.crit, 35.0);
        // A single authored placeholder (native244774) stays one stat;
        // SimC item.cpp1740 does not split its7000 budget into two stats.
        item.stats[0].allocation = 7000;
        item.stats[1] = ItemStatAllocation::default();
        let stats = item_instance_stats(&item, 13, &instance, &tables).unwrap();
        assert_eq!(stats.block.secondary.haste, 70.0);
        assert_eq!(stats.block.secondary.crit, 0.0);
    }

    #[test]
    fn item_instance_invalid_zero_level_is_not_silent_base_gear() {
        let instance = ItemInstanceModifiers {
            item_level_override: Some(0),
            ..Default::default()
        };
        assert!(
            item_instance_stats(&weapon(), 13, &instance, &ItemScalingTables::default()).is_err()
        );
    }
}
