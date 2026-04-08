use std::ops::AddAssign;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Base unit attributes shared by players and creatures.
#[derive(
    Component,
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Default,
)]
pub struct UnitStats {
    pub stamina: f32,
    pub strength: f32,
    pub agility: f32,
    pub intellect: f32,
    pub spirit: f32,
}

impl AddAssign for UnitStats {
    fn add_assign(&mut self, rhs: Self) {
        self.stamina += rhs.stamina;
        self.strength += rhs.strength;
        self.agility += rhs.agility;
        self.intellect += rhs.intellect;
        self.spirit += rhs.spirit;
    }
}

/// Derived combat ratings from gear, buffs, and progression.
#[derive(
    Component,
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Default,
)]
pub struct CombatRatings {
    pub crit: f32,
    pub haste: f32,
    pub mastery: f32,
    pub versatility: f32,
    pub armor: f32,
    pub dodge: f32,
    pub parry: f32,
    pub block: f32,
}

impl AddAssign for CombatRatings {
    fn add_assign(&mut self, rhs: Self) {
        self.crit += rhs.crit;
        self.haste += rhs.haste;
        self.mastery += rhs.mastery;
        self.versatility += rhs.versatility;
        self.armor += rhs.armor;
        self.dodge += rhs.dodge;
        self.parry += rhs.parry;
        self.block += rhs.block;
    }
}

#[derive(
    Component,
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub struct Level(pub u8);

/// Stat contribution from a single equipped item.
#[derive(
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Default,
)]
pub struct ItemStatBlock {
    pub primary: UnitStats,
    pub secondary: CombatRatings,
}

/// Spell school for damage/healing classification.
///
/// In retail WoW, spell resistance per school is removed. Physical damage
/// is mitigated by armor; magic schools have no resistance stat (only
/// versatility provides damage reduction).
#[derive(
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
)]
pub enum SpellSchool {
    Physical,
    Holy,
    Fire,
    Nature,
    Frost,
    Shadow,
    Arcane,
}

impl SpellSchool {
    /// Whether this school's damage is reduced by armor.
    pub fn is_physical(self) -> bool {
        self == Self::Physical
    }
}

/// Weapon category for normalized speed lookup.
///
/// WoW abilities use a fixed "normalized" speed instead of the actual weapon
/// speed to prevent slow weapons from being disproportionately better for
/// instant attacks. Ref: SimC `weapon_t`, Blizzard normalization rules.
#[derive(
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum WeaponType {
    Dagger,
    OneHand,
    TwoHand,
    Ranged,
}

impl WeaponType {
    /// Normalized weapon speed in seconds, used by ability damage formulas.
    pub fn normalized_speed(self) -> f32 {
        match self {
            Self::Dagger => 1.7,
            Self::OneHand => 2.4,
            Self::TwoHand => 3.3,
            Self::Ranged => 2.8,
        }
    }
}

/// Convert average swing damage and weapon speed into DPS.
pub fn weapon_damage_per_second(average_damage: f32, speed: f32) -> f32 {
    if speed > 0.0 {
        average_damage / speed
    } else {
        0.0
    }
}

/// Weapon damage range and speed for an equipped weapon.
///
/// `min_damage` / `max_damage` are the base weapon damage values from item data.
/// `speed` is the weapon swing interval in seconds (e.g. 2.6 for a 2H sword).
#[derive(
    Component,
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    Copy,
    PartialEq,
)]
pub struct WeaponDamage {
    pub min_damage: f32,
    pub max_damage: f32,
    pub speed: f32,
    pub weapon_type: WeaponType,
}

impl WeaponDamage {
    /// Average damage per swing: `(min + max) / 2`.
    pub fn average_damage(&self) -> f32 {
        (self.min_damage + self.max_damage) / 2.0
    }

    /// Damage per second: `average_damage / speed`.
    pub fn dps(&self) -> f32 {
        weapon_damage_per_second(self.average_damage(), self.speed)
    }

