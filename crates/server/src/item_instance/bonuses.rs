//! Native ItemBonus application before the existing item stat formula.
use shared::item_scaling::ItemScaling;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct ItemBonus {
    pub kind: u32,
    pub values: [i32; 4],
}

#[derive(Clone, Copy)]
pub struct ScalingConfig {
    pub offset_curve: u32,
    pub item_level: u16,
    pub squish_era: u8,
}

#[derive(Clone, Copy)]
pub struct OffsetCurve {
    pub curve: u32,
    pub offset: i32,
}

#[derive(Default)]
pub struct ItemBonusTables {
    pub entries: HashMap<u32, Vec<ItemBonus>>,
    pub scaling_configs: HashMap<u32, ScalingConfig>,
    pub offset_curves: HashMap<u32, OffsetCurve>,
    pub curves: HashMap<u32, Vec<(f64, f64)>>,
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
            ..Default::default()
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
            ..Default::default()
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
    fn item_instance_midnight_config_precedes_crafting_delta_regardless_of_input_order() {
        // Actual MID1 crafted bracers: native config271/curve88583 gives272;
        // quality bonus9627 adds13 to reach285 after all config replacements.
        let native = ItemBonusTables {
            entries: HashMap::from([
                (
                    12066,
                    vec![ItemBonus {
                        kind: 49,
                        values: [273, 70, 0, 0],
                    }],
                ),
                (
                    12214,
                    vec![ItemBonus {
                        kind: 49,
                        values: [264, 100, 0, 0],
                    }],
                ),
                (
                    13622,
                    vec![ItemBonus {
                        kind: 49,
                        values: [271, 65, 0, 0],
                    }],
                ),
                (
                    9627,
                    vec![ItemBonus {
                        kind: 52,
                        values: [13, 2, 4, 0],
                    }],
                ),
            ]),
            scaling_configs: HashMap::from([
                (
                    273,
                    ScalingConfig {
                        offset_curve: 47,
                        item_level: 246,
                        squish_era: 2,
                    },
                ),
                (
                    264,
                    ScalingConfig {
                        offset_curve: 47,
                        item_level: 246,
                        squish_era: 2,
                    },
                ),
                (
                    271,
                    ScalingConfig {
                        offset_curve: 47,
                        item_level: 272,
                        squish_era: 2,
                    },
                ),
            ]),
            offset_curves: HashMap::from([(
                47,
                OffsetCurve {
                    curve: 88583,
                    offset: 0,
                },
            )]),
            curves: HashMap::from([(88583, vec![(0.0, 0.0), (1300.0, 1300.0)])]),
        };
        let item = ItemScaling {
            item_level: 197,
            ..Default::default()
        };
        for bonuses in [
            vec![9627, 13622, 12214, 12066, 12214],
            vec![12066, 12214, 12214, 13622, 9627],
        ] {
            let result = apply_item_bonuses(&item, &bonuses, &native).unwrap();
            assert_eq!(result.scaling.item_level, 285);
            assert_eq!(result.scaling.squish_era, 2, "no second legacy squish");
        }
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
