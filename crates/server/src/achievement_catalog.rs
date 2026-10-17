//! Achievement/Criteria/CriteriaTree DB2 rows imported by scripts/import_achievements.py.
//! Unconditional character kill/level/quest/achievement/encounter criteria are enabled. Trees
//! containing unknown criteria, timers or world-state conditions stay disabled.
//! M+ admits only compiled pinned all-of completion predicates; unknown modifiers stay disabled.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use bevy::prelude::*;
use rusqlite::{Connection, OptionalExtension};
use shared::protocol::{
    AchievementProgressSnapshot, AchievementSnapshot, AchievementToastSnapshot,
};

use super::PlayerAchievementState;

/// Event counters add across an account's characters; absolute facts use the maximum.
const SUMMED_ACCOUNT_KINDS: &[u32] = &[
    0, 1, 31, 32, 33, 35, 113, 52, 53, 70, 78, 11, 12, 14, 15, 16, 17, 18, 20, 23, 26, 28, 29, 36,
    37, 41, 42, 59, 62, 63, 65, 67, 68, 69, 73, 66, 74, 76, 77, 80, 82, 84, 107, 108, 109, 110,
    114, 130, 151, 152, 157, 158, 160, 162, 165, 13, 54, 55, 56, 90, 91, 97, 103, 105, 216, 219,
    220, 221, 245, 246, 253,
];

// Retail Blizzard_FrameXMLBase/Constants.lua:123: ACHIEVEMENT_FLAGS_ACCOUNT.
// 0x00010000 is SHOW_CRITERIA_MEMBERS, not account scope.
const ACHIEVEMENT_FLAG_ACCOUNT: u32 = 0x0002_0000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CriteriaEvent {
    KillCreature(u32),
    /// Actual native NPC victim type; None cannot satisfy a typed predicate.
    KillAnyCreature(Option<u8>),
    ReachLevel(u8),
    ActivelyReachLevel(u8),
    ReachMaxLevel(u8),
    GainLevels(u8),
    SkillRank(u32, u16),
    /// TC GetPureMaxSkillValue of a known skill line.
    SkillMaxRank(u32, u16),
    CurrencyGained(u32, u32),
    ReputationGained(u32, u32),
    TotalExaltedFactions(u32),
    TotalReveredFactions(u32),
    TotalHonoredFactions(u32),
    AcquireItem(u32, u32),
    LootItem(u32, u32),
    /// One committed Retail acquisition/loot receipt, not its stack quantity.
    ObtainAnyItem(u32),
    LootAnyItem(u32),
    ParticipateInBattleground(u32),
    DieOnMap(u32),
    /// Native dungeon/raid copy maximum, not the number of players currently inside.
    DieInInstance(u32),
    DieAnywhere,
    KilledByCreature(u32),
    KilledByPlayer,
    DieFromEnvironmentalDamage(u32),
    /// TC HandleFall: current survived damaging fall distance in hundredths of a yard.
    FallWithoutDying(u32),
    /// TC client loot type (`crate::loot::LootKind`) and looted item count.
    GetLootByType(u8, u32),
    LootMoney(u64),
    MoneySpentOnPostage(u64),
    MostMoneyOwned(u64),
    MoneyEarnedFromAuctions(u64),
    AuctionPosted,
    HighestAuctionBid(u64),
    AuctionWon,
    HighestAuctionSale(u64),
    DamageDealt(u64),
    HighestDamageDone(u64),
    HighestDamageTaken(u64),
    TotalDamageTaken(u64),
    HealingDone(u64),
    HighestHealCast(u64),
    TotalHealReceived(u64),
    HighestHealReceived(u64),
    /// TextEmote ID and the same-map NPC entry resolved from this request's unit GUID.
    DoEmote(u32, Option<Entity>),
    DeliveredKillingBlow,
    /// One eligible native world-player kill, independent of its Honor XP amount.
    EarnHonorableKill,
    /// Actual victim race for an eligible HK recipient, not only the final attacker.
    HonorableKillRace(u8),
    /// Actual victim class for each eligible HK recipient (TC RewardHonor6754).
    HonorableKillClass(u8),
    /// Actual current terrain area of this eligible recipient; includes authored ancestors.
    HonorableKillInArea(u32),
    /// Absolute persisted character lifetime HK, read after the eligible kill increment or login.
    LifetimeHonorableKills(u64),
    /// One native battleground/arena kill reward; world honor eligibility is separate.
    KillPlayer,
    UseItem(u32),
    EquipItem(u32),
    UseGameObject(u32),
    KnowSkillLineSpells(u32, u32),
    LearnTradeskillSkillLine(u32, u32),
    KnowPetCreature(u32),
    OwnUniquePets(u32),
    LearnToy(u32),
    OwnToys(u32),
    WinPetBattle,
    PetBattleAchievementPoints(u32),
    CaptureBattlePet,
    /// TC BattlePetMgr::AddPet: one newly committed pet GUID of this species.
    LearnedNewPet(u32),
    /// TC BattlegroundScore::UpdatePvpStat: one native objective credit of this PvpStat.
    TrackedWorldStateUIModified(u32),
    BattlePetReachedLevel {
        species: u32,
        level: u8,
    },
    ActivelyEarnedPetLevel {
        species: u32,
        level: u8,
    },
    Login,
    ExploreAreaBit(u32),
    EnterTopLevelArea(u32),
    EnterArea(u32),
    LeaveArea(u32),
    TriggerGameEvent(u32),
    GainAura(u32),
    WinBattleground(u32),
    WinAnyBattleground,
    /// TC Arena.cpp249: winner's pre-result personal rating, minimum1.
    WinAnyRankedArena(u32),
    /// Current authored arena asset0/1 and actual post-settlement personal rating.
    PersonalArenaRating {
        bracket: u32,
        rating: u32,
    },
    /// Native rated result: map of the completed arena.
    WinArena(u32),
    ParticipateInArena(u32),
    CompleteMythicPlus(MythicCompletion),
    CompleteScenario(u32),
    CompleteAnyScenario,
    CompleteQuest(u32),
    CompleteDailyQuest,
    /// Positive copper paid by a committed quest reward.
    MoneyEarnedFromQuesting(u64),
    /// One committed reward of a quest whose QuestSortID is this zone.
    CompleteQuestInZone(u32),
    /// Login: counter-increasing rewarded quests in this zone.
    RewardedQuestsInZone(u32, u32),
    RewardedQuestCount(u32),
    AbandonQuest,
    /// Purchased tabs of TC BankType (0 Character, 2 Account).
    BankTabsOwned(u32, u32),
    LearnTaxiNode(u32),
    BuyTaxi,
    WinDuel,
    AcceptSummon,
    LoseDuel,
    GotHaircut,
    MoneySpentAtBarberShop(u64),
    BuyItemsFromVendors,
    FulfillAnyCraftingOrder,
    FulfillCraftingOrderType(u8),
    SellItemsToVendors,
    MoneyEarnedFromSales(u64),
    MoneySpentOnTaxis(u64),
    CastSpell(u32),
    BeSpellTarget(u32),
    /// Spell, actual hit Player identity, pre-effect health, and live hit entity.
    /// Consumers read hit-unit auras synchronously before effect dispatch; never persisted.
    LandTargetedSpell(u32, Option<SpellHitPlayer>, Option<f32>, Option<Entity>),
    LearnSpell(u32),
    DefeatEncounter(u32),
    /// One DONE while the character's active encounter loot-lock bit was unset.
    DefeatEncounterEligibleForLoot(u32),
}