    /// Normalized speed for ability damage formulas.
    pub fn normalized_speed(&self) -> f32 {
        self.weapon_type.normalized_speed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_stats_default_is_zero() {
        let stats = UnitStats::default();
        assert_eq!(stats.stamina, 0.0);
        assert_eq!(stats.strength, 0.0);
        assert_eq!(stats.agility, 0.0);
        assert_eq!(stats.intellect, 0.0);
        assert_eq!(stats.spirit, 0.0);
    }

    #[test]
    fn unit_stats_roundtrip_bitcode() {
        let stats = UnitStats {
            stamina: 150.0,
            strength: 200.0,
            agility: 80.0,
            intellect: 50.0,
            spirit: 60.0,
        };
        let encoded = bitcode::encode(&stats);
        let decoded: UnitStats = bitcode::decode(&encoded).unwrap();
        assert_eq!(stats, decoded);
    }

    #[test]
    fn combat_ratings_default_is_zero() {
        let ratings = CombatRatings::default();
        assert_eq!(ratings.crit, 0.0);
        assert_eq!(ratings.haste, 0.0);
        assert_eq!(ratings.mastery, 0.0);
        assert_eq!(ratings.versatility, 0.0);
        assert_eq!(ratings.armor, 0.0);
        assert_eq!(ratings.dodge, 0.0);
        assert_eq!(ratings.parry, 0.0);
        assert_eq!(ratings.block, 0.0);
    }

    #[test]
    fn combat_ratings_roundtrip_bitcode() {
        let ratings = CombatRatings {
            crit: 25.0,
            haste: 18.5,
            mastery: 30.0,
            versatility: 12.0,
            armor: 1500.0,
            dodge: 5.0,
            parry: 8.0,
            block: 10.0,
        };
        let encoded = bitcode::encode(&ratings);
        let decoded: CombatRatings = bitcode::decode(&encoded).unwrap();
        assert_eq!(ratings, decoded);
    }

    #[test]
    fn spell_school_physical_check() {
        assert!(SpellSchool::Physical.is_physical());
        assert!(!SpellSchool::Fire.is_physical());
        assert!(!SpellSchool::Holy.is_physical());
        assert!(!SpellSchool::Shadow.is_physical());
    }

    #[test]
    fn level_roundtrip_bitcode() {
        let level = Level(70);
        let encoded = bitcode::encode(&level);
        let decoded: Level = bitcode::decode(&encoded).unwrap();
        assert_eq!(level, decoded);
    }

    #[test]
    fn weapon_damage_average() {
        let weapon = WeaponDamage {
            min_damage: 100.0,
            max_damage: 200.0,
            speed: 2.6,
            weapon_type: WeaponType::TwoHand,
        };
        assert_eq!(weapon.average_damage(), 150.0);
    }

    #[test]
    fn weapon_damage_dps() {
        let weapon = WeaponDamage {
            min_damage: 100.0,
            max_damage: 200.0,
            speed: 2.0,
            weapon_type: WeaponType::OneHand,
        };
        assert_eq!(weapon.dps(), 75.0);
    }

    #[test]
    fn weapon_damage_dps_zero_speed() {
        let weapon = WeaponDamage {
            min_damage: 100.0,
            max_damage: 200.0,
            speed: 0.0,
            weapon_type: WeaponType::OneHand,
        };
        assert_eq!(weapon.dps(), 0.0);
    }

    #[test]
    fn weapon_damage_roundtrip_bitcode() {
        let weapon = WeaponDamage {
            min_damage: 85.0,
            max_damage: 159.0,
            speed: 3.3,
            weapon_type: WeaponType::TwoHand,
        };
        let encoded = bitcode::encode(&weapon);
        let decoded: WeaponDamage = bitcode::decode(&encoded).unwrap();
        assert_eq!(weapon, decoded);
    }

    #[test]
    fn normalized_speed_values() {
        assert_eq!(WeaponType::Dagger.normalized_speed(), 1.7);
        assert_eq!(WeaponType::OneHand.normalized_speed(), 2.4);
        assert_eq!(WeaponType::TwoHand.normalized_speed(), 3.3);
        assert_eq!(WeaponType::Ranged.normalized_speed(), 2.8);
    }

    #[test]
    fn weapon_normalized_speed_delegates() {
        let weapon = WeaponDamage {
            min_damage: 50.0,
            max_damage: 100.0,
            speed: 1.5,
            weapon_type: WeaponType::Dagger,
        };
        assert_eq!(weapon.normalized_speed(), 1.7);
    }
}
