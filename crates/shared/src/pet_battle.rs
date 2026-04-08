use serde::{Deserialize, Serialize};

/// Pet quality tier (affects stat scaling).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PetQuality {
    Poor = 0,
    Common = 1,
    Uncommon = 2,
    Rare = 3,
    Epic = 4,
    Legendary = 5,
}

impl PetQuality {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Poor),
            1 => Some(Self::Common),
            2 => Some(Self::Uncommon),
            3 => Some(Self::Rare),
            4 => Some(Self::Epic),
            5 => Some(Self::Legendary),
            _ => None,
        }
    }
}

/// Maximum pet level.
pub const MAX_PET_LEVEL: u8 = 25;
/// Maximum pets per account journal.
pub const MAX_JOURNAL_SIZE: usize = 1000;
/// Maximum duplicates of the same species.
pub const MAX_PER_SPECIES: usize = 3;

/// A single pet instance owned by an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedPet {
    /// Unique pet instance ID.
    pub id: u64,
    /// Species template ID (which kind of pet).
    pub species_id: u32,
    /// Pet's custom name (or species default if None).
    pub custom_name: Option<String>,
    /// Current level (1–25).
    pub level: u8,
    /// Quality tier.
    pub quality: PetQuality,
    /// Current XP toward next level.
    pub xp: u32,
}

/// Why a pet journal operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetJournalError {
    /// Journal is full (1000 pets max).
    JournalFull,
    /// Already have 3 of this species.
    SpeciesLimitReached,
    /// Pet not found in journal.
    PetNotFound,
    /// Name is empty.
    EmptyName,
}

/// Account-wide pet collection.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PetJournal {
    pub pets: Vec<OwnedPet>,
    next_id: u64,
}

impl PetJournal {
    /// Add a new pet to the journal. Returns the assigned pet ID.
    pub fn add(
        &mut self,
        species_id: u32,
        level: u8,
        quality: PetQuality,
    ) -> Result<u64, PetJournalError> {
        if self.pets.len() >= MAX_JOURNAL_SIZE {
            return Err(PetJournalError::JournalFull);
        }
        let species_count = self.count_species(species_id);
        if species_count >= MAX_PER_SPECIES {
            return Err(PetJournalError::SpeciesLimitReached);
        }
        self.next_id += 1;
        let id = self.next_id;
        self.pets.push(OwnedPet {
            id,
            species_id,
            custom_name: None,
            level: level.clamp(1, MAX_PET_LEVEL),
            quality,
            xp: 0,
        });
        Ok(id)
    }

    /// Remove a pet from the journal (release).
    pub fn remove(&mut self, pet_id: u64) -> Result<(), PetJournalError> {
        let idx = self
            .pets
            .iter()
            .position(|p| p.id == pet_id)
            .ok_or(PetJournalError::PetNotFound)?;
        self.pets.remove(idx);
        Ok(())
    }

    /// Rename a pet.
    pub fn rename(&mut self, pet_id: u64, name: String) -> Result<(), PetJournalError> {
        if name.is_empty() {
            return Err(PetJournalError::EmptyName);
        }
        let pet = self
            .pets
            .iter_mut()
            .find(|p| p.id == pet_id)
            .ok_or(PetJournalError::PetNotFound)?;
        pet.custom_name = Some(name);
        Ok(())
    }

    /// Get a pet by ID.
    pub fn get(&self, pet_id: u64) -> Option<&OwnedPet> {
        self.pets.iter().find(|p| p.id == pet_id)
    }

    /// Number of pets in the journal.
    pub fn count(&self) -> usize {
        self.pets.len()
    }

    /// Number of unique species collected.
    pub fn unique_species(&self) -> usize {
        let mut species: Vec<u32> = self.pets.iter().map(|p| p.species_id).collect();
        species.sort();
        species.dedup();
        species.len()
    }

    /// Count how many of a specific species are owned.
    pub fn count_species(&self, species_id: u32) -> usize {
        self.pets
            .iter()
            .filter(|p| p.species_id == species_id)
            .count()
    }

    /// All pets of a given species.
    pub fn by_species(&self, species_id: u32) -> Vec<&OwnedPet> {
        self.pets
            .iter()
            .filter(|p| p.species_id == species_id)
            .collect()
    }

    /// Highest-level pet in the journal.
    pub fn highest_level(&self) -> u8 {
        self.pets.iter().map(|p| p.level).max().unwrap_or(0)
    }
}

// -- Pet XP & Leveling --

