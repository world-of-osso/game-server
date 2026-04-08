use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::aura::{Aura, AuraEffect};

/// Player class, matching AzerothCore class IDs.
///
/// IDs follow WoW's `ChrClasses.dbc`: 1=Warrior through 13=Evoker.
/// Note: 10 is unused (was Monk placeholder in some expansions).
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
#[repr(u8)]
pub enum Class {
    Warrior = 1,
    Paladin = 2,
    Hunter = 3,
    Rogue = 4,
    Priest = 5,
    DeathKnight = 6,
    Shaman = 7,
    Mage = 8,
    Warlock = 9,
    Monk = 10,
    Druid = 11,
    DemonHunter = 12,
    Evoker = 13,
}

impl Class {
    /// Convert from a raw u8 class ID (as stored in DB / network protocol).
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(Self::Warrior),
            2 => Some(Self::Paladin),
            3 => Some(Self::Hunter),
            4 => Some(Self::Rogue),
            5 => Some(Self::Priest),
            6 => Some(Self::DeathKnight),
            7 => Some(Self::Shaman),
            8 => Some(Self::Mage),
            9 => Some(Self::Warlock),
            10 => Some(Self::Monk),
            11 => Some(Self::Druid),
            12 => Some(Self::DemonHunter),
            13 => Some(Self::Evoker),
            _ => None,
        }
    }

    /// The raw u8 class ID.
    pub fn id(self) -> u8 {
        self as u8
    }

    /// Whether this class uses mana as a primary resource.
    pub fn uses_mana(self) -> bool {
        matches!(
            self,
            Self::Paladin
                | Self::Hunter
                | Self::Priest
                | Self::Shaman
                | Self::Mage
                | Self::Warlock
                | Self::Monk
                | Self::Druid
                | Self::Evoker
        )
    }

    /// The primary resource type for this class.
    pub fn primary_resource(self) -> crate::spell_data::ResourceType {
        use crate::spell_data::ResourceType;
        match self {
            Self::Warrior => ResourceType::Rage,
            Self::Rogue => ResourceType::Energy,
            Self::DeathKnight => ResourceType::RunicPower,
            Self::Hunter => ResourceType::Focus,
            Self::DemonHunter => ResourceType::Energy,
            _ => ResourceType::Mana,
        }
    }
}

/// Combat role a specialization fills.
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
pub enum Role {
    Tank,
    Healer,
    MeleeDps,
    RangedDps,
}

/// Player specialization. Each class has 2–4 specs that determine abilities,
/// passive bonuses, and role (tank/healer/DPS).
///
/// IDs match WoW's `ChrSpecialization.dbc` where practical.
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
pub struct Specialization {
    pub id: u32,
    pub name: &'static str,
    pub class: Class,
    pub role: Role,
}