impl CriteriaEvent {
    fn kind(self) -> u32 {
        match self {
            Self::KillCreature(_) => 0,
            Self::KillAnyCreature(_) => 78,
            Self::WinAnyRankedArena(_) => 37,
            Self::PersonalArenaRating { .. } => 39,
            Self::WinArena(_) => 32,
            Self::ParticipateInArena(_) => 33,
            Self::ReachLevel(_) => 5,
            Self::ActivelyReachLevel(_) => 196,
            Self::ReachMaxLevel(_) => 221,
            Self::GainLevels(_) => 253,
            Self::SkillRank(_, _) => 7,
            Self::SkillMaxRank(_, _) => 40,
            Self::CurrencyGained(_, _) => 12,
            Self::ReputationGained(_, _) => 46,
            Self::TotalExaltedFactions(_) => 47,
            Self::TotalReveredFactions(_) => 87,
            Self::TotalHonoredFactions(_) => 88,
            Self::AcquireItem(_, _) => 36,
            Self::LootItem(_, _) => 42,
            Self::ParticipateInBattleground(_) => 15,
            Self::DieOnMap(_) => 16,
            Self::DieAnywhere => 17,
            Self::DieInInstance(_) => 18,
            Self::KilledByCreature(_) => 20,
            Self::KilledByPlayer => 23,
            Self::DieFromEnvironmentalDamage(_) => 26,
            Self::FallWithoutDying(_) => 24,
            Self::ObtainAnyItem(_) => 91,
            Self::LootAnyItem(_) => 90,
            Self::GetLootByType(_, _) => 109,
            Self::LootMoney(_) => 67,
            Self::MoneySpentOnPostage(_) => 66,
            Self::MostMoneyOwned(_) => 86,
            Self::MoneyEarnedFromAuctions(_) => 80,
            Self::AuctionPosted => 82,
            Self::HighestAuctionBid(_) => 83,
            Self::AuctionWon => 84,
            Self::HighestAuctionSale(_) => 85,
            Self::DamageDealt(_) => 13,
            Self::HighestDamageDone(_) => 101,
            Self::HighestDamageTaken(_) => 102,
            Self::TotalDamageTaken(_) => 103,
            Self::HealingDone(_) => 55,
            Self::HighestHealCast(_) => 104,
            Self::TotalHealReceived(_) => 105,
            Self::HighestHealReceived(_) => 106,
            Self::DoEmote(_, _) => 54,
            Self::DeliveredKillingBlow => 56,
            Self::KillPlayer => 70,
            Self::EarnHonorableKill => 35,
            Self::HonorableKillRace(_) => 53,
            Self::HonorableKillClass(_) => 52,
            Self::HonorableKillInArea(_) => 31,
            Self::LifetimeHonorableKills(_) => 113,
            Self::UseItem(_) => 41,
            Self::EquipItem(_) => 57,
            Self::UseGameObject(_) => 68,
            Self::KnowSkillLineSpells(_, _) => 75,
            Self::LearnTradeskillSkillLine(_, _) => 112,
            Self::KnowPetCreature(_) => 96,
            Self::OwnUniquePets(_) => 156,
            Self::LearnToy(_) => 185,
            Self::OwnToys(_) => 186,
            Self::WinPetBattle => 158,
            Self::PetBattleAchievementPoints(_) => 81,
            Self::CaptureBattlePet => 157,
            Self::LearnedNewPet(_) => 155,
            Self::TrackedWorldStateUIModified(_) => 30,
            Self::BattlePetReachedLevel { .. } => 160,
            Self::ActivelyEarnedPetLevel { .. } => 162,
            Self::Login => 74,
            Self::ExploreAreaBit(_) => 43,
            Self::EnterTopLevelArea(_) => 225,
            Self::EnterArea(_) => 163,
            Self::LeaveArea(_) => 164,
            Self::TriggerGameEvent(_) => 73,
            Self::GainAura(_) => 69,
            Self::WinBattleground(_) => 1,
            Self::WinAnyBattleground => 130,
            Self::CompleteMythicPlus(_) => 216,
            Self::CompleteScenario(_) => 152,
            Self::CompleteAnyScenario => 151,
            Self::CompleteQuest(_) => 27,
            Self::CompleteDailyQuest => 14,
            Self::MoneyEarnedFromQuesting(_) => 62,
            Self::CompleteQuestInZone(_) | Self::RewardedQuestsInZone(_, _) => 11,
            Self::RewardedQuestCount(_) => 9,
            Self::AbandonQuest => 107,
            Self::BankTabsOwned(_, _) => 260,
            Self::LearnTaxiNode(_) => 262,
            Self::BuyTaxi => 108,
            Self::WinDuel => 76,
            Self::AcceptSummon => 114,
            Self::LoseDuel => 77,
            Self::GotHaircut => 48,
            Self::MoneySpentAtBarberShop(_) => 65,
            Self::BuyItemsFromVendors => 219,
            Self::FulfillAnyCraftingOrder => 245,
            Self::FulfillCraftingOrderType(_) => 246,
            Self::SellItemsToVendors => 220,
            Self::MoneyEarnedFromSales(_) => 59,
            Self::MoneySpentOnTaxis(_) => 63,
            Self::CastSpell(_) => 29,
            Self::BeSpellTarget(_) => 28,
            Self::LandTargetedSpell(_, _, _, _) => 110,
            Self::LearnSpell(_) => 34,
            Self::DefeatEncounter(_) => 165,
            Self::DefeatEncounterEligibleForLoot(_) => 97,
        }
    }