/// XP required to advance from a given level to the next.
///
/// Quadratic curve: each level requires more XP than the last.
/// Formula: `50 + 25 * level^2`. Level 25 is max (no further XP needed).
pub fn xp_to_next_level(level: u8) -> u32 {
    if level >= MAX_PET_LEVEL {
        return 0; // already max
    }
    50 + 25 * (level as u32) * (level as u32)
}

/// Total XP needed from level 1 to reach a target level.
pub fn total_xp_for_level(target_level: u8) -> u32 {
    (1..target_level).map(xp_to_next_level).sum()
}

/// Result of awarding XP to a pet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XpGainResult {
    pub xp_gained: u32,
    pub levels_gained: u8,
    pub new_level: u8,
    pub new_xp: u32,
}

/// Award XP to a pet, handling level-ups. Returns details of what changed.
pub fn award_xp(pet: &mut OwnedPet, xp: u32) -> XpGainResult {
    if pet.level >= MAX_PET_LEVEL {
        return XpGainResult {
            xp_gained: 0,
            levels_gained: 0,
            new_level: pet.level,
            new_xp: pet.xp,
        };
    }
    let start_level = pet.level;
    pet.xp += xp;

    while pet.level < MAX_PET_LEVEL {
        let needed = xp_to_next_level(pet.level);
        if pet.xp < needed {
            break;
        }
        pet.xp -= needed;
        pet.level += 1;
    }

    // Cap XP at 0 if max level reached
    if pet.level >= MAX_PET_LEVEL {
        pet.xp = 0;
    }

    XpGainResult {
        xp_gained: xp,
        levels_gained: pet.level - start_level,
        new_level: pet.level,
        new_xp: pet.xp,
    }
}

/// XP awarded for defeating a pet in battle.
///
/// Base XP = 100, scaled by opponent level. Higher-level opponents give more.
pub fn battle_xp(winner_level: u8, opponent_level: u8) -> u32 {
    let base = 100u32;
    let level_bonus = opponent_level.saturating_sub(winner_level.saturating_sub(5)) as u32;
    base + level_bonus * 10
}

// -- Species Template & Stats --

/// Base stats for a pet species (before level/quality scaling).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PetSpecies {
    pub id: u32,
    /// Pet family (determines type effectiveness).
    pub family: PetFamily,
    /// Base health at level 1, Common quality.
    pub base_health: f32,
    /// Base power (attack) at level 1, Common quality.
    pub base_power: f32,
    /// Base speed at level 1, Common quality.
    pub base_speed: f32,
}

/// Computed combat stats for a pet at a specific level and quality.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetStats {
    pub health: u32,
    pub power: u32,
    pub speed: u32,
}

/// Quality stat multiplier. Higher quality → better stats.
///
/// WoW pet battle quality multipliers (approximate):
/// Poor=0.9, Common=1.0, Uncommon=1.1, Rare=1.2, Epic=1.3, Legendary=1.4
fn quality_multiplier(quality: PetQuality) -> f32 {
    match quality {
        PetQuality::Poor => 0.9,
        PetQuality::Common => 1.0,
        PetQuality::Uncommon => 1.1,
        PetQuality::Rare => 1.2,
        PetQuality::Epic => 1.3,
        PetQuality::Legendary => 1.4,
    }
}

/// Level scaling factor. Stats scale linearly from 1.0 at level 1 to 5.0 at level 25.
fn level_multiplier(level: u8) -> f32 {
    let clamped = level.clamp(1, MAX_PET_LEVEL);
    1.0 + (clamped - 1) as f32 * (4.0 / 24.0)
}

/// Compute effective stats for a pet given its species, level, and quality.
///
/// Formula: `stat = round(base * level_multiplier * quality_multiplier)`
///
/// - Level scales linearly: 1.0× at level 1, 5.0× at level 25
/// - Quality adds a percentage bonus per tier
/// - Health gets a +5 × level bonus (pets are tankier than raw stats)
pub fn compute_stats(species: &PetSpecies, level: u8, quality: PetQuality) -> PetStats {
    let lvl = level_multiplier(level);
    let qual = quality_multiplier(quality);
    let health_bonus = 5.0 * level.clamp(1, MAX_PET_LEVEL) as f32;
    PetStats {
        health: (species.base_health * lvl * qual + health_bonus).round() as u32,
        power: (species.base_power * lvl * qual).round() as u32,
        speed: (species.base_speed * lvl * qual).round() as u32,
    }
}

