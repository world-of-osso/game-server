use std::ops::AddAssign;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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
    Default,
)]
pub struct CharacterAppearance {
    pub sex: u8,
    pub skin_color: u8,
    pub face: u8,
    pub eye_color: u8,
    pub hair_style: u8,
    pub hair_color: u8,
    pub facial_style: u8,
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
)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
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
)]
pub struct Rotation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
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
)]
pub struct Health {
    pub current: f32,
    pub max: f32,
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
)]
pub struct Mana {
    pub current: f32,
    pub max: f32,
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
)]
pub struct MovementSpeed(pub f32);

#[derive(
    Component,
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    PartialEq,
)]
pub struct Player {
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub appearance: CharacterAppearance,
}

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
pub enum EquipmentVisualSlot {
    Head,
    Shoulder,
    Back,
    Chest,
    Shirt,
    Tabard,
    Wrist,
    Hands,
    Waist,
    Legs,
    Feet,
    MainHand,
    OffHand,
}

#[derive(
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
pub struct EquippedAppearanceEntry {
    pub slot: EquipmentVisualSlot,
    pub item_id: Option<u32>,
    pub display_info_id: Option<u32>,
    pub inventory_type: u8,
    pub hidden: bool,
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
    PartialEq,
    Eq,
    Default,
)]
pub struct EquipmentAppearance {
    pub entries: Vec<EquippedAppearanceEntry>,
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
)]
pub struct Npc {
    pub template_id: u32,
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
)]
pub struct Zone {
    pub id: u32,
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
)]
pub struct ModelDisplay {
    pub display_id: u32,
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
    PartialEq,
)]
pub struct GuildMembership {
    pub guild_id: u32,
    pub guild_name: String,
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
}

impl WeaponDamage {
    /// Average damage per swing: `(min + max) / 2`.
    pub fn average_damage(&self) -> f32 {
        (self.min_damage + self.max_damage) / 2.0
    }

    /// Damage per second: `average_damage / speed`.
    pub fn dps(&self) -> f32 {
        if self.speed > 0.0 {
            self.average_damage() / self.speed
        } else {
            0.0
        }
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
        };
        assert_eq!(weapon.average_damage(), 150.0);
    }

    #[test]
    fn weapon_damage_dps() {
        let weapon = WeaponDamage {
            min_damage: 100.0,
            max_damage: 200.0,
            speed: 2.0,
        };
        assert_eq!(weapon.dps(), 75.0);
    }

    #[test]
    fn weapon_damage_dps_zero_speed() {
        let weapon = WeaponDamage {
            min_damage: 100.0,
            max_damage: 200.0,
            speed: 0.0,
        };
        assert_eq!(weapon.dps(), 0.0);
    }

    #[test]
    fn weapon_damage_roundtrip_bitcode() {
        let weapon = WeaponDamage {
            min_damage: 85.0,
            max_damage: 159.0,
            speed: 3.3,
        };
        let encoded = bitcode::encode(&weapon);
        let decoded: WeaponDamage = bitcode::decode(&encoded).unwrap();
        assert_eq!(weapon, decoded);
    }
}
