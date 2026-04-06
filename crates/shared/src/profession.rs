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

/// Maximum skill level per expansion tier.
pub const MAX_SKILL_LEVEL: u16 = 450; // WotLK cap

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
    pub fn train_tier(&mut self, new_max: u16) {
        self.max = new_max.min(MAX_SKILL_LEVEL);
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
        skill.train_tier(150); // journeyman
        assert_eq!(skill.max, 150);
        skill.train_tier(9999); // clamped
        assert_eq!(skill.max, MAX_SKILL_LEVEL);
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