// -- Pet Families & Type Effectiveness --

/// Pet family (10 families, matching WoW pet battle types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PetFamily {
    Aquatic,
    Beast,
    Critter,
    Dragonkin,
    Elemental,
    Flying,
    Humanoid,
    Magic,
    Mechanical,
    Undead,
}

/// Damage multiplier for an attacking family against a defending family.
///
/// Returns 1.5 for strong, 0.67 for weak, 1.0 for neutral.
/// WoW pet battle type chart (simplified):
/// - Beast → Critter (strong), Flying (weak)
/// - Critter → Undead (strong), Beast (weak)
/// - Dragonkin → Magic (strong), Undead (weak)
/// - Elemental → Mechanical (strong), Critter (weak)
/// - Flying → Aquatic (strong), Dragonkin (weak)
/// - Humanoid → Dragonkin (strong), Beast (weak)
/// - Magic → Flying (strong), Mechanical (weak)
/// - Mechanical → Beast (strong), Elemental (weak)
/// - Aquatic → Elemental (strong), Magic (weak)
/// - Undead → Humanoid (strong), Aquatic (weak)
pub fn type_effectiveness(attacker: PetFamily, defender: PetFamily) -> f32 {
    use PetFamily::*;
    match (attacker, defender) {
        (Beast, Critter)
        | (Critter, Undead)
        | (Dragonkin, Magic)
        | (Elemental, Mechanical)
        | (Flying, Aquatic)
        | (Humanoid, Dragonkin)
        | (Magic, Flying)
        | (Mechanical, Beast)
        | (Aquatic, Elemental)
        | (Undead, Humanoid) => 1.5,

        (Beast, Flying)
        | (Critter, Beast)
        | (Dragonkin, Undead)
        | (Elemental, Critter)
        | (Flying, Dragonkin)
        | (Humanoid, Beast)
        | (Magic, Mechanical)
        | (Mechanical, Elemental)
        | (Aquatic, Magic)
        | (Undead, Aquatic) => 0.67,

        _ => 1.0,
    }
}

// -- Pet Abilities --

/// Maximum abilities a pet can know.
pub const MAX_ABILITIES: usize = 6;
/// Number of active ability slots in battle.
pub const ACTIVE_SLOTS: usize = 3;

/// A pet ability definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PetAbility {
    pub id: u32,
    pub name: String,
    /// Base damage (scaled by power in combat).
    pub base_damage: u32,
    /// Family type of this ability (determines type effectiveness).
    pub family: PetFamily,
    /// Cooldown in turns (0 = no cooldown).
    pub cooldown: u8,
}

/// A pet's ability loadout: 6 known abilities, 3 active slots.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AbilityLoadout {
    /// All known abilities (up to 6).
    pub known: Vec<PetAbility>,
    /// Indices into `known` for the 3 active battle slots.
    pub active_slots: [usize; ACTIVE_SLOTS],
}

impl AbilityLoadout {
    /// Get the ability in a given active slot (0–2).
    pub fn active_ability(&self, slot: u8) -> Option<&PetAbility> {
        let idx = *self.active_slots.get(slot as usize)?;
        self.known.get(idx)
    }
}

/// Calculate ability damage: `(base_damage + power) * type_effectiveness`.
pub fn ability_damage(
    ability: &PetAbility,
    attacker_power: u32,
    defender_family: PetFamily,
) -> u32 {
    let raw = ability.base_damage + attacker_power;
    let effectiveness = type_effectiveness(ability.family, defender_family);
    (raw as f32 * effectiveness).round() as u32
}

// -- Wild Pet Capture --

/// Base trap chance at full health.
const TRAP_BASE_CHANCE: f32 = 0.20;
/// Maximum trap chance at 1 HP.
const TRAP_MAX_CHANCE: f32 = 0.95;

/// Compute the capture chance for a wild pet based on its remaining health.
///
/// Scales linearly from `TRAP_BASE_CHANCE` (20%) at full health to
/// `TRAP_MAX_CHANCE` (95%) at near-zero health. A dead pet cannot be captured.
pub fn capture_chance(current_health: u32, max_health: u32) -> f32 {
    if current_health == 0 || max_health == 0 {
        return 0.0;
    }
    let health_pct = current_health as f32 / max_health as f32;
    let range = TRAP_MAX_CHANCE - TRAP_BASE_CHANCE;
    TRAP_BASE_CHANCE + range * (1.0 - health_pct)
}

