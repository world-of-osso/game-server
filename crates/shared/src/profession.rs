//! Profession types: gathering and crafting.
//!
//! Ref: AzerothCore `SkillLineAbility.dbc`, profession skill system.

use serde::{Deserialize, Serialize};

/// Profession category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProfessionCategory {
    Gathering,
    Crafting,
    Secondary,
}

/// All WoW professions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            Self::Mining, Self::Herbalism, Self::Skinning,
            Self::Blacksmithing, Self::Leatherworking, Self::Tailoring,
            Self::Engineering, Self::Alchemy, Self::Enchanting,
            Self::Jewelcrafting, Self::Inscription,
            Self::Cooking, Self::Fishing,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
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
            let primary_count = self.skills.iter().filter(|s| s.profession.is_primary()).count();
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
    Success { items: Vec<(u32, u16)>, skilled_up: bool },
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
    if skilled_up
        && let Some(s) = profs.get_mut(node.profession)
    {
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
    if skilled_up
        && let Some(s) = profs.get_mut(recipe.profession)
    {
        s.skill_up();
    }

    CraftResult::Success {
        output_item_id: recipe.output_item_id,
        output_count: recipe.output_count,
        skilled_up,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profession_categories() {
        assert_eq!(Profession::Mining.category(), ProfessionCategory::Gathering);
        assert_eq!(Profession::Blacksmithing.category(), ProfessionCategory::Crafting);
        assert_eq!(Profession::Cooking.category(), ProfessionCategory::Secondary);
        assert!(Profession::Mining.is_primary());
        assert!(!Profession::Cooking.is_primary());
    }

    #[test]
    fn profession_all_count() {
        assert_eq!(Profession::all().len(), 13);
    }

    #[test]
    fn learn_profession() {
        let mut profs = PlayerProfessions::default();
        assert!(profs.learn(Profession::Mining).is_ok());
        assert!(profs.knows(Profession::Mining));
        assert_eq!(profs.get(Profession::Mining).unwrap().current, 1);
    }

    #[test]
    fn learn_duplicate_rejected() {
        let mut profs = PlayerProfessions::default();
        profs.learn(Profession::Mining).unwrap();
        assert_eq!(profs.learn(Profession::Mining), Err(ProfessionError::AlreadyKnown));
    }

    #[test]
    fn max_two_primary() {
        let mut profs = PlayerProfessions::default();
        profs.learn(Profession::Mining).unwrap();
        profs.learn(Profession::Herbalism).unwrap();
        assert_eq!(profs.learn(Profession::Skinning), Err(ProfessionError::TooManyPrimary));
    }

    #[test]
    fn secondary_unlimited() {
        let mut profs = PlayerProfessions::default();
        profs.learn(Profession::Mining).unwrap();
        profs.learn(Profession::Herbalism).unwrap();
        // Secondary professions don't count toward the 2 limit
        assert!(profs.learn(Profession::Cooking).is_ok());
        assert!(profs.learn(Profession::Fishing).is_ok());
    }

    #[test]
    fn skill_up() {
        let mut profs = PlayerProfessions::default();
        profs.learn(Profession::Mining).unwrap();
        let skill = profs.get_mut(Profession::Mining).unwrap();
        assert!(skill.skill_up());
        assert_eq!(skill.current, 2);
    }

    #[test]
    fn skill_up_capped() {
        let mut skill = ProfessionSkill::new(Profession::Mining);
        skill.current = 75; // at max
        assert!(!skill.skill_up());
    }

    #[test]
    fn train_tier() {
        let mut skill = ProfessionSkill::new(Profession::Mining);
        skill.train_tier(ProfessionTier::Journeyman);
        assert_eq!(skill.max, 150);
        skill.train_tier(ProfessionTier::GrandMaster);
        assert_eq!(skill.max, 450);
    }

    // --- Tier system tests ---

    #[test]
    fn tier_skill_caps() {
        assert_eq!(ProfessionTier::Apprentice.max_skill(), 75);
        assert_eq!(ProfessionTier::Journeyman.max_skill(), 150);
        assert_eq!(ProfessionTier::Expert.max_skill(), 225);
        assert_eq!(ProfessionTier::Artisan.max_skill(), 300);
        assert_eq!(ProfessionTier::Master.max_skill(), 375);
        assert_eq!(ProfessionTier::GrandMaster.max_skill(), 450);
    }

    #[test]
    fn tier_required_levels() {
        assert_eq!(ProfessionTier::Apprentice.required_level(), 5);
        assert_eq!(ProfessionTier::Expert.required_level(), 20);
        assert_eq!(ProfessionTier::GrandMaster.required_level(), 65);
    }

    #[test]
    fn tier_next() {
        assert_eq!(ProfessionTier::Apprentice.next(), Some(ProfessionTier::Journeyman));
        assert_eq!(ProfessionTier::GrandMaster.next(), None);
    }

    #[test]
    fn next_trainable_apprentice_to_journeyman() {
        // Apprentice (max=75), skill=50, level=10 → can train Journeyman
        let tier = next_trainable_tier(75, 50, 10);
        assert_eq!(tier, Some(ProfessionTier::Journeyman));
    }

    #[test]
    fn next_trainable_skill_too_low() {
        // Max=75, skill=30, level=10 → need 50 skill for Journeyman
        let tier = next_trainable_tier(75, 30, 10);
        assert_eq!(tier, None);
    }

    #[test]
    fn next_trainable_level_too_low() {
        // Max=75, skill=50, level=5 → need level 10 for Journeyman
        let tier = next_trainable_tier(75, 50, 5);
        assert_eq!(tier, None);
    }

    #[test]
    fn next_trainable_at_grand_master() {
        let tier = next_trainable_tier(450, 450, 80);
        assert_eq!(tier, None); // already max
    }

    #[test]
    fn unlearn_profession() {
        let mut profs = PlayerProfessions::default();
        profs.learn(Profession::Mining).unwrap();
        profs.unlearn(Profession::Mining);
        assert!(!profs.knows(Profession::Mining));
        assert!(profs.learn(Profession::Skinning).is_ok());
    }

    // --- Gathering tests ---

    fn copper_vein() -> GatherNode {
        GatherNode {
            profession: Profession::Mining,
            required_skill: 1,
            trivial_skill: 50,
            yields: vec![(2770, 1, 3)], // Copper Ore
            respawn_secs: 300,
        }
    }

    fn mining_profs(skill: u16) -> PlayerProfessions {
        let mut profs = PlayerProfessions::default();
        profs.learn(Profession::Mining).unwrap();
        let s = profs.get_mut(Profession::Mining).unwrap();
        s.current = skill;
        s.max = 75;
        profs
    }

    #[test]
    fn gather_success() {
        let node = copper_vein();
        let mut profs = mining_profs(10);
        let result = attempt_gather(&node, &mut profs, 0.5, &[0.5]);
        match result {
            GatherResult::Success { items, .. } => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].0, 2770);
                assert!(items[0].1 >= 1 && items[0].1 <= 3);
            }
            _ => panic!("expected success"),
        }
    }

    #[test]
    fn gather_skill_too_low() {
        let mut node = copper_vein();
        node.required_skill = 50;
        let mut profs = mining_profs(10);
        assert_eq!(attempt_gather(&node, &mut profs, 0.0, &[]), GatherResult::SkillTooLow);
    }

    #[test]
    fn gather_wrong_profession() {
        let node = copper_vein();
        let mut profs = PlayerProfessions::default();
        profs.learn(Profession::Herbalism).unwrap();
        assert_eq!(attempt_gather(&node, &mut profs, 0.0, &[]), GatherResult::WrongProfession);
    }

    #[test]
    fn gather_skill_up_at_low_skill() {
        let node = copper_vein(); // required=1, trivial=50
        let mut profs = mining_profs(5);
        // roll=0.0 → always skill up (chance is high at skill 5)
        let result = attempt_gather(&node, &mut profs, 0.0, &[0.0]);
        match result {
            GatherResult::Success { skilled_up, .. } => assert!(skilled_up),
            _ => panic!("expected success"),
        }
        assert_eq!(profs.get(Profession::Mining).unwrap().current, 6);
    }

    #[test]
    fn gather_no_skill_up_at_trivial() {
        let node = copper_vein();
        let mut profs = mining_profs(50); // at trivial
        let result = attempt_gather(&node, &mut profs, 0.0, &[0.0]);
        match result {
            GatherResult::Success { skilled_up, .. } => assert!(!skilled_up),
            _ => panic!("expected success"),
        }
    }

    #[test]
    fn skill_up_chance_values() {
        assert_eq!(skill_up_chance(1, 1, 50), 1.0);
        assert_eq!(skill_up_chance(50, 1, 50), 0.0);
        let mid = skill_up_chance(25, 1, 50);
        assert!(mid > 0.0 && mid < 1.0);
    }

    // --- Crafting tests ---

    fn copper_bar_recipe() -> Recipe {
        Recipe {
            id: 2657,
            profession: Profession::Mining,
            required_skill: 1,
            trivial_skill: 25,
            materials: vec![CraftMaterial { item_id: 2770, count: 2 }], // 2 Copper Ore
            output_item_id: 2840, // Copper Bar
            output_count: 1,
        }
    }

    #[test]
    fn craft_success() {
        let recipe = copper_bar_recipe();
        let mut profs = mining_profs(10);
        let inv = |id: u32| if id == 2770 { 10 } else { 0 };
        let result = attempt_craft(&recipe, &mut profs, &inv, 0.5);
        match result {
            CraftResult::Success { output_item_id, output_count, .. } => {
                assert_eq!(output_item_id, 2840);
                assert_eq!(output_count, 1);
            }
            other => panic!("expected success, got {other:?}"),
        }
    }

    #[test]
    fn craft_wrong_profession() {
        let recipe = copper_bar_recipe();
        let mut profs = PlayerProfessions::default();
        profs.learn(Profession::Herbalism).unwrap();
        let inv = |_: u32| 99;
        assert!(matches!(attempt_craft(&recipe, &mut profs, &inv, 0.0), CraftResult::WrongProfession));
    }

    #[test]
    fn craft_skill_too_low() {
        let mut recipe = copper_bar_recipe();
        recipe.required_skill = 50;
        let mut profs = mining_profs(10);
        let inv = |_: u32| 99;
        assert!(matches!(attempt_craft(&recipe, &mut profs, &inv, 0.0), CraftResult::SkillTooLow));
    }

    #[test]
    fn craft_missing_materials() {
        let recipe = copper_bar_recipe();
        let mut profs = mining_profs(10);
        let inv = |id: u32| if id == 2770 { 1 } else { 0 }; // only 1, need 2
        let result = attempt_craft(&recipe, &mut profs, &inv, 0.0);
        match result {
            CraftResult::MissingMaterials { missing } => {
                assert_eq!(missing.len(), 1);
                assert_eq!(missing[0].item_id, 2770);
                assert_eq!(missing[0].count, 1); // need 1 more
            }
            other => panic!("expected missing, got {other:?}"),
        }
    }

    #[test]
    fn craft_skill_up() {
        let recipe = copper_bar_recipe();
        let mut profs = mining_profs(5);
        let inv = |_: u32| 99;
        let result = attempt_craft(&recipe, &mut profs, &inv, 0.0); // roll=0 → skill up
        match result {
            CraftResult::Success { skilled_up, .. } => assert!(skilled_up),
            other => panic!("expected success, got {other:?}"),
        }
        assert_eq!(profs.get(Profession::Mining).unwrap().current, 6);
    }

    #[test]
    fn check_materials_all_present() {
        let recipe = copper_bar_recipe();
        let inv = |id: u32| if id == 2770 { 5 } else { 0 };
        let missing = check_materials(&recipe, &inv);
        assert!(missing.is_empty());
    }

    #[test]
    fn check_materials_partial() {
        let recipe = Recipe {
            id: 1,
            profession: Profession::Blacksmithing,
            required_skill: 1,
            trivial_skill: 50,
            materials: vec![
                CraftMaterial { item_id: 100, count: 5 },
                CraftMaterial { item_id: 200, count: 3 },
            ],
            output_item_id: 300,
            output_count: 1,
        };
        let inv = |id: u32| match id { 100 => 2, 200 => 3, _ => 0 };
        let missing = check_materials(&recipe, &inv);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], CraftMaterial { item_id: 100, count: 3 });
    }
}
