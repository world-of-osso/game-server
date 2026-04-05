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

/// A single buff or debuff active on a unit.
///
/// Auras are stored as a `Vec<Aura>` inside the `Auras` component on each
/// entity. Each aura tracks its source spell, caster, remaining duration,
/// and stack count.
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
)]
pub struct Aura {
    /// Spell ID that created this aura.
    pub spell_id: u32,
    /// Entity bits of the caster (for same-caster stacking rules).
    pub caster: u64,
    /// Total duration in seconds (for refresh logic).
    pub duration: f32,
    /// Time remaining in seconds. Ticks down each frame.
    pub remaining: f32,
    /// Current stack count.
    pub stacks: u8,
    /// Maximum stacks allowed (1 = non-stacking).
    pub max_stacks: u8,
}

/// Collection of active auras on a unit.
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
    Default,
)]
pub struct Auras {
    pub active: Vec<Aura>,
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
        if self.speed > 0.0 {
            self.average_damage() / self.speed
        } else {
            0.0
        }
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
    fn aura_roundtrip_bitcode() {
        let aura = Aura {
            spell_id: 12345,
            caster: 99,
            duration: 30.0,
            remaining: 25.5,
            stacks: 2,
            max_stacks: 5,
        };
        let encoded = bitcode::encode(&aura);
        let decoded: Aura = bitcode::decode(&encoded).unwrap();
        assert_eq!(aura, decoded);
    }

    #[test]
    fn auras_default_is_empty() {
        let auras = Auras::default();
        assert!(auras.active.is_empty());
    }

    #[test]
    fn auras_collection_roundtrip_bitcode() {
        let auras = Auras {
            active: vec![
                Aura {
                    spell_id: 1,
                    caster: 10,
                    duration: 10.0,
                    remaining: 8.0,
                    stacks: 1,
                    max_stacks: 1,
                },
                Aura {
                    spell_id: 2,
                    caster: 20,
                    duration: 60.0,
                    remaining: 45.0,
                    stacks: 3,
                    max_stacks: 5,
                },
            ],
        };
        let encoded = bitcode::encode(&auras);
        let decoded: Auras = bitcode::decode(&encoded).unwrap();
        assert_eq!(auras, decoded);
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
