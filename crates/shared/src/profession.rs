//! Profession types: gathering and crafting.
//!
//! Ref: AzerothCore `SkillLineAbility.dbc`, profession skill system.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Profession category.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
)]
pub enum ProfessionCategory {
    Gathering,
    Crafting,
    Secondary,
}

/// All WoW professions.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
)]
pub enum Profession {
    // Gathering
    Mining,
    Herbalism,
    Skinning,
    // Crafting
    Blacksmithing,
    Leatherworking,
    Tailoring,
    Engineering,
    Alchemy,
    Enchanting,
    Jewelcrafting,
    Inscription,
    // Secondary (don't count toward the 2-primary limit)
    Cooking,
    Fishing,
}

impl Profession {
    pub fn category(self) -> ProfessionCategory {
        match self {
            Self::Mining | Self::Herbalism | Self::Skinning => ProfessionCategory::Gathering,
            Self::Cooking | Self::Fishing => ProfessionCategory::Secondary,
            _ => ProfessionCategory::Crafting,
        }
    }

    /// Whether this counts toward the 2-primary-profession limit.
    pub fn is_primary(self) -> bool {
        self.category() != ProfessionCategory::Secondary
    }

    /// All profession variants.
    pub fn all() -> &'static [Profession] {
        &[
            Self::Mining,
            Self::Herbalism,
            Self::Skinning,
            Self::Blacksmithing,
            Self::Leatherworking,
            Self::Tailoring,
            Self::Engineering,
            Self::Alchemy,
            Self::Enchanting,
            Self::Jewelcrafting,
            Self::Inscription,
            Self::Cooking,
            Self::Fishing,
        ]
    }
}

/// Maximum primary professions a player can have.
pub const MAX_PRIMARY_PROFESSIONS: usize = 2;

/// Maximum skill level (WotLK Grand Master).
pub const MAX_SKILL_LEVEL: u16 = 450;

/// Profession training tiers with skill caps and required character level.
///
/// Ref: AzerothCore profession trainer data.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, bitcode::Encode, bitcode::Decode,
)]
pub enum ProfessionTier {
    Apprentice,
    Journeyman,
    Expert,
    Artisan,
    /// TBC tier.
    Master,
    /// WotLK tier.
    GrandMaster,
}

/// Tier progression data: (tier, max_skill, required_char_level, required_skill).
const TIER_DATA: &[(ProfessionTier, u16, u8, u16)] = &[
    (ProfessionTier::Apprentice, 75, 5, 0),
    (ProfessionTier::Journeyman, 150, 10, 50),
    (ProfessionTier::Expert, 225, 20, 125),
    (ProfessionTier::Artisan, 300, 35, 200),
    (ProfessionTier::Master, 375, 50, 275),
    (ProfessionTier::GrandMaster, 450, 65, 350),
];

impl ProfessionTier {
    /// Skill cap for this tier.
    pub fn max_skill(self) -> u16 {
        TIER_DATA
            .iter()
            .find(|(t, _, _, _)| *t == self)
            .map(|(_, max, _, _)| *max)
            .unwrap_or(75)
    }

    /// Required character level to train this tier.
    pub fn required_level(self) -> u8 {
        TIER_DATA
            .iter()
            .find(|(t, _, _, _)| *t == self)
            .map(|(_, _, lvl, _)| *lvl)
            .unwrap_or(5)
    }

    /// Required skill level to train this tier.
    pub fn required_skill(self) -> u16 {
        TIER_DATA
            .iter()
            .find(|(t, _, _, _)| *t == self)
            .map(|(_, _, _, skill)| *skill)
            .unwrap_or(0)
    }