    fn matches(self, criteria: &Criteria) -> bool {
        if criteria.kind != self.kind() {
            return false;
        }
        match self {
            Self::ReachLevel(_) => true,
            Self::HonorableKillRace(race) => race != 0 && u32::from(race) == criteria.asset,
            Self::HonorableKillClass(class) => class != 0 && u32::from(class) == criteria.asset,
            Self::FallWithoutDying(distance) => distance > 0 && criteria.asset == 0,
            Self::ObtainAnyItem(item) | Self::LootAnyItem(item) => item > 0 && criteria.asset == 0,
            Self::WinArena(map) | Self::ParticipateInArena(map) => criteria.asset == map,
            Self::WinAnyRankedArena(rating) => rating > 0 && criteria.asset == 0,
            Self::PersonalArenaRating { bracket, rating } => {
                rating > 0 && criteria.asset == bracket
            }
            Self::ActivelyReachLevel(level) => u32::from(level) == criteria.asset,
            Self::BattlePetReachedLevel { species, level }
            | Self::ActivelyEarnedPetLevel { species, level } => {
                species != 0
                    && u32::from(level) == criteria.asset
                    && criteria
                        .modifier_asset
                        .is_none_or(|required| required == species)
            }
            Self::ExploreAreaBit(bit) => criteria.exploration_bits.contains(&bit),
            Self::EnterArea(area) | Self::HonorableKillInArea(area) => {
                area != 0 && criteria.area_members.contains(&area)
            }
            Self::LeaveArea(area) => area != 0 && !criteria.area_members.contains(&area),
            Self::GetLootByType(kind, _) => u32::from(kind) == criteria.asset,
            Self::FulfillCraftingOrderType(kind) => u32::from(kind) == criteria.asset,
            Self::ReachMaxLevel(class) => {
                criteria.asset == 0
                    && criteria
                        .modifier_asset
                        .is_none_or(|required| required == u32::from(class))
            }
            Self::LearnedNewPet(species) => {
                criteria.asset == 0 && criteria.modifier_asset == Some(species)
            }
            Self::KillAnyCreature(_)
            | Self::DeliveredKillingBlow
            | Self::EarnHonorableKill
            | Self::LifetimeHonorableKills(_)
            | Self::TotalExaltedFactions(_)
            | Self::TotalReveredFactions(_)
            | Self::TotalHonoredFactions(_)
            | Self::GainLevels(_)
            | Self::LootMoney(_)
            | Self::DamageDealt(_)
            | Self::HighestDamageDone(_)
            | Self::HighestDamageTaken(_)
            | Self::TotalDamageTaken(_)
            | Self::HealingDone(_)
            | Self::HighestHealCast(_)
            | Self::TotalHealReceived(_)
            | Self::HighestHealReceived(_)
            | Self::MoneySpentOnPostage(_)
            | Self::MostMoneyOwned(_)
            | Self::MoneyEarnedFromAuctions(_)
            | Self::AuctionPosted
            | Self::HighestAuctionBid(_)
            | Self::AuctionWon
            | Self::HighestAuctionSale(_)
            | Self::DieAnywhere
            | Self::KilledByPlayer
            | Self::KillPlayer
            | Self::CompleteDailyQuest
            | Self::MoneyEarnedFromQuesting(_)
            | Self::RewardedQuestCount(_)
            | Self::AbandonQuest
            | Self::BuyTaxi
            | Self::WinDuel
            | Self::AcceptSummon
            | Self::LoseDuel
            | Self::GotHaircut
            | Self::MoneySpentAtBarberShop(_)
            | Self::BuyItemsFromVendors
            | Self::FulfillAnyCraftingOrder
            | Self::SellItemsToVendors
            | Self::MoneyEarnedFromSales(_)
            | Self::MoneySpentOnTaxis(_)
            | Self::OwnUniquePets(_)
            | Self::OwnToys(_)
            | Self::WinPetBattle
            | Self::PetBattleAchievementPoints(_)
            | Self::CaptureBattlePet
            | Self::Login
            | Self::CompleteAnyScenario
            | Self::WinAnyBattleground => criteria.asset == 0,
            Self::CompleteMythicPlus(completion) => criteria
                .mythic
                .is_some_and(|requirements| requirements.matches(completion)),
            Self::SkillRank(asset, _)
            | Self::SkillMaxRank(asset, _)
            | Self::CurrencyGained(asset, _)
            | Self::ReputationGained(asset, _)
            | Self::AcquireItem(asset, _)
            | Self::LootItem(asset, _)
            | Self::KnowSkillLineSpells(asset, _)
            | Self::LearnTradeskillSkillLine(asset, _) => asset == criteria.asset,
            Self::KillCreature(asset)
            | Self::CompleteQuest(asset)
            | Self::CastSpell(asset)
            | Self::BeSpellTarget(asset)
            | Self::LandTargetedSpell(asset, _, _, _)
            | Self::LearnSpell(asset)
            | Self::TriggerGameEvent(asset)
            | Self::GainAura(asset)
            | Self::WinBattleground(asset)
            | Self::ParticipateInBattleground(asset)
            | Self::DieOnMap(asset)
            | Self::DieInInstance(asset)
            | Self::TrackedWorldStateUIModified(asset)
            | Self::KilledByCreature(asset)
            | Self::DieFromEnvironmentalDamage(asset)
            | Self::DoEmote(asset, _)
            | Self::UseItem(asset)
            | Self::EquipItem(asset)
            | Self::UseGameObject(asset)
            | Self::KnowPetCreature(asset)
            | Self::LearnToy(asset)
            | Self::CompleteScenario(asset)
            | Self::CompleteQuestInZone(asset)
            | Self::EnterTopLevelArea(asset)
            | Self::LearnTaxiNode(asset)
            | Self::RewardedQuestsInZone(asset, _)
            | Self::BankTabsOwned(asset, _)
            | Self::DefeatEncounter(asset)
            | Self::DefeatEncounterEligibleForLoot(asset) => asset == criteria.asset,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MythicCompletion {
    pub key_level: u32,
    pub challenge_map: u32,
    pub player_level: u8,
    pub in_time: bool,
}

#[derive(Debug, Clone, Copy)]
struct MythicRequirements {
    min_level: u32,
    challenge_map: Option<u32>,
    player_level: Option<u32>,
    in_time: bool,
}

impl MythicRequirements {
    fn matches(self, completion: MythicCompletion) -> bool {
        let level_matches = completion.key_level >= self.min_level;
        let map_matches = self
            .challenge_map
            .is_none_or(|map| map == completion.challenge_map);
        let player_matches = self
            .player_level
            .is_none_or(|level| level == u32::from(completion.player_level));
        let time_matches = !self.in_time || completion.in_time;
        level_matches && map_matches && player_matches && time_matches
    }
}

#[path = "achievement_conditions.rs"]
mod conditions;
#[path = "achievement_mythic_predicates.rs"]
mod mythic_predicates;
#[path = "achievement_node_counts.rs"]
mod node_counts;
#[path = "achievement_overlays.rs"]
mod overlays;
#[path = "achievement_single_modifiers.rs"]
mod single_modifiers;

/// Compiled TC DungeonDifficulty68/LegacyDungeonDifficulty20 tree: the player's current
/// map difficulty must (`allowed`) or must not be one of `ids`.
#[derive(Debug, Clone, Copy)]
struct DifficultyRule {
    allowed: bool,
    ids: &'static [u32],
}

impl DifficultyRule {
    fn admits(self, difficulty: u32) -> bool {
        self.ids.contains(&difficulty) == self.allowed
    }
}

/// Compiled All facts or a plain Some of complete target-creature/map/area/aura alternatives.
#[derive(Debug, Clone, Copy)]
enum CompiledConditions {
    All {
        difficulty: Option<DifficultyRule>,
        // Negative compiled kinds denote bounded SingleFalse leaves, never raw DB2 types.
        facts: &'static [(i32, u32)],
    },
    TargetCreatures(&'static [u32]),
    PlayerMaps(&'static [u32]),
    PlayerLocations(&'static [(i32, u32)]),
    PlayerAuras(&'static [u32]),
    EventItems(&'static [u32]),
}

/// TC ModifierTree facts at a criteria update.
#[derive(Debug, Default)]
pub(crate) struct PlayerConditions {
    /// Difficulty.db2 ID of the current map copy; 0 on continents.
    pub difficulty: u32,
    /// None for map-wide credit, which has no reference player.
    pub player: Option<ReferencePlayer>,
    /// Actual native spell-hit or requested emote units; never persisted.
    pub hit_units: BTreeMap<Entity, CriteriaTargetUnit>,
}

/// Separate actual unit authority: neither NPC entry nor auras imply Player identity.
#[derive(Debug, Default)]
pub(crate) struct CriteriaTargetUnit {
    pub creature_entry: Option<u32>,
    pub auras: Option<BTreeSet<u32>>,
    pub is_player: bool,
    pub dead: Option<bool>,
    pub hostile: Option<bool>,
}

/// Identity of the actual Player hit by this spell; NPC/missing targets have no snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpellHitPlayer {
    pub race: u8,
    pub class: u8,
}

/// The reference player's own state read by TC player-only ModifierTree leaves.
#[derive(Debug, Default)]
pub(crate) struct ReferencePlayer {
    /// Spell IDs of active auras.
    pub auras: BTreeSet<u32>,
    /// Missing ActiveAuras is unknown, not proof that an aura is absent.
    pub auras_known: bool,
    /// IDs in existing class/profession knowledge and the owning mount journal.
    pub known_spells: BTreeSet<u32>,
    /// Absence is meaningful only after class, profession and account mount sources are complete.
    pub spell_knowledge_complete: bool,
    /// CompleteQuest facts; Login74 reconstructs these from this character's persisted rewards.
    pub rewarded_quests: BTreeSet<u32>,
    /// This character's earned IDs plus account-wide awards; None when state/catalog is unknown.
    pub earned_achievements: Option<BTreeSet<u32>>,
    /// Negative87 requires the account aggregate too; character-only state is incomplete.
    pub account_achievements_known: bool,
    /// Minimum owned pet level across all three equipped journal slots; None unknown/incomplete.
    pub battle_pet_team_min_level: Option<u8>,
    /// Actual current authored ScenarioStep ID; None outside/after a scenario or without authority.
    pub scenario_step: Option<u32>,
    /// Current terrain area (TC Player::GetAreaId).
    pub area: Option<u32>,
    pub map: u32,
    pub race: u8,
    pub class: u8,
    pub level: u8,
    /// Full group roster, including offline members;0 means known solo, None unknown.
    pub group_size: Option<usize>,
    pub arena_team_size: Option<u8>,
    pub in_ranked_arena: bool,
    /// Live accepted node-copy Alliance/Horde counts; None is missing authority, not zero.
    pub controlled_node_counts: Option<[u8; 2]>,
    /// None when health authority is absent; positive health and native life state agree.
    pub alive: Option<bool>,
}

/// A compiled ModifierTree leaf resolved at catalog load.
#[derive(Debug, Clone)]
enum CriteriaFact {
    /// Bounded native PlayerCondition2 class/race rule; all other fields reject at load.
    Identity(crate::game_objects::identity_conditions::IdentityCondition),
    Aura(u32),
    NotAura(u32),
    KnownSpell(u32),
    NotKnownSpell(u32),
    CompletedQuest(u32),
    /// Authored step IDs whose OrderIndex equals the one-based modifier asset minus one.
    ScenarioSteps(BTreeSet<u32>),
    EarnedAchievement(u32),
    NotEarnedAchievement(u32),
    BattlePetTeamLevel(u8),
    /// Resolved species IDs whose authored PetTypeEnum matches the required type.
    BattlePetSpeciesOfType(BTreeSet<u32>),
    /// Some8 requiring one of these complete, authenticated PlayerHasAura8 spells.
    PlayerAuras(BTreeSet<u32>),
    /// ItemId19 or item-template quality14/15 against this native receipt's itemID.
    EventItems(BTreeSet<u32>),
    /// TargetCreatureId4: actual text-emote target or pre-effect spell-hit NPC entry.
    TargetCreature(u32),
    /// CreatureType30 on kill78 reads the actual rewarded NPC victim.
    KilledCreatureType(u8),
    /// TargetRace27 from the actual hit unit, never the reference player.
    TargetRace(u8),
    TargetIsPlayer,
    TargetIsDead,
    TargetIsHostile,
    /// TC TargetHasAura10 on the actual hit unit, independently of Player identity.
    TargetAura(u32),
    /// Accepted only with positive player/race authority; NPC class predicates stay disabled.
    TargetClass(u8),
    /// Inclusive TC TargetHealthBelowPercent46 against the pre-effect hit unit.
    TargetHealthBelowPercent(u8),
    /// Some8 requiring one of these complete TargetCreatureId4 leaves.
    TargetCreatures(BTreeSet<u32>),
    ArenaTeamSize(u8),
    RankedArena,
    Alive,
    /// TC LessThanTappers29 actually compares Group::GetMembersCount, not nearby tappers.
    LessThanGroupMembers(u32),
    NotInParty,
    EitherTeamNodeCount(u8),
    /// Accepted current areas: descendants for17, one flagged parent for Zone41.
    Area(BTreeSet<u32>),
    Race(u8),
    Class(u8),
    Map(u32),
    /// Some8 requiring one of these complete, authenticated PlayerMap32 leaves.
    PlayerMaps(BTreeSet<u32>),
    /// Authenticated Map.InstanceType members, compared by raw type ID.
    MapInstanceType(BTreeSet<u32>),
    MinLevel(u8),
    MaxLevel(u8),
    /// TC268 uses ContentTuning min/max, never MinLevelWithDelta.
    ContentTuningLevels(std::ops::RangeInclusive<u8>),
}

fn event_creature_entry(
    event: CriteriaEvent,
    hit_units: &BTreeMap<Entity, CriteriaTargetUnit>,
) -> Option<u32> {
    match event {
        CriteriaEvent::DoEmote(_, Some(target))
        | CriteriaEvent::LandTargetedSpell(_, _, _, Some(target)) => {
            hit_units.get(&target).and_then(|unit| unit.creature_entry)
        }
        _ => None,
    }
}

impl CriteriaFact {
    fn holds(
        &self,
        player: &ReferencePlayer,
        event: CriteriaEvent,
        hit_units: &BTreeMap<Entity, CriteriaTargetUnit>,
    ) -> bool {
        match self {
            Self::Identity(identity) => identity.permits_race_class(player.race, player.class),
            Self::Aura(spell) => player.auras.contains(spell),
            Self::NotAura(spell) => player.auras_known && !player.auras.contains(spell),
            Self::KnownSpell(spell) => player.known_spells.contains(spell),
            Self::CompletedQuest(quest) => player.rewarded_quests.contains(quest),
            Self::ScenarioSteps(steps) => player
                .scenario_step
                .is_some_and(|step| steps.contains(&step)),
            Self::NotKnownSpell(spell) => {
                player.spell_knowledge_complete && !player.known_spells.contains(spell)
            }
            Self::EarnedAchievement(achievement) => player
                .earned_achievements
                .as_ref()
                .is_some_and(|earned| earned.contains(achievement)),
            Self::NotEarnedAchievement(achievement) => {
                player.account_achievements_known
                    && player
                        .earned_achievements
                        .as_ref()
                        .is_some_and(|earned| !earned.contains(achievement))
            }
            Self::BattlePetTeamLevel(level) => player
                .battle_pet_team_min_level
                .is_some_and(|minimum| minimum >= *level),
            Self::PlayerAuras(spells) => !player.auras.is_disjoint(spells),
            Self::EventItems(items) => match event {
                CriteriaEvent::ObtainAnyItem(item) | CriteriaEvent::LootAnyItem(item) => {
                    items.contains(&item)
                }
                _ => false,
            },
            Self::TargetCreature(entry) => event_creature_entry(event, hit_units) == Some(*entry),
            Self::TargetRace(race) => {
                matches!(event, CriteriaEvent::LandTargetedSpell(_, Some(target), _, _) if target.race == *race)
            }
            Self::TargetIsPlayer => match event {
                CriteriaEvent::DoEmote(_, Some(target)) => {
                    hit_units.get(&target).is_some_and(|unit| unit.is_player)
                }
                CriteriaEvent::LandTargetedSpell(_, Some(_), _, _) => true,
                _ => false,
            },
            Self::TargetIsDead => matches!(event, CriteriaEvent::DoEmote(_, Some(target))
                if hit_units.get(&target).and_then(|unit| unit.dead) == Some(true)),
            Self::TargetIsHostile => matches!(event, CriteriaEvent::DoEmote(_, Some(target))
                if hit_units.get(&target).and_then(|unit| unit.hostile) == Some(true)),
            Self::TargetAura(spell) => {
                matches!(event, CriteriaEvent::LandTargetedSpell(_, _, _, Some(target))
                    if hit_units.get(&target).and_then(|unit| unit.auras.as_ref())
                        .is_some_and(|auras| auras.contains(spell)))
            }
            Self::TargetClass(class) => {
                matches!(event, CriteriaEvent::LandTargetedSpell(_, Some(target), _, _) if target.class == *class)
            }
            Self::TargetHealthBelowPercent(limit) => {
                matches!(event, CriteriaEvent::LandTargetedSpell(_, _, Some(percent), _) if percent <= f32::from(*limit))
            }
            Self::TargetCreatures(entries) => {
                event_creature_entry(event, hit_units).is_some_and(|entry| entries.contains(&entry))
            }
            Self::ArenaTeamSize(size) => player.arena_team_size == Some(*size),
            Self::RankedArena => player.in_ranked_arena,
            Self::Alive => player.alive == Some(true),
            Self::Area(members) => player.area.is_some_and(|area| members.contains(&area)),
            Self::Race(race) => player.race == *race,
            Self::Class(class) => player.class == *class,
            Self::LessThanGroupMembers(limit) => {
                player.group_size.is_some_and(|size| size < *limit as usize)
            }
            Self::NotInParty => player.group_size == Some(0),
            Self::BattlePetSpeciesOfType(species) => {
                matches!(event, CriteriaEvent::BattlePetReachedLevel { species: id, .. } if species.contains(&id))
            }
            Self::KilledCreatureType(required) => {
                matches!(event, CriteriaEvent::KillAnyCreature(Some(kind)) if kind == *required)
            }
            Self::EitherTeamNodeCount(required) => player
                .controlled_node_counts
                .is_some_and(|counts| counts.contains(required)),
            Self::Map(map) => player.map == *map,
            Self::PlayerMaps(maps) => maps.contains(&player.map),
            Self::MapInstanceType(maps) => maps.contains(&player.map),
            Self::MinLevel(level) => player.level >= *level,
            Self::MaxLevel(level) => player.level <= *level,
            Self::ContentTuningLevels(range) => range.contains(&player.level),
        }
    }
}

#[derive(Debug, Clone)]
struct Criteria {
    kind: u32,
    asset: u32,
    supported: bool,
    mythic: Option<MythicRequirements>,
    /// Compiled sole PlayerClass26/BattlePetSpecies91 requirement.
    modifier_asset: Option<u32>,
    difficulty: Option<DifficultyRule>,
    /// Compiled All4 reference-player or emote-target leaves; all must hold.
    facts: Vec<CriteriaFact>,
    exploration_bits: Vec<u32>,
    area_members: BTreeSet<u32>,
}

impl Criteria {
    fn conditions_hold(&self, conditions: &PlayerConditions, event: CriteriaEvent) -> bool {
        self.difficulty
            .is_none_or(|rule| rule.admits(conditions.difficulty))
            && (self.facts.is_empty()
                || conditions.player.as_ref().is_some_and(|player| {
                    self.facts
                        .iter()
                        .all(|fact| fact.holds(player, event, &conditions.hit_units))
                }))
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum TreeOperator {
    Single,
    All,
    SumChildren,
    MaxChild,
    CountDirectChildren,
    Any,
    SumChildrenWeight,
    Unsupported,
}

impl TreeOperator {
    fn from_db2(value: u32) -> Self {
        match value {
            0 => Self::Single,
            4 => Self::All,
            5 => Self::SumChildren,
            6 => Self::MaxChild,
            7 => Self::CountDirectChildren,
            8 => Self::Any,
            9 => Self::SumChildrenWeight,
            _ => Self::Unsupported,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CriteriaTree {
    pub criteria_id: u32,
    pub amount: u64,
    pub operator: TreeOperator,
}

#[derive(Debug, Clone)]
struct AchievementDef {
    id: u32,
    name: String,
    points: u32,
    root: u32,
    map_id: Option<u32>,
    faction: i32,
}

#[path = "achievement_evaluation.rs"]
mod evaluation;
pub(crate) use evaluation::EvaluationCache;

#[cfg(test)]
thread_local! {
    static EVALUATION_WORK: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}

#[derive(Resource, Debug, Default)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct AchievementCatalog {
    window: window::WindowData,
    definitions: BTreeMap<u32, AchievementDef>,
    criteria: HashMap<u32, Criteria>,
    pub trees: HashMap<u32, CriteriaTree>,
    pub children: HashMap<u32, Vec<u32>>,
    event_criteria: HashMap<u32, Vec<u32>>,
    evaluation_index: evaluation::EvaluationIndex,
    faction_groups: HashMap<u32, u32>,
    area_bits: HashMap<u32, u32>,
    pet_creatures: BTreeSet<u32>,
    pet_species: BTreeSet<u32>,
    spell_skill_lines: HashMap<u32, BTreeSet<u32>>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TreeProgress {
    pub current: u64,
    pub required: u64,
    pub completed: bool,
}

impl AchievementCatalog {
    /// TC CriteriaHandler.cpp718-740 counts each known spell once per authored skill line.
    pub(super) fn known_spell_events(
        &self,
        spells: impl Iterator<Item = u32>,
    ) -> Vec<CriteriaEvent> {
        let known: BTreeSet<_> = spells.collect();
        let mut counts = HashMap::<u32, u32>::new();
        for spell in &known {
            for &skill in self.spell_skill_lines.get(spell).into_iter().flatten() {
                *counts.entry(skill).or_default() += 1;
            }
        }
        let skills: BTreeSet<_> = [75, 112]
            .into_iter()
            .flat_map(|kind| self.event_criteria.get(&kind).into_iter().flatten())
            .map(|id| self.criteria[id].asset)
            .collect();
        known
            .into_iter()
            .map(CriteriaEvent::LearnSpell)
            .chain(skills.into_iter().flat_map(|skill| {
                let count = counts.get(&skill).copied().unwrap_or(0);
                [
                    CriteriaEvent::KnowSkillLineSpells(skill, count),
                    CriteriaEvent::LearnTradeskillSkillLine(skill, count),
                ]
            }))
            .collect()
    }

    pub(super) fn area_bit(&self, area: u32) -> Option<u32> {
        self.area_bits.get(&area).copied()
    }

    pub(super) fn tracks(&self, event: CriteriaEvent) -> bool {
        self.event_criteria
            .get(&event.kind())
            .into_iter()
            .flatten()
            .any(|id| event.matches(&self.criteria[id]))
    }

    /// An event outside any instance copy (Difficulty 0).
    #[cfg(test)]
    pub fn update(
        &self,
        state: &mut PlayerAchievementState,
        event: CriteriaEvent,
        completed_map: Option<u32>,
        race: u8,
    ) -> Vec<AchievementToastSnapshot> {
        self.update_at(
            state,
            event,
            completed_map,
            race,
            &PlayerConditions::default(),
        )
    }

    /// Apply one event by a player with `conditions`, and return newly earned toasts. Map achievements are granted
    /// only at the complete-run boundary, not on a partial dungeon's boss kill.
    pub fn update_at(
        &self,
        state: &mut PlayerAchievementState,
        event: CriteriaEvent,
        completed_map: Option<u32>,
        race: u8,
        conditions: &PlayerConditions,
    ) -> Vec<AchievementToastSnapshot> {
        if !state.signed_reputation {
            state
                .criteria_progress
                .retain(|id, _| self.criteria.get(id).is_none_or(|row| row.kind != 46));
            state.signed_reputation = true;
        }
        for &id in self.event_criteria.get(&event.kind()).into_iter().flatten() {
            let criteria = &self.criteria[&id];
            if !event.matches(criteria) || !criteria.conditions_hold(conditions, event) {
                continue;
            }
            Self::update_counter(state, id, event);
        }
        self.grant_completed(state, completed_map, race, false)
    }

    pub(crate) fn iter_earned_achievement_ids<'a>(
        &'a self,
        state: &'a PlayerAchievementState,
    ) -> impl Iterator<Item = u32> + 'a {
        let character = state
            .earned_ids
            .iter()
            .filter(|id| !self.window.has_account_flag(**id))
            .copied();
        let account = state
            .account
            .iter()
            .flat_map(|account| account.earned_ids.iter().copied())
            .filter(|id| self.window.has_account_flag(*id));
        character.chain(account)
    }

    pub fn snapshot(&self, state: &PlayerAchievementState, race: u8) -> AchievementSnapshot {
        AchievementSnapshot {
            earned_ids: self.iter_earned_achievement_ids(state).collect(),
            progress: self.progress_snapshot(state, race),
        }
    }

    fn progress_snapshot(
        &self,
        state: &PlayerAchievementState,
        race: u8,
    ) -> Vec<AchievementProgressSnapshot> {
        let character_progress = self.cached_progress(state);
        let account_progress = state
            .account
            .as_deref()
            .and_then(|account| self.cached_progress(account));
        self.definitions
            .values()
            .filter(|definition| self.faction_matches(definition.faction, race))
            .filter(|definition| {
                !self.window.has_account_flag(definition.id) || state.account.is_some()
            })
            .map(|definition| {
                let cached = if self.window.has_account_flag(definition.id) {
                    account_progress
                } else {
                    character_progress
                };
                let state = self.definition_state(state, definition.id);
                let progress = cached
                    .map(|progress| progress[&definition.id].clone())
                    .unwrap_or_else(|| self.tree_progress(definition.root, state));
                AchievementProgressSnapshot {
                    achievement_id: definition.id,
                    current: progress.current.min(u64::from(u32::MAX)) as u32,
                    required: progress.required.min(u64::from(u32::MAX)) as u32,
                    completed: state.earned_ids.contains(&definition.id),
                }
            })
            .collect()
    }

    pub(super) fn definition_state<'a>(
        &self,
        state: &'a PlayerAchievementState,
        id: u32,
    ) -> &'a PlayerAchievementState {
        if self.window.has_account_flag(id) {
            static EMPTY: std::sync::LazyLock<PlayerAchievementState> =
                std::sync::LazyLock::new(PlayerAchievementState::default);
            state.account.as_deref().unwrap_or(&EMPTY)
        } else {
            state
        }
    }

    /// Account counters are rebuilt from durable per-character contributions.
    /// Absolute facts (levels/quests/ranks/exploration/reputation) use max;
    /// event counters (kills/casts/targets/encounters) add across characters.
    pub(crate) fn aggregate(
        &self,
        contributions: impl Iterator<Item = PlayerAchievementState>,
    ) -> PlayerAchievementState {
        let mut aggregate = PlayerAchievementState {
            signed_reputation: true,
            ..Default::default()
        };
        for contribution in contributions {
            aggregate.earned_ids.extend(contribution.earned_ids);
            for (id, date) in contribution.earned_at {
                aggregate
                    .earned_at
                    .entry(id)
                    .and_modify(|existing| *existing = (*existing).min(date))
                    .or_insert(date);
            }
            for (id, value) in contribution.criteria_progress {
                // Account-journal quantities come from the owning journal, not stale character facts.
                if self
                    .criteria
                    .get(&id)
                    .is_some_and(|row| matches!(row.kind, 156 | 185 | 186))
                {
                    continue;
                }
                if !contribution.signed_reputation
                    && self.criteria.get(&id).is_some_and(|row| row.kind == 46)
                {
                    continue;
                }
                let counter = aggregate.criteria_progress.entry(id).or_default();
                *counter = if self
                    .criteria
                    .get(&id)
                    .is_some_and(|row| SUMMED_ACCOUNT_KINDS.contains(&row.kind))
                {
                    counter.saturating_add(value)
                } else {
                    (*counter).max(value)
                };
            }
        }
        aggregate
    }

    fn update_counter(state: &mut PlayerAchievementState, id: u32, event: CriteriaEvent) {
        // TC SetCriteriaProgress does not create a first zero-valued progress row.
        if matches!(
            event,
            CriteriaEvent::LifetimeHonorableKills(0)
                | CriteriaEvent::KnowSkillLineSpells(_, 0)
                | CriteriaEvent::LearnTradeskillSkillLine(_, 0)
                | CriteriaEvent::OwnUniquePets(0)
                | CriteriaEvent::OwnToys(0)
                | CriteriaEvent::TotalExaltedFactions(0)
                | CriteriaEvent::TotalReveredFactions(0)
                | CriteriaEvent::TotalHonoredFactions(0)
                | CriteriaEvent::RewardedQuestCount(0)
                | CriteriaEvent::RewardedQuestsInZone(_, 0)
                | CriteriaEvent::BankTabsOwned(_, 0)
                | CriteriaEvent::SkillMaxRank(_, 0)
                | CriteriaEvent::PetBattleAchievementPoints(0)
                | CriteriaEvent::MostMoneyOwned(0)
                | CriteriaEvent::HighestAuctionBid(0)
                | CriteriaEvent::HighestAuctionSale(0)
                | CriteriaEvent::DamageDealt(0)
                | CriteriaEvent::HighestDamageDone(0)
                | CriteriaEvent::HighestDamageTaken(0)
                | CriteriaEvent::TotalDamageTaken(0)
                | CriteriaEvent::HealingDone(0)
                | CriteriaEvent::HighestHealCast(0)
                | CriteriaEvent::TotalHealReceived(0)
                | CriteriaEvent::HighestHealReceived(0)
        ) && !state.criteria_progress.contains_key(&id)
        {
            return;
        }
        let counter = state.criteria_progress.entry(id).or_default();
        *counter = match event {
            CriteriaEvent::ReachLevel(level) => (*counter).max(u64::from(level)),
            CriteriaEvent::FallWithoutDying(distance) => u64::from(distance),
            CriteriaEvent::LifetimeHonorableKills(count) => count,
            CriteriaEvent::PersonalArenaRating { rating, .. } => (*counter).max(u64::from(rating)),
            CriteriaEvent::MostMoneyOwned(amount)
            | CriteriaEvent::HighestAuctionBid(amount)
            | CriteriaEvent::HighestAuctionSale(amount)
            | CriteriaEvent::HighestDamageDone(amount)
            | CriteriaEvent::HighestDamageTaken(amount)
            | CriteriaEvent::HighestHealCast(amount)
            | CriteriaEvent::HighestHealReceived(amount) => (*counter).max(amount),
            CriteriaEvent::SkillRank(_, rank) | CriteriaEvent::SkillMaxRank(_, rank) => {
                u64::from(rank)
            }
            CriteriaEvent::ActivelyReachLevel(_)
            | CriteriaEvent::LearnedNewPet(_)
            | CriteriaEvent::CompleteQuest(_)
            | CriteriaEvent::LearnSpell(_)
            | CriteriaEvent::ExploreAreaBit(_)
            | CriteriaEvent::EnterArea(_)
            | CriteriaEvent::LeaveArea(_)
            | CriteriaEvent::EnterTopLevelArea(_)
            | CriteriaEvent::KnowPetCreature(_)
            | CriteriaEvent::LearnToy(_)
            | CriteriaEvent::LearnTaxiNode(_)
            | CriteriaEvent::GotHaircut
            | CriteriaEvent::EquipItem(_) => 1,
            CriteriaEvent::TotalExaltedFactions(count)
            | CriteriaEvent::TotalReveredFactions(count)
            | CriteriaEvent::TotalHonoredFactions(count)
            | CriteriaEvent::KnowSkillLineSpells(_, count)
            | CriteriaEvent::LearnTradeskillSkillLine(_, count)
            | CriteriaEvent::OwnUniquePets(count)
            | CriteriaEvent::OwnToys(count)
            | CriteriaEvent::RewardedQuestCount(count)
            | CriteriaEvent::BankTabsOwned(_, count)
            | CriteriaEvent::PetBattleAchievementPoints(count) => u64::from(count),
            CriteriaEvent::RewardedQuestsInZone(_, count) => (*counter).max(u64::from(count)),
            CriteriaEvent::DamageDealt(amount)
            | CriteriaEvent::TotalDamageTaken(amount)
            | CriteriaEvent::HealingDone(amount)
            | CriteriaEvent::TotalHealReceived(amount)
            | CriteriaEvent::MoneyEarnedFromQuesting(amount)
            | CriteriaEvent::MoneySpentOnTaxis(amount)
            | CriteriaEvent::MoneySpentAtBarberShop(amount)
            | CriteriaEvent::MoneySpentOnPostage(amount)
            | CriteriaEvent::MoneyEarnedFromSales(amount)
            | CriteriaEvent::MoneyEarnedFromAuctions(amount) => counter.saturating_add(amount),
            CriteriaEvent::ReputationGained(_, value) => u64::from(value),
            CriteriaEvent::LootMoney(amount) => counter.saturating_add(amount),
            CriteriaEvent::GainLevels(amount) => counter.saturating_add(u64::from(amount)),
            CriteriaEvent::CurrencyGained(_, amount)
            | CriteriaEvent::AcquireItem(_, amount)
            | CriteriaEvent::LootItem(_, amount)
            | CriteriaEvent::GetLootByType(_, amount) => counter.saturating_add(u64::from(amount)),
            _ => counter.saturating_add(1),
        };
    }

    /// Type81 counts the points of every earned Pet Battles achievement, including those
    /// this grant pass just earned, so the loop reaches a fixed point.
    fn set_pet_battle_points(&self, state: &mut PlayerAchievementState) {
        let points = self.window.pet_battle_points(|id| {
            state.earned_ids.contains(&id)
                || state
                    .account
                    .as_deref()
                    .is_some_and(|account| account.earned_ids.contains(&id))
        });
        for &id in self.event_criteria.get(&81).into_iter().flatten() {
            Self::update_counter(state, id, CriteriaEvent::PetBattleAchievementPoints(points));
        }
    }

    pub(super) fn set_account_unique_pet_count(
        &self,
        state: &mut PlayerAchievementState,
        count: u32,
    ) {
        for &id in self.event_criteria.get(&156).into_iter().flatten() {
            Self::update_counter(state, id, CriteriaEvent::OwnUniquePets(count));
        }
    }

    pub(super) fn set_account_toy_ownership(
        &self,
        state: &mut PlayerAchievementState,
        toys: &BTreeMap<u32, bool>,
    ) {
        for &id in self.event_criteria.get(&185).into_iter().flatten() {
            let item = self.criteria[&id].asset;
            if toys.contains_key(&item) {
                Self::update_counter(state, id, CriteriaEvent::LearnToy(item));
            }
        }
        for &id in self.event_criteria.get(&186).into_iter().flatten() {
            Self::update_counter(
                state,
                id,
                CriteriaEvent::OwnToys(u32::try_from(toys.len()).expect("toy count fits u32")),
            );
        }
    }

    /// Scenario credit is restricted to the active tree and never grants achievements.
    pub(crate) fn update_tree_event(
        &self,
        root: u32,
        state: &mut PlayerAchievementState,
        event: CriteriaEvent,
        conditions: &PlayerConditions,
    ) -> bool {
        self.update_roots_event(std::iter::once(root), state, event, conditions)
    }

    /// Multiple visible activities may share criteria; one actual event counts once.
    pub(crate) fn update_roots_event(
        &self,
        roots: impl Iterator<Item = u32>,
        state: &mut PlayerAchievementState,
        event: CriteriaEvent,
        conditions: &PlayerConditions,
    ) -> bool {
        let ids: BTreeSet<_> = roots
            .flat_map(|root| self.criterion_tree_ids(root))
            .map(|tree_id| self.trees[&tree_id].criteria_id)
            .collect();
        let mut changed = false;
        for id in ids {
            if self.criteria.get(&id).is_some_and(|row| {
                row.supported && event.matches(row) && row.conditions_hold(conditions, event)
            }) {
                Self::update_counter(state, id, event);
                changed = true;
            }
        }
        changed
    }

    pub fn tree_progress(&self, id: u32, state: &PlayerAchievementState) -> TreeProgress {
        #[cfg(test)]
        EVALUATION_WORK.with(|work| {
            let (definitions, trees) = work.get();
            work.set((definitions, trees + 1));
        });
        let tree = &self.trees[&id];
        let counter = self.criterion_counter(tree.criteria_id, state);
        let children = self.children.get(&id).into_iter().flatten();
        let progress: Vec<_> = children
            .map(|child| self.tree_progress(*child, state))
            .collect();
        match tree.operator {
            TreeOperator::Single => {
                // Authored amount0 still requires one real credit (e.g. LearnOrKnowSpell).
                // TC CriteriaHandler.cpp:1278-1279: AchieveSkillStep amounts are 75-point steps.
                let step = if self
                    .criteria
                    .get(&tree.criteria_id)
                    .is_some_and(|row| row.kind == 40)
                {
                    75
                } else {
                    1
                };
                let required = tree.amount.max(1) * step;
                TreeProgress {
                    current: counter.unwrap_or(0).min(required),
                    required,
                    completed: counter.is_some_and(|counter| counter >= required),
                }
            }
            TreeOperator::All => TreeProgress {
                current: progress.iter().map(|child| child.current).sum(),
                required: progress.iter().map(|child| child.required).sum(),
                completed: !progress.is_empty() && progress.iter().all(|child| child.completed),
            },
            _ => self.aggregate_progress(id, state, &progress),
        }
    }

    fn aggregate_progress(
        &self,
        id: u32,
        state: &PlayerAchievementState,
        children: &[TreeProgress],
    ) -> TreeProgress {
        let tree = &self.trees[&id];
        let current = match tree.operator {
            TreeOperator::Any => children.iter().filter(|child| child.completed).count() as u64,
            TreeOperator::CountDirectChildren => self.count_started_children(id, state),
            TreeOperator::SumChildren => self.sum_counters(id, state, false),
            TreeOperator::SumChildrenWeight => self.sum_counters(id, state, true),
            TreeOperator::MaxChild => self.max_counter(id, state),
            _ => return TreeProgress::default(),
        };
        TreeProgress {
            current: current.min(tree.amount),
            required: tree.amount,
            completed: current >= tree.amount,
        }
    }

    /// TC StartedAtLeast1118–1129 counts direct children with an actual positive counter.
    fn count_started_children(&self, id: u32, state: &PlayerAchievementState) -> u64 {
        let direct_children = self.children.get(&id).into_iter().flatten();
        direct_children
            .filter(|child| {
                let criteria = self.trees[child].criteria_id;
                self.criterion_counter(criteria, state)
                    .is_some_and(|counter| counter >= 1)
            })
            .count() as u64
    }

    fn sum_counters(&self, id: u32, state: &PlayerAchievementState, weighted: bool) -> u64 {
        let tree = &self.trees[&id];
        let counter = self.criterion_counter(tree.criteria_id, state).unwrap_or(0);
        let own = if weighted {
            counter.saturating_mul(tree.amount)
        } else {
            counter
        };
        self.children
            .get(&id)
            .into_iter()
            .flatten()
            .fold(own, |sum, child| {
                sum.saturating_add(self.sum_counters(*child, state, weighted))
            })
    }

    fn max_counter(&self, id: u32, state: &PlayerAchievementState) -> u64 {
        let own = self
            .criterion_counter(self.trees[&id].criteria_id, state)
            .unwrap_or(0);
        self.children
            .get(&id)
            .into_iter()
            .flatten()
            .fold(own, |max, child| max.max(self.max_counter(*child, state)))
    }

    pub(crate) fn faction_matches(&self, required: i32, race: u8) -> bool {
        if required == -1 {
            return true;
        }
        let template = crate::faction::faction_template_for_race(race);
        // FactionTemplate group bits: Alliance=2, Horde=4; player bit=1.
        // Achievement.Faction uses Horde=0, Alliance=1, Any=-1 instead.
        let faction = match self.faction_groups.get(&template).map(|group| group & 6) {
            Some(2) => Some(1),
            Some(4) => Some(0),
            _ => None,
        };
        faction == Some(required)
    }

    fn criterion_counter(&self, id: u32, state: &PlayerAchievementState) -> Option<u64> {
        if let Some(criteria) = self.criteria.get(&id)
            && criteria.kind == 8
        {
            return (state.earned_ids.contains(&criteria.asset)
                || state.account.as_deref().is_some_and(|account| {
                    self.window.has_account_flag(criteria.asset)
                        && account.earned_ids.contains(&criteria.asset)
                }))
            .then_some(1);
        }
        state.criteria_progress.get(&id).copied()
    }

    pub(crate) fn supports_tree(&self, id: u32, visiting: &mut BTreeSet<u32>) -> bool {
        let Some(tree) = self.trees.get(&id) else {
            return false;
        };
        if !visiting.insert(id) {
            return false;
        }
        let children = self.children.get(&id);
        let supported = match tree.operator {
            TreeOperator::Unsupported => false,
            TreeOperator::Single => self
                .criteria
                .get(&tree.criteria_id)
                .is_some_and(|criteria| criteria.supported),
            _ => children.is_some_and(|children| {
                !children.is_empty()
                    && children
                        .iter()
                        .all(|child| self.supports_tree(*child, visiting))
            }),
        };
        visiting.remove(&id);
        supported
    }
}

pub(crate) fn load_achievement_catalog(conn: &Connection) -> rusqlite::Result<AchievementCatalog> {
    let mut catalog = AchievementCatalog::default();
    let mut areas =
        conn.prepare("SELECT ID, AreaBit FROM area_table WHERE ID != 0 AND AreaBit >= 0")?;
    catalog.area_bits = areas
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut pets =
        conn.prepare("SELECT DISTINCT creature_id FROM battle_pet_species WHERE creature_id != 0")?;
    catalog.pet_creatures = pets
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut species = conn.prepare("SELECT species_id FROM battle_pet_species")?;
    catalog.pet_species = species
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    load_criteria(conn, &mut catalog)?;
    load_spell_skill_lines(conn, &mut catalog)?;
    load_trees(conn, &mut catalog)?;
    load_definitions(conn, &mut catalog)?;
    let mut query = conn.prepare("SELECT ID, FactionGroup FROM faction_template")?;
    catalog.faction_groups = query
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    catalog.window = window::load_window_data(conn)?;
    for (&id, criteria) in &catalog.criteria {
        if criteria.supported {
            catalog
                .event_criteria
                .entry(criteria.kind)
                .or_default()
                .push(id);
        }
    }
    catalog.index_evaluation_dependencies();
    Ok(catalog)
}

/// TC DB2Manager::IsInArea follows authored parent links, unlike terrain zone selection.
fn collect_area_descendants(root: u32, children: &HashMap<u32, Vec<u32>>) -> BTreeSet<u32> {
    let mut descendants = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(area) = pending.pop() {
        if descendants.insert(area) {
            pending.extend(children.get(&area).into_iter().flatten().copied());
        }
    }
    descendants
}

fn load_map_instance_type(
    conn: &Connection,
    instance_type: u32,
) -> rusqlite::Result<Option<CriteriaFact>> {
    let mut query = conn.prepare_cached("SELECT ID FROM map WHERE InstanceType = ?1")?;
    let members = query
        .query_map([instance_type], |row| row.get::<_, u32>(0))?
        .collect::<rusqlite::Result<BTreeSet<_>>>()?;
    Ok((!members.is_empty()).then_some(CriteriaFact::MapInstanceType(members)))
}

/// TC Zone41 uses the current area, or its immediate parent only when flagged subzone.
fn load_zone_members(conn: &Connection, zone: u32) -> rusqlite::Result<BTreeSet<u32>> {
    let areas = crate::area_table::load_area_table(conn)?;
    let mut query = conn.prepare_cached("SELECT ID, ContinentID FROM area_table")?;
    let rows = query.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))?;
    let members = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(members
        .into_iter()
        .filter_map(|(area, map)| (areas.zone_id(area, map) == Some(zone)).then_some(area))
        .collect())
}

/// TC BattlePetType2157–2164 resolves the actual event species' authored type.
fn load_battle_pet_type_fact(
    conn: &Connection,
    required_type: u32,
) -> rusqlite::Result<Option<CriteriaFact>> {
    if required_type > 9 {
        return Ok(None);
    }
    let mut query = conn.prepare_cached(
        "SELECT s.species_id FROM battle_pet_species s \
         JOIN battle_pet_species_metadata m ON m.ID=s.species_id WHERE m.PetTypeEnum=?1",
    )?;
    let species = query
        .query_map([required_type], |row| row.get::<_, u32>(0))?
        .collect::<rusqlite::Result<BTreeSet<_>>>()?;
    Ok((!species.is_empty()).then_some(CriteriaFact::BattlePetSpeciesOfType(species)))
}

/// TC PlayerScenarioStep compares the actual authored OrderIndex, not vector position.
fn load_scenario_step_fact(
    conn: &Connection,
    stage: u32,
) -> rusqlite::Result<Option<CriteriaFact>> {
    let Some(order) = stage.checked_sub(1) else {
        return Ok(None);
    };
    let mut query = conn.prepare_cached("SELECT ID FROM scenario_step WHERE OrderIndex=?1")?;
    let steps = query
        .query_map([order], |row| row.get::<_, u32>(0))?
        .collect::<rusqlite::Result<BTreeSet<_>>>()?;
    Ok((!steps.is_empty()).then_some(CriteriaFact::ScenarioSteps(steps)))
}

/// Every leaf must name data the server has, else the tree can never hold (None).
fn load_content_tuning_fact(
    conn: &Connection,
    asset: u32,
) -> rusqlite::Result<Option<CriteriaFact>> {
    let range = conn
        .prepare_cached("SELECT minlevel, maxlevel FROM content_tuning_level WHERE ID = ?1")?
        .query_row([asset], |row| {
            Ok((row.get::<_, Option<u8>>(0)?, row.get::<_, Option<u8>>(1)?))
        })
        .optional()?;
    let Some((Some(min), Some(max))) = range else {
        return Ok(None);
    };
    Ok((min > 0 && min <= max).then_some(CriteriaFact::ContentTuningLevels(min..=max)))
}

fn load_criteria_facts(
    conn: &Connection,
    leaves: &[(i32, u32)],
    area_children: &HashMap<u32, Vec<u32>>,
) -> rusqlite::Result<Option<Vec<CriteriaFact>>> {
    let mut facts = Vec::with_capacity(leaves.len());
    for &(kind, asset) in leaves {
        let exists = |table: &str| {
            conn.prepare_cached(&format!(
                "SELECT EXISTS(SELECT 1 FROM {table} WHERE ID = ?1)"
            ))?
            .query_row([asset], |row| row.get::<_, bool>(0))
        };
        let small = u8::try_from(asset).ok();
        let fact = match kind {
            2 => crate::game_objects::identity_conditions::load_condition(conn, asset)?
                .ok()
                .map(CriteriaFact::Identity),
            4 => Some(CriteriaFact::TargetCreature(asset)),
            5 => Some(CriteriaFact::TargetIsPlayer),
            6 if asset == 0 => Some(CriteriaFact::TargetIsDead),
            7 if asset == 0 => Some(CriteriaFact::TargetIsHostile),
            8 => exists("spell")?.then_some(CriteriaFact::Aura(asset)),
            -8 => exists("spell")?.then_some(CriteriaFact::NotAura(asset)),
            10 => exists("spell")?.then_some(CriteriaFact::TargetAura(asset)),
            82 => load_scenario_step_fact(conn, asset)?,
            268 => load_content_tuning_fact(conn, asset)?,
            87 => exists("achievement")?.then_some(CriteriaFact::EarnedAchievement(asset)),
            -87 => exists("achievement")?.then_some(CriteriaFact::NotEarnedAchievement(asset)),
            104 => exists("spell")?.then_some(CriteriaFact::KnownSpell(asset)),
            -104 => exists("spell")?.then_some(CriteriaFact::NotKnownSpell(asset)),
            110 => exists("content_quest_template")?.then_some(CriteriaFact::CompletedQuest(asset)),
            14 | 15 => load_item_quality_fact(conn, kind, asset)?,
            16 => Some(CriteriaFact::Alive),
            19 => exists("content_item")?
                .then(|| CriteriaFact::EventItems([asset].into_iter().collect())),
            17 => exists("area_table")?
                .then(|| CriteriaFact::Area(collect_area_descendants(asset, area_children))),
            24 => small
                .filter(|size| matches!(size, 2 | 3))
                .map(CriteriaFact::ArenaTeamSize),
            60 => Some(CriteriaFact::RankedArena),
            // Only a race a character can be created as can ever hold.
            25 => small
                .filter(|race| crate::race_class::race_bit(*race).is_some())
                .map(CriteriaFact::Race),
            26 => small
                .filter(|class| (1..=13).contains(class))
                .map(CriteriaFact::Class),
            27 => small
                .filter(|race| crate::race_class::race_bit(*race).is_some())
                .map(CriteriaFact::TargetRace),
            28 => small
                .filter(|class| (1..=13).contains(class))
                .map(CriteriaFact::TargetClass),
            29 => Some(CriteriaFact::LessThanGroupMembers(asset)),
            32 => exists("map")?.then_some(CriteriaFact::Map(asset)),
            34 => small
                .filter(|level| (1..=shared::pet_battle::MAX_PET_LEVEL).contains(level))
                .map(CriteriaFact::BattlePetTeamLevel),
            35 => Some(CriteriaFact::NotInParty),
            30 => small
                .filter(|kind| (1..=15).contains(kind))
                .map(CriteriaFact::KilledCreatureType),
            67 => node_counts::required_owned_nodes(asset).map(CriteriaFact::EitherTeamNodeCount),
            78 => load_battle_pet_type_fact(conn, asset)?,
            46 => small
                .filter(|percent| (1..=100).contains(percent))
                .map(CriteriaFact::TargetHealthBelowPercent),
            41 => {
                if !exists("area_table")? {
                    return Ok(None);
                }
                Some(CriteriaFact::Area(load_zone_members(conn, asset)?))
            }
            122 => load_map_instance_type(conn, asset)?,
            69 => small
                .filter(|level| (1..=crate::experience::MAX_LEVEL).contains(level))
                .map(CriteriaFact::MinLevel),
            71 => Some(CriteriaFact::MaxLevel(small.unwrap_or(u8::MAX))),
            _ => {
                unreachable!(
                    "generated facts use only -104/-87/-8/2/4/5/6/7/8/14/15/16/17/19/24/25/26/27/28/29/30/32/34/35/41/46/60/67/69/71/78/82/87/104/110/122/268"
                )
            }
        };
        let Some(fact) = fact else {
            return Ok(None);
        };
        facts.push(fact);
    }
    Ok(Some(facts))
}

/// TC item quality14/15 reads the actual item's canonical template, not its slot.
fn load_item_quality_fact(
    conn: &Connection,
    kind: i32,
    quality: u32,
) -> rusqlite::Result<Option<CriteriaFact>> {
    let operator = if kind == 14 { ">=" } else { "=" };
    let mut query = conn.prepare_cached(&format!(
        "SELECT ID FROM content_item WHERE Quality {operator} ?1"
    ))?;
    let items = query
        .query_map([quality], |row| row.get::<_, u32>(0))?
        .collect::<rusqlite::Result<BTreeSet<_>>>()?;
    Ok((!items.is_empty()).then_some(CriteriaFact::EventItems(items)))
}

/// Reject every alternative when any authored item is absent; never discard a child.
fn load_event_item_alternatives(
    conn: &Connection,
    items: &[u32],
) -> rusqlite::Result<Option<CriteriaFact>> {
    let mut query =
        conn.prepare_cached("SELECT EXISTS(SELECT 1 FROM content_item WHERE ID = ?1)")?;
    for item in items {
        if !query.query_row([item], |row| row.get::<_, bool>(0))? {
            return Ok(None);
        }
    }
    Ok(Some(CriteriaFact::EventItems(
        items.iter().copied().collect(),
    )))
}

/// Reject the entire Some when even one authored map is missing.
fn load_player_map_alternatives(
    conn: &Connection,
    maps: &[u32],
) -> rusqlite::Result<Option<CriteriaFact>> {
    let mut query = conn.prepare_cached("SELECT EXISTS(SELECT 1 FROM map WHERE ID = ?1)")?;
    for map in maps {
        let exists = query.query_row([map], |row| row.get::<_, bool>(0))?;
        if !exists {
            return Ok(None);
        }
    }
    Ok(Some(CriteriaFact::PlayerMaps(
        maps.iter().copied().collect(),
    )))
}

/// Validate every typed area17/zone41 leaf before unioning their accepted current areas.
fn load_player_location_alternatives(
    conn: &Connection,
    leaves: &[(i32, u32)],
    area_children: &HashMap<u32, Vec<u32>>,
) -> rusqlite::Result<Option<CriteriaFact>> {
    let Some(facts) = load_criteria_facts(conn, leaves, area_children)? else {
        return Ok(None);
    };
    let members = facts
        .into_iter()
        .flat_map(|fact| match fact {
            CriteriaFact::Area(members) => members,
            _ => unreachable!("validated17/41 leaves produce only Area facts"),
        })
        .collect();
    Ok(Some(CriteriaFact::Area(members)))
}

/// Reuse complete8 spell validation; no area ancestry participates in aura leaves.
fn load_player_aura_alternatives(
    conn: &Connection,
    spells: &[u32],
) -> rusqlite::Result<Option<CriteriaFact>> {
    let leaves: Vec<_> = spells.iter().map(|spell| (8, *spell)).collect();
    Ok(load_criteria_facts(conn, &leaves, &HashMap::new())?
        .map(|_| CriteriaFact::PlayerAuras(spells.iter().copied().collect())))
}

fn load_criteria(conn: &Connection, catalog: &mut AchievementCatalog) -> rusqlite::Result<()> {
    let mut areas = conn.prepare("SELECT ID, ParentAreaID FROM area_table WHERE ID != 0")?;
    let mut area_children = HashMap::<u32, Vec<u32>>::new();
    for row in areas.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))? {
        let (area, parent) = row?;
        if parent != 0 {
            area_children.entry(parent).or_default().push(area);
        }
    }
    let native_pvp_stats: BTreeSet<u32> = shared::battleground::all_templates()
        .iter()
        .flat_map(|template| shared::battleground::pvp_stat_columns(template.map_id))
        .map(|column| column.stat)
        .collect();
    let mut query = conn.prepare("SELECT c.ID, c.Type, c.Asset, c.ModifierTreeId, c.StartEvent, c.StartTimer, c.FailEvent, c.EligibilityWorldStateID, EXISTS(SELECT 1 FROM spell s WHERE s.ID = c.Asset), EXISTS(SELECT 1 FROM skill_line s WHERE s.ID = c.Asset), EXISTS(SELECT 1 FROM toy t WHERE t.ItemID = c.Asset), EXISTS(SELECT 1 FROM scenario s WHERE s.ID = c.Asset), EXISTS(SELECT 1 FROM area_table a WHERE a.ID = c.Asset), EXISTS(SELECT 1 FROM taxi_nodes t WHERE t.ID = c.Asset) FROM criteria c")?;
    let rows = query.query_map([], |row| {
        let kind = row.get(1)?;
        let conditional = (3..8)
            .map(|index| row.get::<_, u32>(index))
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let criteria_id: u32 = row.get(0)?;
        let mythic = if kind == 216 {
            mythic_predicates::mythic_requirements(criteria_id, conditional[0])
        } else {
            None
        };
        let exploration_bits: Vec<_> = if kind == 43 {
            overlays::areas(row.get::<_, i32>(2)? as u32)
                .into_iter()
                .flatten()
                .filter_map(|area| catalog.area_bit(area))
                .collect()
        } else {
            Vec::new()
        };
        let accepts_single_modifier = match kind {
            155 | 221 => row.get::<_, i32>(2)? == 0,
            160 | 162 => (1..=25).contains(&row.get::<_, i32>(2)?),
            _ => false,
        };
        let modifier_asset = if accepts_single_modifier {
            single_modifiers::single_modifier_asset(criteria_id, conditional[0])
                .filter(|species| kind == 221 || catalog.pet_species.contains(species))
        } else {
            None
        };
        // Single-modifier families already compile their own player leaf.
        let compiled = conditions::criteria_conditions(criteria_id, conditional[0])
            .filter(|_| modifier_asset.is_none());
        let resolved = match compiled {
            Some(CompiledConditions::All { difficulty, facts }) => {
                load_criteria_facts(conn, facts, &area_children)?.map(|facts| (difficulty, facts))
            }
            Some(CompiledConditions::TargetCreatures(entries)) => Some((
                None,
                vec![CriteriaFact::TargetCreatures(entries.iter().copied().collect())],
            )),
            Some(CompiledConditions::PlayerMaps(maps)) => {
                load_player_map_alternatives(conn, maps)?.map(|fact| (None, vec![fact]))
            }
            Some(CompiledConditions::PlayerLocations(leaves)) => {
                load_player_location_alternatives(conn, leaves, &area_children)?.map(|fact| (None, vec![fact]))
            }
            Some(CompiledConditions::PlayerAuras(spells)) => {
                load_player_aura_alternatives(conn, spells)?.map(|fact| (None, vec![fact]))
            }
            Some(CompiledConditions::EventItems(items)) => load_event_item_alternatives(conn, items)?.map(|fact| (None, vec![fact])),
            None => None,
        };
        let condition_tree = resolved.is_some();
        let (difficulty, facts) = resolved.unwrap_or_default();
        let supported_predicate = (mythic.is_some() || modifier_asset.is_some())
            && conditional.iter().skip(1).all(|value| *value == 0);
        Ok((
            row.get(0)?,
            Criteria {
                kind,
                mythic,
                modifier_asset,
                area_members: if matches!(kind, 31 | 163 | 164) {
                    collect_area_descendants(row.get::<_, i32>(2)? as u32, &area_children)
                } else {
                    BTreeSet::new()
                },
                exploration_bits: exploration_bits.clone(),
                asset: row.get::<_, i32>(2)? as u32,
                // TDB1210 criteria_data:11118-11127 require holiday404;
                // 5258 has data_type0 (no condition). No holiday inference.
                supported: supported_predicate
                    || ((matches!(kind, 0 | 1 | 5 | 8 | 12 | 15 | 16 | 18 | 27 | 36 | 42 | 46 | 57 | 68 | 73 | 165)
                        // TDB1210 criteria_data11134-11140 require orphan aura66303/holiday404.
                        || (kind == 41 && !(11134..=11140).contains(&criteria_id))
                        // TC completion1288-1293 makes type20 statistic-only; new ordinary61917
                        // criterion111356 has no sourced completion semantics in the pinned TC.
                        || (kind == 20 && criteria_id != 111356)
                        // TC IsCompletedCriteria1300–1327 makes91 statistic-only;
                        // pinned ordinary12927/13707 lack compatible completion semantics.
                        || (kind == 91 && !matches!(criteria_id, 41244 | 45924) && row.get::<_, i32>(2)? == 0)
                        || (kind == 90 && row.get::<_, i32>(2)? == 0)
                        // TC Player.h environmental0/1/2/6; no native lava/slime/fire source.
                        || (kind == 26 && matches!(row.get::<_, i32>(2)?, 0 | 1 | 2 | 6))
                        // TC TextEmotes IDs produced by native Dance/Wave/Sit/Sleep/Kneel/Love.
                        // TDB1210 has no criteria_data overlay for these unconditional rows.
                        || (kind == 54 && matches!(row.get::<_, i32>(2)?, 34 | 101 | 86 | 87 | 59 | 225 | 56))
                        // Current criteria451/447 and authored370/595 map2v2/3v3 to0/1,
                        // unlike old TC ArenaTeam numeric2/3/5; no retired/other bracket inference.
                        || (kind == 39 && matches!(row.get::<_, i32>(2)?, 0 | 1))
                        || (kind == 31 && row.get::<_, i32>(2)? > 0 && row.get::<_, bool>(12)?)
                        || (kind == 52 && (1..=13).contains(&row.get::<_, i32>(2)?))
                        || (kind == 53 && u8::try_from(row.get::<_, u32>(2)?).ok()
                            .is_some_and(|race| crate::race_class::race_bit(race).is_some()))
                        // Native rated arena result has a real positive map ID, never an all-map alias0.
                        || (matches!(kind, 32 | 33 | 97) && row.get::<_, i32>(2)? > 0)
                        || (kind == 196 && (1..=i32::from(crate::experience::MAX_LEVEL)).contains(&row.get::<_, i32>(2)?))
                        || (kind == 43 && !exploration_bits.is_empty())
                        || (kind == 185 && row.get::<_, bool>(10)?)
                        || (kind == 152 && row.get::<_, bool>(11)?)
                        || (matches!(kind, 11 | 163 | 164 | 225) && row.get::<_, bool>(12)?)
                        || (kind == 262 && row.get::<_, bool>(13)?)
                        // PvpStat columns credited by native battleground objectives.
                        || (kind == 30 && native_pvp_stats.contains(&(row.get::<_, i32>(2)? as u32)))
                        // TC CriteriaHandler.cpp:796-807 counts Character0 and Account2 banks.
                        || (kind == 260 && matches!(row.get::<_, i32>(2)?, 0 | 2))
                        // Retail CraftingOrderType Public0/Guild1/Personal2; native NPC3 unavailable.
                        || (kind == 246 && matches!(row.get::<_, i32>(2)?, 0..=2))
                        // Loot types `crate::loot::LootKind` produces; no fishing3.
                        || (kind == 109 && matches!(row.get::<_, i32>(2)?, 1 | 4 | 6 | 9))
                        || (matches!(kind, 160 | 162) && (1..=25).contains(&row.get::<_, i32>(2)?))
                        || (matches!(kind, 24 | 35 | 113 | 37 | 9 | 13 | 101 | 102 | 103 | 14 | 17 | 23 | 48 | 55 | 104 | 105 | 106 | 59 | 62 | 63 | 65 | 66 | 67 | 74 | 76 | 77 | 80 | 81 | 82 | 83 | 84 | 85 | 86 | 107 | 108 | 114 | 130 | 219 | 220 | 245 | 151 | 156 | 157 | 158 | 186 | 221 | 253 | 47 | 87 | 88 | 56 | 70 | 78) && row.get::<_, i32>(2)? == 0)
                        || (matches!(kind, 7 | 40 | 75 | 112) && row.get::<_, bool>(9)?)
                        || (kind == 96
                            && catalog
                                .pet_creatures
                                .contains(&(row.get::<_, i32>(2)? as u32)))
                        || (matches!(kind, 28 | 34 | 69 | 110) && row.get::<_, bool>(8)?)
                        || (kind == 29
                            && row.get::<_, bool>(8)?
                            && !(11118..=11127).contains(&row.get::<_, u32>(0)?)))
                        && (conditional.iter().all(|value| *value == 0)
                            || (condition_tree
                                && conditional.iter().skip(1).all(|value| *value == 0)))),
                difficulty,
                facts,
            },
        ))
    })?;
    catalog.criteria = rows.collect::<rusqlite::Result<_>>()?;
    Ok(())
}

fn load_spell_skill_lines(
    conn: &Connection,
    catalog: &mut AchievementCatalog,
) -> rusqlite::Result<()> {
    let mut query = conn.prepare("SELECT DISTINCT s.Spell, s.SkillLine FROM skill_line_ability s JOIN criteria c ON c.Asset=s.SkillLine WHERE c.Type IN (75,112)")?;
    for row in query.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))? {
        let (spell, skill) = row?;
        catalog
            .spell_skill_lines
            .entry(spell)
            .or_default()
            .insert(skill);
    }
    Ok(())
}

fn load_trees(conn: &Connection, catalog: &mut AchievementCatalog) -> rusqlite::Result<()> {
    let mut query = conn.prepare("SELECT ID, Parent, Amount, Operator, CriteriaID FROM criteria_tree ORDER BY OrderIndex, ID")?;
    let rows = query.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            CriteriaTree {
                amount: row.get(2)?,
                operator: TreeOperator::from_db2(row.get(3)?),
                criteria_id: row.get(4)?,
            },
        ))
    })?;
    for row in rows {
        let (id, parent, tree) = row?;
        catalog.trees.insert(id, tree);
        if parent != 0 {
            catalog.children.entry(parent).or_default().push(id);
        }
    }
    Ok(())
}