/// Result of a trap attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureResult {
    /// Pet was captured successfully.
    Captured,
    /// Trap failed — pet dodged.
    Failed,
    /// Cannot trap — pet is dead.
    Dead,
    /// Cannot trap — not a wild pet battle.
    NotWild,
}

/// Attempt to capture the opponent's active pet.
///
/// `roll` is a random value in [0.0, 1.0). Capture succeeds if `roll < capture_chance`.
/// The battle ends immediately on successful capture.
pub fn attempt_capture(battle: &mut PetBattle, roll: f32) -> CaptureResult {
    if !battle.is_wild {
        return CaptureResult::NotWild;
    }
    if battle.phase != BattlePhase::SelectAction {
        return CaptureResult::Failed;
    }
    let target = battle.opponent.active_pet();
    if !target.is_alive() {
        return CaptureResult::Dead;
    }
    let chance = capture_chance(target.current_health, target.stats.health);
    if roll < chance {
        battle.phase = BattlePhase::Ended;
        battle.outcome = Some(BattleOutcome::PlayerWins);
        CaptureResult::Captured
    } else {
        CaptureResult::Failed
    }
}

// -- Battle System --

/// Number of pets per team in a battle.
pub const TEAM_SIZE: usize = 3;

/// A pet's state during a battle.
#[derive(Debug, Clone, PartialEq)]
pub struct BattlePet {
    pub stats: PetStats,
    pub family: PetFamily,
    pub abilities: AbilityLoadout,
    pub current_health: u32,
}

impl BattlePet {
    /// Create a battle pet with default (no abilities, Beast family).
    pub fn new(stats: PetStats) -> Self {
        Self {
            current_health: stats.health,
            stats,
            family: PetFamily::Beast,
            abilities: AbilityLoadout::default(),
        }
    }

    /// Create a battle pet with family and abilities.
    pub fn with_family(stats: PetStats, family: PetFamily, abilities: AbilityLoadout) -> Self {
        Self {
            current_health: stats.health,
            stats,
            family,
            abilities,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.current_health > 0
    }

    pub fn take_damage(&mut self, amount: u32) {
        self.current_health = self.current_health.saturating_sub(amount);
    }
}

/// Which side of the battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleSide {
    Player,
    Opponent,
}

/// An action selected for a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnAction {
    /// Use an ability (index into the pet's ability list, 0-2).
    UseAbility(u8),
    /// Swap to a different pet (index into the team, 0-2).
    Swap(u8),
    /// Pass the turn (do nothing).
    Pass,
}

/// Phase of the pet battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePhase {
    /// Waiting for both sides to choose actions.
    SelectAction,
    /// Battle is over.
    Ended,
}

/// Outcome of a pet battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleOutcome {
    PlayerWins,
    OpponentWins,
}

/// What happened in a single turn resolution step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnEvent {
    pub side: BattleSide,
    pub damage: u32,
    pub target_fainted: bool,
}

/// One side's team state.
#[derive(Debug, Clone, PartialEq)]
pub struct BattleTeam {
    pub pets: Vec<BattlePet>,
    pub active: usize,
    pub action: Option<TurnAction>,
}

impl BattleTeam {
    pub fn new(pets: Vec<BattlePet>) -> Self {
        Self {
            pets,
            active: 0,
            action: None,
        }
    }

    pub fn active_pet(&self) -> &BattlePet {
        &self.pets[self.active]
    }

    pub fn active_pet_mut(&mut self) -> &mut BattlePet {
        &mut self.pets[self.active]
    }

    /// Whether any pet on this team is still alive.
    pub fn has_alive(&self) -> bool {
        self.pets.iter().any(|p| p.is_alive())
    }

    /// Auto-swap to the next alive pet. Returns false if none alive.
    pub fn auto_swap_if_fainted(&mut self) -> bool {
        if self.active_pet().is_alive() {
            return true;
        }
        let next = self.pets.iter().position(|p| p.is_alive());
        match next {
            Some(idx) => {
                self.active = idx;
                true
            }
            None => false,
        }
    }
}

/// A pet battle instance (3v3 turn-based).
#[derive(Debug, Clone, PartialEq)]
pub struct PetBattle {
    pub player: BattleTeam,
    pub opponent: BattleTeam,
    pub phase: BattlePhase,
    pub turn: u32,
    pub outcome: Option<BattleOutcome>,
    /// Whether this is a wild pet battle (allows trapping).
    pub is_wild: bool,
}