    /// All tiers in order.
    pub fn all() -> &'static [ProfessionTier] {
        &[
            Self::Apprentice,
            Self::Journeyman,
            Self::Expert,
            Self::Artisan,
            Self::Master,
            Self::GrandMaster,
        ]
    }

    /// The next tier after this one, if any.
    pub fn next(self) -> Option<ProfessionTier> {
        let tiers = Self::all();
        tiers
            .iter()
            .position(|&t| t == self)
            .and_then(|i| tiers.get(i + 1).copied())
    }
}

/// Determine which tier a skill can train next, given current skill and character level.
/// Returns `None` if already at max tier or doesn't meet requirements.
pub fn next_trainable_tier(
    current_max: u16,
    current_skill: u16,
    player_level: u8,
) -> Option<ProfessionTier> {
    TIER_DATA
        .iter()
        .find(|(_, max, req_lvl, req_skill)| {
            *max > current_max && player_level >= *req_lvl && current_skill >= *req_skill
        })
        .map(|(tier, _, _, _)| *tier)
}

/// A player's skill in a single profession.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, bitcode::Encode, bitcode::Decode,
)]
pub struct ProfessionSkill {
    pub profession: Profession,
    pub current: u16,
    pub max: u16,
}

impl ProfessionSkill {
    pub fn new(profession: Profession) -> Self {
        Self {
            profession,
            current: 1,
            max: 75, // apprentice tier
        }
    }

    /// Increase skill by 1 (from a successful gather/craft). Clamped to max.
    pub fn skill_up(&mut self) -> bool {
        if self.current >= self.max {
            return false;
        }
        self.current += 1;
        true
    }

    /// Unlock the next tier (raises max skill).
    pub fn train_tier(&mut self, tier: ProfessionTier) {
        self.max = tier.max_skill();
    }
}

/// A player's learned professions.
#[derive(
    Component,
    Debug,
    Clone,
    PartialEq,
    Default,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
)]
pub struct PlayerProfessions {
    pub skills: Vec<ProfessionSkill>,
}

/// Why learning a profession failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfessionError {
    /// Already know this profession.
    AlreadyKnown,
    /// Already have 2 primary professions.
    TooManyPrimary,
}

impl PlayerProfessions {
    /// Learn a new profession. Enforces the 2-primary limit.
    pub fn learn(&mut self, profession: Profession) -> Result<(), ProfessionError> {
        if self.skills.iter().any(|s| s.profession == profession) {
            return Err(ProfessionError::AlreadyKnown);
        }
        if profession.is_primary() {
            let primary_count = self
                .skills
                .iter()
                .filter(|s| s.profession.is_primary())
                .count();
            if primary_count >= MAX_PRIMARY_PROFESSIONS {
                return Err(ProfessionError::TooManyPrimary);
            }
        }
        self.skills.push(ProfessionSkill::new(profession));
        Ok(())
    }

    /// Unlearn a profession (removes it and all skill).
    pub fn unlearn(&mut self, profession: Profession) {
        self.skills.retain(|s| s.profession != profession);
    }

    /// Get skill for a profession.
    pub fn get(&self, profession: Profession) -> Option<&ProfessionSkill> {
        self.skills.iter().find(|s| s.profession == profession)
    }

    /// Get mutable skill for a profession.
    pub fn get_mut(&mut self, profession: Profession) -> Option<&mut ProfessionSkill> {
        self.skills.iter_mut().find(|s| s.profession == profession)
    }

    /// Whether the player knows a profession.
    pub fn knows(&self, profession: Profession) -> bool {
        self.skills.iter().any(|s| s.profession == profession)
    }
}

// --- Gathering ---

/// A gatherable world node (ore vein, herb, etc).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatherNode {
    /// Which gathering profession this requires.
    pub profession: Profession,
    /// Minimum skill to gather from this node.
    pub required_skill: u16,
    /// Skill level where gathering becomes guaranteed (no fail).
    pub trivial_skill: u16,
    /// Items yielded on success: (item_id, min_count, max_count).
    pub yields: Vec<(u32, u16, u16)>,
    /// Respawn time in seconds (0 = one-time like skinning).
    pub respawn_secs: u32,
}

