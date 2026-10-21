//! Native permanent-enchant and gem contributions; spell effects retain their IDs.
use shared::components::ItemStatBlock;
use std::collections::HashMap;

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
    _id: u32,
    _level: u8,
    _tables: &EnchantmentTables,
) -> Result<EnchantContribution, String> {
    Ok(EnchantContribution::default())
}

pub fn gem_stats(
    _item: u32,
    _level: u8,
    _tables: &EnchantmentTables,
) -> Result<EnchantContribution, String> {
    Ok(EnchantContribution::default())
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
    fn item_instance_unknown_gem_and_enchant_are_explicit_errors() {
        assert!(gem_stats(999999, 90, &native()).is_err());
        assert!(enchantment_stats(999999, 90, &native()).is_err());
    }
}