fn load_definitions(conn: &Connection, catalog: &mut AchievementCatalog) -> rusqlite::Result<()> {
    // Counters are statistics; Guild-flag definitions require a guild actor, not a player.
    const NON_PLAYER_GRANTS: u32 = 0x1 | 0x4000;
    let mut query = conn.prepare("SELECT ID, Title_lang, Points, CriteriaTree, Instance_ID, Faction FROM achievement WHERE CriteriaTree != 0 AND (Flags & ?1) = 0")?;
    let rows = query.query_map([NON_PLAYER_GRANTS], |row| {
        let map_id: i32 = row.get(4)?;
        Ok(AchievementDef {
            id: row.get(0)?,
            name: row.get(1)?,
            points: row.get(2)?,
            root: row.get(3)?,
            map_id: (map_id >= 0).then_some(map_id as u32),
            faction: row.get(5)?,
        })
    })?;
    for row in rows {
        let definition = row?;
        if catalog.supports_tree(definition.root, &mut BTreeSet::new()) {
            catalog.definitions.insert(definition.id, definition);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "achievement_catalog_tests.rs"]
mod tests;

#[path = "achievement_window.rs"]
mod window;
pub(crate) use window::bound_text;

#[cfg(test)]
#[path = "achievement_window_tests.rs"]
mod window_tests;
