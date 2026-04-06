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
        // Can learn a new one now
        assert!(profs.learn(Profession::Skinning).is_ok());
    }
}