/// All specializations, indexed by spec ID.
/// Ref: WoW `ChrSpecialization.dbc` IDs.
pub const SPECIALIZATIONS: &[Specialization] = &[
    // Warrior
    Specialization {
        id: 71,
        name: "Arms",
        class: Class::Warrior,
        role: Role::MeleeDps,
    },
    Specialization {
        id: 72,
        name: "Fury",
        class: Class::Warrior,
        role: Role::MeleeDps,
    },
    Specialization {
        id: 73,
        name: "Protection",
        class: Class::Warrior,
        role: Role::Tank,
    },
    // Paladin
    Specialization {
        id: 65,
        name: "Holy",
        class: Class::Paladin,
        role: Role::Healer,
    },
    Specialization {
        id: 66,
        name: "Protection",
        class: Class::Paladin,
        role: Role::Tank,
    },
    Specialization {
        id: 70,
        name: "Retribution",
        class: Class::Paladin,
        role: Role::MeleeDps,
    },
    // Hunter
    Specialization {
        id: 253,
        name: "Beast Mastery",
        class: Class::Hunter,
        role: Role::RangedDps,
    },
    Specialization {
        id: 254,
        name: "Marksmanship",
        class: Class::Hunter,
        role: Role::RangedDps,
    },
    Specialization {
        id: 255,
        name: "Survival",
        class: Class::Hunter,
        role: Role::MeleeDps,
    },
    // Rogue
    Specialization {
        id: 259,
        name: "Assassination",
        class: Class::Rogue,
        role: Role::MeleeDps,
    },
    Specialization {
        id: 260,
        name: "Outlaw",
        class: Class::Rogue,
        role: Role::MeleeDps,
    },
    Specialization {
        id: 261,
        name: "Subtlety",
        class: Class::Rogue,
        role: Role::MeleeDps,
    },
    // Priest
    Specialization {
        id: 256,
        name: "Discipline",
        class: Class::Priest,
        role: Role::Healer,
    },
    Specialization {
        id: 257,
        name: "Holy",
        class: Class::Priest,
        role: Role::Healer,
    },
    Specialization {
        id: 258,
        name: "Shadow",
        class: Class::Priest,
        role: Role::RangedDps,
    },
    // Death Knight
    Specialization {
        id: 250,
        name: "Blood",
        class: Class::DeathKnight,
        role: Role::Tank,
    },
    Specialization {
        id: 251,
        name: "Frost",
        class: Class::DeathKnight,
        role: Role::MeleeDps,
    },
    Specialization {
        id: 252,
        name: "Unholy",
        class: Class::DeathKnight,
        role: Role::MeleeDps,
    },
    // Shaman
    Specialization {
        id: 262,
        name: "Elemental",
        class: Class::Shaman,
        role: Role::RangedDps,
    },
    Specialization {
        id: 263,
        name: "Enhancement",
        class: Class::Shaman,
        role: Role::MeleeDps,
    },
    Specialization {
        id: 264,
        name: "Restoration",
        class: Class::Shaman,
        role: Role::Healer,
    },
    // Mage
    Specialization {
        id: 62,
        name: "Arcane",
        class: Class::Mage,
        role: Role::RangedDps,
    },
    Specialization {
        id: 63,
        name: "Fire",
        class: Class::Mage,
        role: Role::RangedDps,
    },
    Specialization {
        id: 64,
        name: "Frost",
        class: Class::Mage,
        role: Role::RangedDps,
    },
    // Warlock
    Specialization {
        id: 265,
        name: "Affliction",
        class: Class::Warlock,
        role: Role::RangedDps,
    },
    Specialization {
        id: 266,
        name: "Demonology",
        class: Class::Warlock,
        role: Role::RangedDps,
    },
    Specialization {
        id: 267,
        name: "Destruction",
        class: Class::Warlock,
        role: Role::RangedDps,
    },
    // Monk
    Specialization {
        id: 268,
        name: "Brewmaster",
        class: Class::Monk,
        role: Role::Tank,
    },
    Specialization {
        id: 270,
        name: "Mistweaver",
        class: Class::Monk,
        role: Role::Healer,
    },
    Specialization {
        id: 269,
        name: "Windwalker",
        class: Class::Monk,
        role: Role::MeleeDps,
    },
    // Druid
    Specialization {
        id: 102,
        name: "Balance",
        class: Class::Druid,
        role: Role::RangedDps,
    },
    Specialization {
        id: 103,
        name: "Feral",
        class: Class::Druid,
        role: Role::MeleeDps,
    },
    Specialization {
        id: 104,
        name: "Guardian",
        class: Class::Druid,
        role: Role::Tank,
    },
    Specialization {
        id: 105,
        name: "Restoration",
        class: Class::Druid,
        role: Role::Healer,
    },
    // Demon Hunter
    Specialization {
        id: 577,
        name: "Havoc",
        class: Class::DemonHunter,
        role: Role::MeleeDps,
    },
    Specialization {
        id: 581,
        name: "Vengeance",
        class: Class::DemonHunter,
        role: Role::Tank,
    },
    // Evoker
    Specialization {
        id: 1467,
        name: "Devastation",
        class: Class::Evoker,
        role: Role::RangedDps,
    },
    Specialization {
        id: 1468,
        name: "Preservation",
        class: Class::Evoker,
        role: Role::Healer,
    },
    Specialization {
        id: 1473,
        name: "Augmentation",
        class: Class::Evoker,
        role: Role::RangedDps,
    },
];

/// Look up a specialization by its ID.
pub fn spec_by_id(id: u32) -> Option<&'static Specialization> {
    SPECIALIZATIONS.iter().find(|s| s.id == id)
}