impl PetBattle {
    /// Create a new PvP/NPC battle (not wild — no trapping).
    pub fn new(player_pets: Vec<BattlePet>, opponent_pets: Vec<BattlePet>) -> Self {
        Self {
            player: BattleTeam::new(player_pets),
            opponent: BattleTeam::new(opponent_pets),
            phase: BattlePhase::SelectAction,
            turn: 1,
            outcome: None,
            is_wild: false,
        }
    }

    /// Create a wild pet battle (allows trapping).
    pub fn new_wild(player_pets: Vec<BattlePet>, wild_pets: Vec<BattlePet>) -> Self {
        Self {
            player: BattleTeam::new(player_pets),
            opponent: BattleTeam::new(wild_pets),
            phase: BattlePhase::SelectAction,
            turn: 1,
            outcome: None,
            is_wild: true,
        }
    }

    /// Submit an action for one side. When both sides have acted, resolve the turn.
    pub fn submit_action(&mut self, side: BattleSide, action: TurnAction) -> Vec<TurnEvent> {
        if self.phase != BattlePhase::SelectAction {
            return vec![];
        }
        match side {
            BattleSide::Player => self.player.action = Some(action),
            BattleSide::Opponent => self.opponent.action = Some(action),
        }
        if self.player.action.is_some() && self.opponent.action.is_some() {
            return self.resolve_turn();
        }
        vec![]
    }

    fn resolve_turn(&mut self) -> Vec<TurnEvent> {
        let p_action = self.player.action.take().unwrap();
        let o_action = self.opponent.action.take().unwrap();

        let p_speed = self.player.active_pet().stats.speed;
        let o_speed = self.opponent.active_pet().stats.speed;
        let player_first = p_speed >= o_speed;

        let mut events = Vec::new();
        let (first_side, first_action, second_side, second_action) = if player_first {
            (BattleSide::Player, p_action, BattleSide::Opponent, o_action)
        } else {
            (BattleSide::Opponent, o_action, BattleSide::Player, p_action)
        };

        events.extend(self.execute_action(first_side, first_action));
        self.check_faints();
        if self.phase == BattlePhase::Ended {
            return events;
        }

        events.extend(self.execute_action(second_side, second_action));
        self.check_faints();

        self.turn += 1;
        events
    }

    fn execute_action(&mut self, side: BattleSide, action: TurnAction) -> Vec<TurnEvent> {
        match action {
            TurnAction::UseAbility(slot) => {
                let attacker = self.team(side).active_pet();
                let attacker_power = attacker.stats.power;
                let defender_family = self.team(side.opposite()).active_pet().family;

                let damage = match attacker.abilities.active_ability(slot) {
                    Some(ability) => ability_damage(ability, attacker_power, defender_family),
                    None => attacker_power, // fallback: raw power if no ability
                };

                let target = self.team_mut(side.opposite());
                target.active_pet_mut().take_damage(damage);
                let fainted = !target.active_pet().is_alive();
                vec![TurnEvent {
                    side,
                    damage,
                    target_fainted: fainted,
                }]
            }
            TurnAction::Swap(idx) => {
                let team = self.team_mut(side);
                let idx = idx as usize;
                if idx < team.pets.len() && team.pets[idx].is_alive() {
                    team.active = idx;
                }
                vec![]
            }
            TurnAction::Pass => vec![],
        }
    }

    fn check_faints(&mut self) {
        self.player.auto_swap_if_fainted();
        self.opponent.auto_swap_if_fainted();
        if !self.player.has_alive() {
            self.end(BattleOutcome::OpponentWins);
        } else if !self.opponent.has_alive() {
            self.end(BattleOutcome::PlayerWins);
        }
    }

    fn end(&mut self, outcome: BattleOutcome) {
        self.phase = BattlePhase::Ended;
        self.outcome = Some(outcome);
    }
    fn team(&self, side: BattleSide) -> &BattleTeam {
        match side {
            BattleSide::Player => &self.player,
            BattleSide::Opponent => &self.opponent,
        }
    }

    fn team_mut(&mut self, side: BattleSide) -> &mut BattleTeam {
        match side {
            BattleSide::Player => &mut self.player,
            BattleSide::Opponent => &mut self.opponent,
        }
    }
}

impl BattleSide {
    fn opposite(self) -> Self {
        match self {
            Self::Player => Self::Opponent,
            Self::Opponent => Self::Player,
        }
    }
}

#[cfg(test)]
#[path = "pet_battle_tests.rs"]
mod tests;
