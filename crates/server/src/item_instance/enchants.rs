//! Native permanent-enchant and gem contributions; spell effects retain their IDs.
use shared::components::ItemStatBlock;
use std::collections::HashMap;

#[path = "../spell_scaling_data.rs"]
mod scaling_data;

#[derive(Clone, Copy, Default)]
pub struct EnchantEffect {
    pub kind: u32,
    pub points: i32,
    pub argument: i32,
    pub coefficient: f32,
}

#[derive(Clone, Copy, Default)]
pub struct Enchantment {
    pub effects: [EnchantEffect; 3],
    pub scaling_class: i32,
    pub min_level: u8,
    pub max_level: u8,
}

#[derive(Default)]
pub struct EnchantmentTables {
    pub enchants: HashMap<u32, Enchantment>,
    pub gems: HashMap<u32, u32>,
}

#[derive(Default)]
pub struct EnchantContribution {
    pub stats: ItemStatBlock,
    pub equip_spells: Vec<u32>,
}

pub fn enchantment_stats(
    id: u32,
    level: u8,
    tables: &EnchantmentTables,
) -> Result<EnchantContribution, String> {
    let mut result = EnchantContribution::default();
    if id == 0 {
        return Ok(result);
    }
    let enchant = tables
        .enchants
        .get(&id)
        .ok_or_else(|| format!("native enchantment {id} absent"))?;
    for effect in &enchant.effects {
        match effect.kind {
            0 => {}
            3 => result
                .equip_spells
                .push(u32::try_from(effect.argument).map_err(|e| e.to_string())?),
            // TC a352b1fa Player.cpp12430–12447: physical-school resistance is armor.
            4 if effect.argument == 0 => {
                result.stats.secondary.armor +=
                    scaled_amount(enchant, effect, level)?.trunc().max(1.0);
            }
            5 => {
                let amount = scaled_amount(enchant, effect, level)?.round();
                add_stat(&mut result.stats, effect.argument, amount)?;
            }
            kind => {
                return Err(format!(
                    "enchantment {id} effect type {kind} is not modeled"
                ));
            }
        }
    }
    Ok(result)
}

pub fn gem_stats(
    item: u32,
    level: u8,
    tables: &EnchantmentTables,
) -> Result<EnchantContribution, String> {
    let enchant = tables
        .gems
        .get(&item)
        .ok_or_else(|| format!("native gem item {item} has no GemProperties enchant"))?;
    enchantment_stats(*enchant, level, tables)
}

fn scaled_amount(enchant: &Enchantment, effect: &EnchantEffect, level: u8) -> Result<f32, String> {
    if enchant.scaling_class == 0 {
        return Ok(effect.points as f32);
    }
    let level = level.max(enchant.min_level);
    let level = if enchant.max_level == 0 {
        level
    } else {
        level.min(enchant.max_level)
    };
    let column = scaling_column(enchant.scaling_class)?;
    let index = usize::from(level)
        .checked_sub(1)
        .ok_or("enchantment level zero")?;
    let row = scaling_data::SPELL_SCALING
        .get(index)
        .ok_or_else(|| format!("enchantment scaling level {level} absent"))?;
    Ok(row[column] * effect.coefficient)
}

fn scaling_column(class: i32) -> Result<usize, String> {
    match class {
        -1 | -7 => Ok(0),
        -2 => Ok(1),
        -3 => Ok(2),
        -4 => Ok(3),
        -5 => Ok(4),
        -6 => Ok(5),
        -8 => Ok(6),
        -9 => Ok(7),
        -10 => Ok(8),
        _ => Err(format!("unmodeled enchantment scaling class {class}")),
    }
}

fn add_stat(block: &mut ItemStatBlock, stat: i32, value: f32) -> Result<(), String> {
    match stat {
        3 | 4 | 5 | 7 | 71..=74 => add_primary(block, stat, value),
        32 => block.secondary.crit += value,
        36 => block.secondary.haste += value,
        40 => block.secondary.versatility += value,
        49 => block.secondary.mastery += value,
        61 => block.secondary.speed += value,
        62 => block.secondary.leech += value,
        63 => block.secondary.avoidance += value,
        _ => return Err(format!("unmodeled enchantment item stat {stat}")),
    }
    Ok(())
}