/// Get all specializations for a given class.
pub fn specs_for_class(class: Class) -> Vec<&'static Specialization> {
    SPECIALIZATIONS
        .iter()
        .filter(|s| s.class == class)
        .collect()
}

// --- Mastery system ---

/// Per-spec mastery definition: a description plus a coefficient that converts
/// mastery points (from rating) into an actual effect percentage.
///
/// Example: Arms Warrior mastery "Striking" has coefficient 2.0, so 8 mastery
/// points → 16% bonus damage to Mortal Strike/Colossus Smash.
///
/// Ref: SimC `player_t::composite_mastery_value()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MasteryDef {
    pub spec_id: u32,
    pub name: &'static str,
    /// Multiplier applied to mastery points to get the effect percentage.
    /// e.g. 2.0 means 1 mastery point = 2% bonus.
    pub coefficient: f32,
}

/// Per-spec mastery coefficients. Each spec has a unique mastery bonus.
/// Coefficients are from SimC / WoW game data.
pub const MASTERY_DEFS: &[MasteryDef] = &[
    // Warrior
    MasteryDef {
        spec_id: 71,
        name: "Striking",
        coefficient: 2.00,
    },
    MasteryDef {
        spec_id: 72,
        name: "Unshackled Fury",
        coefficient: 1.40,
    },
    MasteryDef {
        spec_id: 73,
        name: "Critical Block",
        coefficient: 1.50,
    },
    // Paladin
    MasteryDef {
        spec_id: 65,
        name: "Lightbringer",
        coefficient: 1.50,
    },
    MasteryDef {
        spec_id: 66,
        name: "Divine Bulwark",
        coefficient: 1.00,
    },
    MasteryDef {
        spec_id: 70,
        name: "Hand of Light",
        coefficient: 1.75,
    },
    // Hunter
    MasteryDef {
        spec_id: 253,
        name: "Master of Beasts",
        coefficient: 1.80,
    },
    MasteryDef {
        spec_id: 254,
        name: "Sniper Training",
        coefficient: 0.63,
    },
    MasteryDef {
        spec_id: 255,
        name: "Spirit Bond",
        coefficient: 1.00,
    },
    // Rogue
    MasteryDef {
        spec_id: 259,
        name: "Potent Assassin",
        coefficient: 1.80,
    },
    MasteryDef {
        spec_id: 260,
        name: "Main Gauche",
        coefficient: 2.00,
    },
    MasteryDef {
        spec_id: 261,
        name: "Executioner",
        coefficient: 2.20,
    },
    // Priest
    MasteryDef {
        spec_id: 256,
        name: "Absolution",
        coefficient: 1.20,
    },
    MasteryDef {
        spec_id: 257,
        name: "Echo of Light",
        coefficient: 1.25,
    },
    MasteryDef {
        spec_id: 258,
        name: "Shadow Weaving",
        coefficient: 2.00,
    },
    // Death Knight
    MasteryDef {
        spec_id: 250,
        name: "Blood Shield",
        coefficient: 2.00,
    },
    MasteryDef {
        spec_id: 251,
        name: "Frozen Heart",
        coefficient: 2.00,
    },
    MasteryDef {
        spec_id: 252,
        name: "Dreadblade",
        coefficient: 1.80,
    },
    // Shaman
    MasteryDef {
        spec_id: 262,
        name: "Elemental Overload",
        coefficient: 1.88,
    },
    MasteryDef {
        spec_id: 263,
        name: "Enhanced Elements",
        coefficient: 2.00,
    },
    MasteryDef {
        spec_id: 264,
        name: "Deep Healing",
        coefficient: 3.00,
    },
    // Mage
    MasteryDef {
        spec_id: 62,
        name: "Savant",
        coefficient: 1.20,
    },
    MasteryDef {
        spec_id: 63,
        name: "Ignite",
        coefficient: 0.75,
    },
    MasteryDef {
        spec_id: 64,
        name: "Icicles",
        coefficient: 2.25,
    },
    // Warlock
    MasteryDef {
        spec_id: 265,
        name: "Potent Afflictions",
        coefficient: 2.50,
    },
    MasteryDef {
        spec_id: 266,
        name: "Master Demonologist",
        coefficient: 1.80,
    },
    MasteryDef {
        spec_id: 267,
        name: "Chaotic Energies",
        coefficient: 1.50,
    },
    // Monk
    MasteryDef {
        spec_id: 268,
        name: "Elusive Brawler",
        coefficient: 1.00,
    },
    MasteryDef {
        spec_id: 270,
        name: "Gust of Mists",
        coefficient: 1.40,
    },
    MasteryDef {
        spec_id: 269,
        name: "Combo Strikes",
        coefficient: 1.25,
    },
    // Druid
    MasteryDef {
        spec_id: 102,
        name: "Astral Invocation",
        coefficient: 1.10,
    },
    MasteryDef {
        spec_id: 103,
        name: "Razor Claws",
        coefficient: 2.00,
    },
    MasteryDef {
        spec_id: 104,
        name: "Nature's Guardian",
        coefficient: 1.00,
    },
    MasteryDef {
        spec_id: 105,
        name: "Harmony",
        coefficient: 1.25,
    },
    // Demon Hunter
    MasteryDef {
        spec_id: 577,
        name: "Demonic Presence",
        coefficient: 1.80,
    },
    MasteryDef {
        spec_id: 581,
        name: "Fel Blood",
        coefficient: 2.00,
    },
    // Evoker
    MasteryDef {
        spec_id: 1467,
        name: "Giantkiller",
        coefficient: 1.80,
    },
    MasteryDef {
        spec_id: 1468,
        name: "Life-Binder's Bond",
        coefficient: 1.20,
    },
    MasteryDef {
        spec_id: 1473,
        name: "Timewalker",
        coefficient: 1.00,
    },
];

