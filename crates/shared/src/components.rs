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
}