fn add_primary(block: &mut ItemStatBlock, stat: i32, value: f32) {
    if matches!(stat, 4 | 71 | 72 | 74) {
        block.primary.strength += value;
    }
    if matches!(stat, 3 | 71 | 72 | 73) {
        block.primary.agility += value;
    }
    if matches!(stat, 5 | 71 | 73 | 74) {
        block.primary.intellect += value;
    }
    if stat == 7 {
        block.primary.stamina += value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native() -> EnchantmentTables {
        let primary = Enchantment {
            scaling_class: -7,
            min_level: 0,
            max_level: 80,
            effects: [
                EnchantEffect {
                    kind: 5,
                    argument: 71,
                    coefficient: 1.48670006,
                    points: 0,
                },
                EnchantEffect::default(),
                EnchantEffect::default(),
            ],
        };
        let gem = Enchantment {
            scaling_class: -7,
            min_level: 0,
            max_level: 80,
            effects: [
                EnchantEffect {
                    kind: 5,
                    argument: 32,
                    coefficient: 0.475744992,
                    points: 0,
                },
                EnchantEffect {
                    kind: 5,
                    argument: 40,
                    coefficient: 0.208138004,
                    points: 0,
                },
                EnchantEffect::default(),
            ],
        };
        let proc = Enchantment {
            effects: [
                EnchantEffect {
                    kind: 3,
                    argument: 1236742,
                    ..Default::default()
                },
                EnchantEffect::default(),
                EnchantEffect::default(),
            ],
            ..Default::default()
        };
        EnchantmentTables {
            enchants: HashMap::from([(7987, primary), (8157, gem), (8039, proc)]),
            gems: HashMap::from([(240910, 8157)]),
        }
    }

    #[test]
    fn item_instance_native_worldsoul_enchant_caps_level_and_keeps_hybrid_primaries() {
        let contribution = enchantment_stats(7987, 90, &native()).unwrap();
        // Native MaxLevel80, gtSpellScaling item33.63146077 *1.48670006 rounds50.
        assert_eq!(contribution.stats.primary.strength, 50.0);
        assert_eq!(contribution.stats.primary.agility, 50.0);
        assert_eq!(contribution.stats.primary.intellect, 50.0);
    }

    #[test]
    fn item_instance_native_gem_property_resolves_both_secondary_ratings() {
        let contribution = gem_stats(240910, 90, &native()).unwrap();
        assert_eq!(contribution.stats.secondary.crit, 16.0);
        assert_eq!(contribution.stats.secondary.versatility, 7.0);
    }

    #[test]
    fn item_instance_native_weapon_enchant_retains_real_spell_not_fake_flat_damage() {
        let contribution = enchantment_stats(8039, 90, &native()).unwrap();
        assert_eq!(contribution.equip_spells, vec![1236742]);
        assert_eq!(contribution.stats, ItemStatBlock::default());
    }

    #[test]
    fn item_instance_native_armor_enchant_keeps_primary_and_physical_resistance() {
        let mut tables = native();
        // Native70448 SpellItemEnchantment8163: type4/school0 and type5/hybrid72.
        tables.enchants.insert(
            8163,
            Enchantment {
                effects: [
                    EnchantEffect {
                        kind: 4,
                        argument: 0,
                        coefficient: 0.802820027,
                        points: 0,
                    },
                    EnchantEffect {
                        kind: 5,
                        argument: 72,
                        coefficient: 1.2191,
                        points: 0,
                    },
                    EnchantEffect::default(),
                ],
                scaling_class: -7,
                min_level: 0,
                max_level: 80,
            },
        );
        let stats = enchantment_stats(8163, 90, &tables).unwrap().stats;
        assert_eq!(stats.secondary.armor, 27.0);
        assert_eq!(stats.primary.strength, 41.0);
        assert_eq!(stats.primary.agility, 41.0);
        assert_eq!(stats.primary.intellect, 0.0);
    }

    #[test]
    fn item_instance_native_armor_enchant_truncates_and_preserves_unscaled_amounts() {
        let mut tables = native();
        // Native70448 entries18/85, not fabricated scaling coefficients.
        for (id, coefficient, scaling_class, max_level, points, expected) in
            [(18, 0.129999995, -1, 25, 32, 1.0), (85, 0.0, 0, 0, 3, 3.0)]
        {
            tables.enchants.insert(
                id,
                Enchantment {
                    effects: [
                        EnchantEffect {
                            kind: 4,
                            argument: 0,
                            coefficient,
                            points,
                        },
                        EnchantEffect::default(),
                        EnchantEffect::default(),
                    ],
                    scaling_class,
                    min_level: 0,
                    max_level,
                },
            );
            let stats = enchantment_stats(id, 90, &tables).unwrap().stats;
            assert_eq!(stats.secondary.armor, expected);
        }
    }

    #[test]
    fn item_instance_unknown_gem_and_enchant_are_explicit_errors() {
        assert!(gem_stats(999999, 90, &native()).is_err());
        assert!(enchantment_stats(999999, 90, &native()).is_err());
    }
}