/// Result of attempting to gather from a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatherResult {
    /// Successfully gathered items.
    Success {
        items: Vec<(u32, u16)>,
        skilled_up: bool,
    },
    /// Skill too low to gather this node.
    SkillTooLow,
    /// Player doesn't have the required profession.
    WrongProfession,
}

/// Skill-up chance based on current skill vs node difficulty.
///
/// - Below required: impossible (caught before this)
/// - Required..trivial: 100% chance → linear decay to 0%
/// - At or above trivial: 0% skill-up chance
fn skill_up_chance(current_skill: u16, required: u16, trivial: u16) -> f32 {
    if current_skill >= trivial {
        return 0.0;
    }
    if trivial <= required {
        return 1.0;
    }
    let range = (trivial - required) as f32;
    let progress = (current_skill - required) as f32;
    1.0 - progress / range
}

/// Attempt to gather from a node.
///
/// `roll` is 0.0–1.0 for skill-up chance.
/// `count_rolls` maps to each yield entry for picking count within range.
pub fn attempt_gather(
    node: &GatherNode,
    profs: &mut PlayerProfessions,
    roll: f32,
    count_rolls: &[f32],
) -> GatherResult {
    let Some(skill) = profs.get(node.profession) else {
        return GatherResult::WrongProfession;
    };
    if skill.current < node.required_skill {
        return GatherResult::SkillTooLow;
    }

    let chance = skill_up_chance(skill.current, node.required_skill, node.trivial_skill);
    let skilled_up = roll < chance;
    if skilled_up && let Some(s) = profs.get_mut(node.profession) {
        s.skill_up();
    }

    let items: Vec<(u32, u16)> = node
        .yields
        .iter()
        .zip(count_rolls.iter().chain(std::iter::repeat(&0.5)))
        .map(|(&(item_id, min, max), &cr)| {
            let range = max - min;
            let count = min + (range as f32 * cr) as u16;
            (item_id, count.min(max))
        })
        .collect();

    GatherResult::Success { items, skilled_up }
}

// --- Crafting ---

/// A material required to craft a recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftMaterial {
    pub item_id: u32,
    pub count: u16,
}

/// A crafting recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    /// Unique recipe ID.
    pub id: u32,
    /// Which profession this recipe belongs to.
    pub profession: Profession,
    /// Minimum skill to learn/use this recipe.
    pub required_skill: u16,
    /// Skill level where the recipe turns grey (no more skill-ups).
    pub trivial_skill: u16,
    /// Materials consumed on craft.
    pub materials: Vec<CraftMaterial>,
    /// Output item ID.
    pub output_item_id: u32,
    /// Output count.
    pub output_count: u16,
    /// How this recipe is acquired (None = always known / from test data).
    pub source: Option<RecipeSource>,
}

/// Result of attempting to craft a recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CraftResult {
    /// Successfully crafted.
    Success {
        output_item_id: u32,
        output_count: u16,
        skilled_up: bool,
    },
    /// Player doesn't have the required profession.
    WrongProfession,
    /// Skill too low for this recipe.
    SkillTooLow,
    /// Missing one or more materials.
    MissingMaterials { missing: Vec<CraftMaterial> },
}

/// Check which materials the player is missing for a recipe.
///
/// `inventory_check` returns how many of each item_id the player has.
pub fn check_materials(
    recipe: &Recipe,
    inventory_check: &dyn Fn(u32) -> u16,
) -> Vec<CraftMaterial> {
    recipe
        .materials
        .iter()
        .filter_map(|mat| {
            let have = inventory_check(mat.item_id);
            if have >= mat.count {
                None
            } else {
                Some(CraftMaterial {
                    item_id: mat.item_id,
                    count: mat.count - have,
                })
            }
        })
        .collect()
}