/// Look up the mastery definition for a spec.
pub fn mastery_for_spec(spec_id: u32) -> Option<&'static MasteryDef> {
    MASTERY_DEFS.iter().find(|m| m.spec_id == spec_id)
}

/// Calculate the effective mastery bonus percentage for a spec.
///
/// `mastery_points` comes from `rating_to_percent(rating, level, Mastery)` +
/// `apply_secondary_dr()`. The coefficient converts points to an effect %.
///
/// Example: 10 mastery points × 2.0 coefficient = 20% bonus.
pub fn effective_mastery(spec_id: u32, mastery_points: f32) -> f32 {
    mastery_for_spec(spec_id)
        .map(|def| mastery_points * def.coefficient)
        .unwrap_or(0.0)
}

// --- Talent system ---

/// A talent definition. Talents are passive bonuses that apply aura effects
/// when selected. Each talent belongs to a spec and grants one or more
/// `AuraEffect`s as a permanent passive aura.
///
/// Ref: WoW talent trees — key talents modify combat behavior
/// (e.g. +10% crit, +20% damage to Mortal Strike, threat reduction).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TalentDef {
    /// Unique talent ID.
    pub id: u32,
    /// Display name.
    pub name: String,
    /// Which spec this talent belongs to (0 = class-wide).
    pub spec_id: u32,
    /// The passive aura effects this talent grants.
    pub effects: Vec<AuraEffect>,
}

/// Component tracking a player's selected talents.
///
/// Selected talents are converted into permanent passive auras on the entity.
/// When talents change (respec), old auras are removed and new ones applied.
#[derive(Component, Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SelectedTalents {
    pub talent_ids: BTreeSet<u32>,
}

impl SelectedTalents {
    pub fn select(&mut self, talent_id: u32) -> bool {
        self.talent_ids.insert(talent_id)
    }

    /// Build passive auras from selected talents.
    ///
    /// Each talent becomes a permanent aura (duration=0 means infinite) with
    /// the talent's effects. The server applies these as persistent auras.
    pub fn build_passive_auras(&self, talent_defs: &[TalentDef]) -> Vec<Aura> {
        self.talent_ids
            .iter()
            .filter_map(|id| talent_defs.iter().find(|t| t.id == *id))
            .map(|talent| {
                let mut effects: [Option<AuraEffect>; 3] = [None, None, None];
                for (i, effect) in talent.effects.iter().take(3).enumerate() {
                    effects[i] = Some(*effect);
                }
                Aura {
                    spell_id: talent.id,
                    caster: 0,     // self-applied
                    duration: 0.0, // permanent (never expires)
                    remaining: f32::MAX,
                    effects,
                    ..Default::default()
                }
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "class_spec_tests.rs"]
mod tests;
