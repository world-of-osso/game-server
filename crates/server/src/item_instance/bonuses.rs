//! Native ItemBonus application before the existing item stat formula.
use shared::item_scaling::ItemScaling;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct ItemBonus {
    pub kind: u32,
    pub values: [i32; 4],
}

#[derive(Default)]
pub struct ItemBonusTables {
    pub entries: HashMap<u32, Vec<ItemBonus>>,
}

pub struct BonusResult {
    pub scaling: ItemScaling,
    pub sockets: Vec<u32>,
}

pub fn apply_item_bonuses(
    item: &ItemScaling,
    bonus_list: &[u32],
    tables: &ItemBonusTables,
) -> Result<BonusResult, String> {
    let _ = (bonus_list, tables);
    Ok(BonusResult {
        scaling: *item,
        sockets: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::item_scaling::{ItemScalingTables, ItemStatAllocation};

    #[test]
    fn item_instance_native_ilevel_and_crit_allocation_bonuses_change_real_stats() {
        // Native70448 ItemBonus44/list15 = +4ilevel;45/list17 = crit allocation3506.
        let native = ItemBonusTables {
            entries: HashMap::from([
                (
                    15,
                    vec![ItemBonus {
                        kind: 1,
                        values: [4, 0, 0, 0],
                    }],
                ),
                (
                    17,
                    vec![ItemBonus {
                        kind: 2,
                        values: [32, 3506, 0, 0],
                    }],
                ),
            ]),
        };
        let item = ItemScaling {
            class_id: 4,
            subclass_id: 4,
            quality: 4,
            item_level: 100,
            squish_era: 2,
            ..Default::default()
        };
        let modified = apply_item_bonuses(&item, &[15, 17], &native).unwrap();
        let mut tables = ItemScalingTables::default();
        tables.rand_prop_points.insert(104, [[100.0; 5]; 3]);
        let stats = shared::item_scaling::item_stats(&modified.scaling, 1, &tables);
        assert_eq!(stats.item_level, 104);
        assert_eq!(stats.block.secondary.crit, 35.0);
        assert_eq!(item.item_level, 100);
    }

    #[test]
    fn item_instance_native_crafted_stat_swap_and_socket_are_retained() {
        // Native12782/list6647 chooses crit;31197/list13750 adds socket type7.
        let native = ItemBonusTables {
            entries: HashMap::from([
                (
                    6647,
                    vec![ItemBonus {
                        kind: 25,
                        values: [32, 100, 0, 0],
                    }],
                ),
                (
                    13750,
                    vec![ItemBonus {
                        kind: 6,
                        values: [1, 7, 0, 0],
                    }],
                ),
            ]),
        };
        let mut item = ItemScaling::default();
        item.stats[0] = ItemStatAllocation {
            stat: 24,
            allocation: 3500,
            socket_multiplier: 0.0,
        };
        let modified = apply_item_bonuses(&item, &[6647, 13750], &native).unwrap();
        assert_eq!(modified.scaling.stats[0].stat, 32);
        assert_eq!(modified.scaling.stats[0].allocation, 3500);
        assert_eq!(modified.sockets, vec![7]);
        assert_eq!(item.stats[0].stat, 24);
    }

    #[test]
    fn item_instance_unknown_bonus_does_not_silently_become_base_gear() {
        let result = apply_item_bonuses(
            &ItemScaling::default(),
            &[987654321],
            &ItemBonusTables::default(),
        );
        assert!(result.is_err());
    }
}
