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
    pub effect_ids: Vec<u32>,
}

pub fn apply_item_bonuses(
    item: &ItemScaling,
    bonus_list: &[u32],
    tables: &ItemBonusTables,
) -> Result<BonusResult, String> {
    let mut result = BonusResult {
        scaling: *item,
        sockets: Vec::new(),
        effect_ids: Vec::new(),
    };
    let mut midnight_scaling = false;
    for bonus in ordered_entries(bonus_list, tables)? {
        match bonus.kind {
            1 if !midnight_scaling => adjust_level(&mut result.scaling, bonus.values[0])?,
            2 => add_allocation(&mut result.scaling, bonus.values)?,
            3 => {
                result.scaling.quality = u8::try_from(bonus.values[0]).map_err(|e| e.to_string())?
            }
            6 => add_sockets(&mut result, bonus.values)?,
            23 => result.effect_ids.push(
                u32::try_from(bonus.values[0])
                    .map_err(|error| format!("ItemEffect ID: {error}"))?,
            ),
            25 => replace_crafted_stat(&mut result.scaling, bonus.values[0])?,
            49 => {
                apply_scaling_config(&mut result.scaling, bonus.values[0], tables)?;
                midnight_scaling = true;
            }
            52 => adjust_level(&mut result.scaling, bonus.values[0])?,
            _ => {}
        }
    }
    Ok(result)
}

fn ordered_entries<'a>(
    ids: &[u32],
    tables: &'a ItemBonusTables,
) -> Result<Vec<&'a ItemBonus>, String> {
    let mut ordered = ids.to_vec();
    for id in &ordered {
        if !tables.entries.contains_key(id) {
            return Err(format!("ItemBonusList {id} has no native rows"));
        }
    }
    // SimC76f8118 sc_item_data.cpp540: crafting/post-squish deltas go last.
    ordered.sort_by_key(|id| {
        let late = tables.entries[id]
            .iter()
            .any(|entry| matches!(entry.kind, 52 | 53));
        (late, *id)
    });
    Ok(ordered
        .into_iter()
        .flat_map(|id| &tables.entries[&id])
        .collect())
}

fn adjust_level(item: &mut ItemScaling, delta: i32) -> Result<(), String> {
    let level = i32::from(item.item_level) + delta;
    item.item_level = u16::try_from(level).map_err(|e| format!("item level {level}: {e}"))?;
    if item.item_level == 0 {
        return Err("item bonus produced item level zero".into());
    }
    Ok(())
}

fn add_allocation(item: &mut ItemScaling, values: [i32; 4]) -> Result<(), String> {
    let stat = i8::try_from(values[0]).map_err(|e| e.to_string())?;
    let existing = item.stats.iter().position(|entry| entry.stat == stat);
    let vacant = item.stats.iter().position(|entry| entry.stat <= 0);
    let index = existing
        .or(vacant)
        .ok_or("item has no free native stat allocation slot")?;
    let entry = &mut item.stats[index];
    entry.stat = stat;
    entry.allocation = entry
        .allocation
        .checked_add(values[1])
        .ok_or("item stat allocation overflow")?;
    Ok(())
}

fn add_sockets(result: &mut BonusResult, values: [i32; 4]) -> Result<(), String> {
    let count = usize::try_from(values[0]).map_err(|e| e.to_string())?;
    let color = u32::try_from(values[1]).map_err(|e| e.to_string())?;
    if result.sockets.len() + count > 3 {
        return Err("ItemBonus exceeds three native gem socket fields".into());
    }
    result.sockets.extend(std::iter::repeat_n(color, count));
    Ok(())
}

fn replace_crafted_stat(item: &mut ItemScaling, value: i32) -> Result<(), String> {
    let stat = i8::try_from(value).map_err(|e| e.to_string())?;
    let Some(old) = item
        .stats
        .iter()
        .find(|entry| matches!(entry.stat, 24 | 25))
        .map(|entry| entry.stat)
    else {
        return Ok(());
    };
    for entry in &mut item.stats {
        if entry.stat == old {
            entry.stat = stat;
        }
    }
    Ok(())
}

fn apply_scaling_config(
    item: &mut ItemScaling,
    id: i32,
    tables: &ItemBonusTables,
) -> Result<(), String> {
    let config = tables
        .scaling_configs
        .get(&(id as u32))
        .ok_or_else(|| format!("ItemScalingConfig {id} absent"))?;
    let offset = tables
        .offset_curves
        .get(&config.offset_curve)
        .ok_or_else(|| format!("ItemOffsetCurve {} absent", config.offset_curve))?;
    let curve_id = if config.squish_era < 2 {
        92181
    } else {
        offset.curve
    };
    let curve = tables
        .curves
        .get(&curve_id)
        .ok_or_else(|| format!("item curve {curve_id} absent"))?;
    let level =
        interpolate_curve(curve, f64::from(config.item_level))?.round() as i32 + offset.offset;
    item.item_level =
        u16::try_from(level).map_err(|e| format!("scaled item level {level}: {e}"))?;
    item.squish_era = 2;
    Ok(())
}

fn interpolate_curve(points: &[(f64, f64)], x: f64) -> Result<f64, String> {
    let first = points.first().ok_or("item scaling curve has no points")?;
    let upper = points.partition_point(|point| point.0 <= x);
    if upper == 0 {
        return Ok(first.1);
    }
    if upper == points.len() {
        return Ok(points[upper - 1].1);
    }
    let (x0, y0) = points[upper - 1];
    let (x1, y1) = points[upper];
    Ok(y0 + (y1 - y0) * (x - x0) / (x1 - x0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::item_scaling::{ItemScalingTables, ItemStatAllocation};

    #[test]
    fn item_instance_native_mid2_bonus_keeps_effect_identity_not_flat_stats() {
        // Native70448 list13771/row31240 adds ItemEffect234561 (spell1297382).
        let tables = ItemBonusTables {
            entries: HashMap::from([(
                13771,
                vec![ItemBonus {
                    kind: 23,
                    values: [234561, 0, 0, 0],
                }],
            )]),
            ..Default::default()
        };
        let item = ItemScaling {
            item_level: 285,
            ..Default::default()
        };
        let resolved = apply_item_bonuses(&item, &[13771], &tables).unwrap();
        assert_eq!(resolved.effect_ids, vec![234561]);
        assert_eq!(resolved.scaling.item_level, 285);
        assert!(
            resolved
                .scaling
                .stats
                .iter()
                .all(|stat| stat.allocation == 0)
        );
    }

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