/// Attempt to craft a recipe.
///
/// `inventory_check` returns how many of each item_id the player has.
/// `roll` is 0.0–1.0 for skill-up chance.
///
/// On success, the caller must consume materials and grant the output item.
pub fn attempt_craft(
    recipe: &Recipe,
    profs: &mut PlayerProfessions,
    inventory_check: &dyn Fn(u32) -> u16,
    roll: f32,
) -> CraftResult {
    let Some(skill) = profs.get(recipe.profession) else {
        return CraftResult::WrongProfession;
    };
    if skill.current < recipe.required_skill {
        return CraftResult::SkillTooLow;
    }

    let missing = check_materials(recipe, inventory_check);
    if !missing.is_empty() {
        return CraftResult::MissingMaterials { missing };
    }

    let chance = skill_up_chance(skill.current, recipe.required_skill, recipe.trivial_skill);
    let skilled_up = roll < chance;
    if skilled_up && let Some(s) = profs.get_mut(recipe.profession) {
        s.skill_up();
    }

    CraftResult::Success {
        output_item_id: recipe.output_item_id,
        output_count: recipe.output_count,
        skilled_up,
    }
}

// --- Profession specializations ---

/// A profession specialization (e.g. Weaponsmith, Elixir Master).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfessionSpec {
    pub id: u32,
    pub profession: Profession,
    pub name: &'static str,
    /// Minimum skill level to choose this specialization.
    pub required_skill: u16,
}

/// All profession specializations.
///
/// Ref: AzerothCore specialization quests.
pub const PROFESSION_SPECS: &[ProfessionSpec] = &[
    // Blacksmithing
    ProfessionSpec {
        id: 1,
        profession: Profession::Blacksmithing,
        name: "Weaponsmith",
        required_skill: 200,
    },
    ProfessionSpec {
        id: 2,
        profession: Profession::Blacksmithing,
        name: "Armorsmith",
        required_skill: 200,
    },
    // Leatherworking
    ProfessionSpec {
        id: 3,
        profession: Profession::Leatherworking,
        name: "Elemental LW",
        required_skill: 225,
    },
    ProfessionSpec {
        id: 4,
        profession: Profession::Leatherworking,
        name: "Dragonscale LW",
        required_skill: 225,
    },
    ProfessionSpec {
        id: 5,
        profession: Profession::Leatherworking,
        name: "Tribal LW",
        required_skill: 225,
    },
    // Alchemy
    ProfessionSpec {
        id: 6,
        profession: Profession::Alchemy,
        name: "Elixir Master",
        required_skill: 325,
    },
    ProfessionSpec {
        id: 7,
        profession: Profession::Alchemy,
        name: "Potion Master",
        required_skill: 325,
    },
    ProfessionSpec {
        id: 8,
        profession: Profession::Alchemy,
        name: "Transmutation Master",
        required_skill: 325,
    },
    // Engineering
    ProfessionSpec {
        id: 9,
        profession: Profession::Engineering,
        name: "Gnomish Engineer",
        required_skill: 200,
    },
    ProfessionSpec {
        id: 10,
        profession: Profession::Engineering,
        name: "Goblin Engineer",
        required_skill: 200,
    },
    // Tailoring
    ProfessionSpec {
        id: 11,
        profession: Profession::Tailoring,
        name: "Mooncloth Tailor",
        required_skill: 350,
    },
    ProfessionSpec {
        id: 12,
        profession: Profession::Tailoring,
        name: "Shadoweave Tailor",
        required_skill: 350,
    },
    ProfessionSpec {
        id: 13,
        profession: Profession::Tailoring,
        name: "Spellfire Tailor",
        required_skill: 350,
    },
];

/// Get available specializations for a profession.
pub fn specs_for_profession(profession: Profession) -> Vec<&'static ProfessionSpec> {
    PROFESSION_SPECS
        .iter()
        .filter(|s| s.profession == profession)
        .collect()
}

/// Player's chosen profession specializations.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ChosenSpecs {
    pub spec_ids: Vec<u32>,
}

/// Why choosing a specialization failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecError {
    /// Already specialized in this profession.
    AlreadySpecialized,
    /// Skill too low to specialize.
    SkillTooLow,
    /// Spec not found.
    InvalidSpec,
}

impl ChosenSpecs {
    /// Choose a specialization. One per profession.
    pub fn choose(&mut self, spec_id: u32, profs: &PlayerProfessions) -> Result<(), SpecError> {
        let spec = PROFESSION_SPECS
            .iter()
            .find(|s| s.id == spec_id)
            .ok_or(SpecError::InvalidSpec)?;

        // Check if already specialized in this profession
        let already = self.spec_ids.iter().any(|&id| {
            PROFESSION_SPECS
                .iter()
                .any(|s| s.id == id && s.profession == spec.profession)
        });
        if already {
            return Err(SpecError::AlreadySpecialized);
        }

        let skill = profs.get(spec.profession).ok_or(SpecError::SkillTooLow)?;
        if skill.current < spec.required_skill {
            return Err(SpecError::SkillTooLow);
        }

        self.spec_ids.push(spec_id);
        Ok(())
    }

    /// Whether the player has a specific spec.
    pub fn has_spec(&self, spec_id: u32) -> bool {
        self.spec_ids.contains(&spec_id)
    }

    /// Get the player's spec for a profession, if any.
    pub fn spec_for_profession(&self, profession: Profession) -> Option<&'static ProfessionSpec> {
        self.spec_ids.iter().find_map(|&id| {
            PROFESSION_SPECS
                .iter()
                .find(|s| s.id == id && s.profession == profession)
        })
    }

    /// Drop a specialization (re-specialize).
    pub fn drop_spec(&mut self, profession: Profession) {
        self.spec_ids.retain(|&id| {
            !PROFESSION_SPECS
                .iter()
                .any(|s| s.id == id && s.profession == profession)
        });
    }
}

// --- Recipe sources ---

/// How a recipe is acquired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecipeSource {
    /// Learned from a profession trainer at the required skill level.
    Trainer,
    /// Learned from a recipe item (world drop, vendor, quest reward).
    Item { item_id: u32 },
    /// Discovered randomly while crafting other recipes in the profession.
    Discovery { chance: u16 },
}

/// A player's known recipes for one profession.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct KnownRecipes {
    pub recipe_ids: std::collections::HashSet<u32>,
}

impl KnownRecipes {
    /// Learn a recipe. Returns `false` if already known.
    pub fn learn(&mut self, recipe_id: u32) -> bool {
        self.recipe_ids.insert(recipe_id)
    }

    /// Whether a recipe is known.
    pub fn knows(&self, recipe_id: u32) -> bool {
        self.recipe_ids.contains(&recipe_id)
    }

    /// Get all trainer recipes available at a skill level.
    pub fn available_from_trainer(recipes: &[Recipe], skill: u16) -> Vec<&Recipe> {
        recipes
            .iter()
            .filter(|r| {
                matches!(r.source, Some(RecipeSource::Trainer)) && r.required_skill <= skill
            })
            .collect()
    }

    /// Roll for a discovery while crafting. Returns the recipe ID if discovered.
    ///
    /// `discoverable` contains recipes with `RecipeSource::Discovery`.
    /// `roll` is 0..10000 (hundredths of percent).
    pub fn roll_discovery(&mut self, discoverable: &[Recipe], roll: u16) -> Option<u32> {
        for recipe in discoverable {
            if let Some(RecipeSource::Discovery { chance }) = recipe.source
                && !self.knows(recipe.id)
                && roll < chance
            {
                self.learn(recipe.id);
                return Some(recipe.id);
            }
        }
        None
    }
}

#[cfg(test)]
#[path = "profession_tests.rs"]
mod tests;
