//! Achievement/Criteria/CriteriaTree DB2 rows imported by scripts/import_achievements.py.
//! Unconditional character kill/level/quest/achievement/encounter criteria are enabled. Trees
//! containing unknown criteria, timers or world-state conditions stay disabled.
//! M+ admits only compiled pinned all-of completion predicates; unknown modifiers stay disabled.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use bevy::prelude::*;
use rusqlite::Connection;
use shared::protocol::{
    AchievementProgressSnapshot, AchievementSnapshot, AchievementToastSnapshot,
};

use super::PlayerAchievementState;

/// Event counters add across an account's characters; absolute facts use the maximum.
const SUMMED_ACCOUNT_KINDS: &[u32] = &[
    0, 1, 78, 11, 12, 14, 15, 16, 17, 20, 23, 26, 28, 29, 36, 41, 42, 59, 62, 63, 65, 67, 68, 69,
    73, 66, 74, 76, 77, 80, 82, 84, 107, 108, 109, 110, 114, 130, 151, 152, 157, 158, 160, 162,
    165, 13, 54, 55, 56, 103, 105, 216, 219, 220, 221, 245, 246, 253,
];

// Retail Blizzard_FrameXMLBase/Constants.lua:123: ACHIEVEMENT_FLAGS_ACCOUNT.
// 0x00010000 is SHOW_CRITERIA_MEMBERS, not account scope.
const ACHIEVEMENT_FLAG_ACCOUNT: u32 = 0x0002_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CriteriaEvent {
    KillCreature(u32),
    KillAnyCreature,
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
    ParticipateInBattleground(u32),
    DieOnMap(u32),
    DieAnywhere,
    KilledByCreature(u32),
    KilledByPlayer,
    DieFromEnvironmentalDamage(u32),
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
    DoEmote(u32),
    DeliveredKillingBlow,
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
    BattlePetReachedLevel(u8),
    ActivelyEarnedPetLevel(u8),
    Login,
    ExploreAreaBit(u32),
    EnterTopLevelArea(u32),
    EnterArea(u32),
    LeaveArea(u32),
    TriggerGameEvent(u32),
    GainAura(u32),
    WinBattleground(u32),
    WinAnyBattleground,
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
    LandTargetedSpell(u32),
    LearnSpell(u32),
    DefeatEncounter(u32),
}

impl CriteriaEvent {
    fn kind(self) -> u32 {
        match self {
            Self::KillCreature(_) => 0,
            Self::KillAnyCreature => 78,
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
            Self::KilledByCreature(_) => 20,
            Self::KilledByPlayer => 23,
            Self::DieFromEnvironmentalDamage(_) => 26,
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
            Self::DoEmote(_) => 54,
            Self::DeliveredKillingBlow => 56,
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
            Self::BattlePetReachedLevel(_) => 160,
            Self::ActivelyEarnedPetLevel(_) => 162,
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
            Self::LandTargetedSpell(_) => 110,
            Self::LearnSpell(_) => 34,
            Self::DefeatEncounter(_) => 165,
        }
    }

    fn matches(self, criteria: &Criteria) -> bool {
        if criteria.kind != self.kind() {
            return false;
        }
        match self {
            Self::ReachLevel(_) => true,
            Self::ActivelyReachLevel(level) => u32::from(level) == criteria.asset,
            Self::BattlePetReachedLevel(level) | Self::ActivelyEarnedPetLevel(level) => {
                u32::from(level) == criteria.asset
            }
            Self::ExploreAreaBit(bit) => criteria.exploration_bits.contains(&bit),
            Self::EnterArea(area) => area != 0 && criteria.area_members.contains(&area),
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
            Self::KillAnyCreature
            | Self::DeliveredKillingBlow
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
            | Self::LandTargetedSpell(asset)
            | Self::LearnSpell(asset)
            | Self::TriggerGameEvent(asset)
            | Self::GainAura(asset)
            | Self::WinBattleground(asset)
            | Self::ParticipateInBattleground(asset)
            | Self::DieOnMap(asset)
            | Self::KilledByCreature(asset)
            | Self::DieFromEnvironmentalDamage(asset)
            | Self::DoEmote(asset)
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
            | Self::DefeatEncounter(asset) => asset == criteria.asset,
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

#[path = "achievement_mythic_predicates.rs"]
mod mythic_predicates;
#[path = "achievement_overlays.rs"]
mod overlays;
#[path = "achievement_single_modifiers.rs"]
mod single_modifiers;

#[derive(Debug, Clone)]
struct Criteria {
    kind: u32,
    asset: u32,
    supported: bool,
    mythic: Option<MythicRequirements>,
    /// Compiled sole PlayerClass26/BattlePetSpecies91 requirement.
    modifier_asset: Option<u32>,
    exploration_bits: Vec<u32>,
    area_members: BTreeSet<u32>,
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

#[derive(Resource, Debug, Default)]
pub(crate) struct AchievementCatalog {
    window: window::WindowData,
    definitions: BTreeMap<u32, AchievementDef>,
    criteria: HashMap<u32, Criteria>,
    pub trees: HashMap<u32, CriteriaTree>,
    pub children: HashMap<u32, Vec<u32>>,
    event_criteria: HashMap<u32, Vec<u32>>,
    faction_groups: HashMap<u32, u32>,
    area_bits: HashMap<u32, u32>,
    pet_creatures: BTreeSet<u32>,
    pet_species: BTreeSet<u32>,
    spell_skill_lines: HashMap<u32, BTreeSet<u32>>,
}

#[derive(Debug, Default)]
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

    /// Apply one event and return newly earned toasts. Map achievements are granted
    /// only at the complete-run boundary, not on a partial dungeon's boss kill.
    pub fn update(
        &self,
        state: &mut PlayerAchievementState,
        event: CriteriaEvent,
        completed_map: Option<u32>,
        race: u8,
    ) -> Vec<AchievementToastSnapshot> {
        if !state.signed_reputation {
            state
                .criteria_progress
                .retain(|id, _| self.criteria.get(id).is_none_or(|row| row.kind != 46));
            state.signed_reputation = true;
        }
        for &id in self.event_criteria.get(&event.kind()).into_iter().flatten() {
            if !event.matches(&self.criteria[&id]) {
                continue;
            }
            Self::update_counter(state, id, event);
        }
        self.grant_completed(state, completed_map, race, false)
    }

    pub(super) fn grant_completed(
        &self,
        state: &mut PlayerAchievementState,
        completed_map: Option<u32>,
        race: u8,
        account_scope: bool,
    ) -> Vec<AchievementToastSnapshot> {
        let mut toasts = Vec::new();
        loop {
            self.set_pet_battle_points(state);
            let completed =
                self.find_completed_definitions(state, completed_map, race, account_scope);
            if completed.is_empty() {
                return toasts;
            }
            for definition in completed {
                state.earned_ids.insert(definition.id);
                toasts.push(AchievementToastSnapshot {
                    achievement_id: definition.id,
                    name: definition.name.clone(),
                    points: definition.points,
                    icon_fdid: self.window.icon_fdid(definition.id),
                });
            }
        }
    }

    fn find_completed_definitions(
        &self,
        state: &PlayerAchievementState,
        completed_map: Option<u32>,
        race: u8,
        account_scope: bool,
    ) -> Vec<&AchievementDef> {
        self.definitions
            .values()
            .filter(|definition| {
                if self.window.has_account_flag(definition.id) != account_scope
                    || state.earned_ids.contains(&definition.id)
                {
                    return false;
                }
                if !self.faction_matches(definition.faction, race) {
                    return false;
                }
                if let Some(map) = definition.map_id
                    && completed_map != Some(map)
                {
                    return false;
                }
                self.tree_progress(definition.root, state).completed
            })
            .collect()
    }

    pub fn snapshot(&self, state: &PlayerAchievementState, race: u8) -> AchievementSnapshot {
        AchievementSnapshot {
            earned_ids: state
                .earned_ids
                .iter()
                .filter(|id| !self.window.has_account_flag(**id))
                .copied()
                .chain(
                    state
                        .account
                        .iter()
                        .flat_map(|account| account.earned_ids.iter().copied())
                        .filter(|id| self.window.has_account_flag(*id)),
                )
                .collect(),
            progress: self.progress_snapshot(state, race),
        }
    }

    fn progress_snapshot(
        &self,
        state: &PlayerAchievementState,
        race: u8,
    ) -> Vec<AchievementProgressSnapshot> {
        self.definitions
            .values()
            .filter(|definition| self.faction_matches(definition.faction, race))
            .filter(|definition| {
                !self.window.has_account_flag(definition.id) || state.account.is_some()
            })
            .map(|definition| {
                let state = self.definition_state(state, definition.id);
                let progress = self.tree_progress(definition.root, state);
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
            CriteriaEvent::KnowSkillLineSpells(_, 0)
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
    ) -> bool {
        self.update_roots_event(std::iter::once(root), state, event)
    }

    /// Multiple visible activities may share criteria; one actual event counts once.
    pub(crate) fn update_roots_event(
        &self,
        roots: impl Iterator<Item = u32>,
        state: &mut PlayerAchievementState,
        event: CriteriaEvent,
    ) -> bool {
        let ids: BTreeSet<_> = roots
            .flat_map(|root| self.criterion_tree_ids(root))
            .map(|tree_id| self.trees[&tree_id].criteria_id)
            .collect();
        let mut changed = false;
        for id in ids {
            if self
                .criteria
                .get(&id)
                .is_some_and(|row| row.supported && event.matches(row))
            {
                Self::update_counter(state, id, event);
                changed = true;
            }
        }
        changed
    }

    pub fn tree_progress(&self, id: u32, state: &PlayerAchievementState) -> TreeProgress {
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
            TreeOperator::CountDirectChildren => self
                .children
                .get(&id)
                .into_iter()
                .flatten()
                .filter(|child| {
                    let criteria = self.trees[child].criteria_id;
                    self.criterion_counter(criteria, state)
                        .is_some_and(|counter| counter >= 1)
                })
                .count() as u64,
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

fn load_criteria(conn: &Connection, catalog: &mut AchievementCatalog) -> rusqlite::Result<()> {
    let mut areas = conn.prepare("SELECT ID, ParentAreaID FROM area_table WHERE ID != 0")?;
    let mut area_children = HashMap::<u32, Vec<u32>>::new();
    for row in areas.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))? {
        let (area, parent) = row?;
        if parent != 0 {
            area_children.entry(parent).or_default().push(area);
        }
    }
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
        let modifier_asset = if matches!(kind, 155 | 221) && row.get::<_, i32>(2)? == 0 {
            single_modifiers::single_modifier_asset(criteria_id, conditional[0])
                .filter(|species| kind != 155 || catalog.pet_species.contains(species))
        } else {
            None
        };
        let supported_predicate = (mythic.is_some() || modifier_asset.is_some())
            && conditional.iter().skip(1).all(|value| *value == 0);
        Ok((
            row.get(0)?,
            Criteria {
                kind,
                mythic,
                modifier_asset,
                area_members: if matches!(kind, 163 | 164) {
                    collect_area_descendants(row.get::<_, i32>(2)? as u32, &area_children)
                } else {
                    BTreeSet::new()
                },
                exploration_bits: exploration_bits.clone(),
                asset: row.get::<_, i32>(2)? as u32,
                // TDB1210 criteria_data:11118-11127 require holiday404;
                // 5258 has data_type0 (no condition). No holiday inference.
                supported: supported_predicate
                    || ((matches!(kind, 0 | 1 | 5 | 8 | 12 | 15 | 16 | 27 | 36 | 42 | 46 | 57 | 68 | 73 | 165)
                        // TDB1210 criteria_data11134-11140 require orphan aura66303/holiday404.
                        || (kind == 41 && !(11134..=11140).contains(&criteria_id))
                        // TC completion1288-1293 makes type20 statistic-only; new ordinary61917
                        // criterion111356 has no sourced completion semantics in the pinned TC.
                        || (kind == 20 && criteria_id != 111356)
                        // TC Player.h environmental0/1/2/6; no native lava/slime/fire source.
                        || (kind == 26 && matches!(row.get::<_, i32>(2)?, 0 | 1 | 2 | 6))
                        // TC TextEmotes IDs produced by native Dance/Wave/Sit/Sleep/Kneel.
                        // TDB1210 has no criteria_data overlay for these unconditional rows.
                        || (kind == 54 && matches!(row.get::<_, i32>(2)?, 34 | 101 | 86 | 87 | 59))
                        || (kind == 196 && (1..=i32::from(crate::experience::MAX_LEVEL)).contains(&row.get::<_, i32>(2)?))
                        || (kind == 43 && !exploration_bits.is_empty())
                        || (kind == 185 && row.get::<_, bool>(10)?)
                        || (kind == 152 && row.get::<_, bool>(11)?)
                        || (matches!(kind, 11 | 163 | 164 | 225) && row.get::<_, bool>(12)?)
                        || (kind == 262 && row.get::<_, bool>(13)?)
                        // TC CriteriaHandler.cpp:796-807 counts Character0 and Account2 banks.
                        || (kind == 260 && matches!(row.get::<_, i32>(2)?, 0 | 2))
                        // Retail CraftingOrderType Public0/Guild1/Personal2; native NPC3 unavailable.
                        || (kind == 246 && matches!(row.get::<_, i32>(2)?, 0..=2))
                        // Loot types `crate::loot::LootKind` produces; no fishing3.
                        || (kind == 109 && matches!(row.get::<_, i32>(2)?, 1 | 4 | 6 | 9))
                        || (matches!(kind, 160 | 162) && (1..=25).contains(&row.get::<_, i32>(2)?))
                        || (matches!(kind, 9 | 13 | 101 | 102 | 103 | 14 | 17 | 23 | 48 | 55 | 104 | 105 | 106 | 59 | 62 | 63 | 65 | 66 | 67 | 74 | 76 | 77 | 80 | 81 | 82 | 83 | 84 | 85 | 86 | 107 | 108 | 114 | 130 | 219 | 220 | 245 | 151 | 156 | 157 | 158 | 186 | 221 | 253 | 47 | 87 | 88 | 56 | 78) && row.get::<_, i32>(2)? == 0)
                        || (matches!(kind, 7 | 40 | 75 | 112) && row.get::<_, bool>(9)?)
                        || (kind == 96
                            && catalog
                                .pet_creatures
                                .contains(&(row.get::<_, i32>(2)? as u32)))
                        || (matches!(kind, 28 | 34 | 69 | 110) && row.get::<_, bool>(8)?)
                        || (kind == 29
                            && row.get::<_, bool>(8)?
                            && !(11118..=11127).contains(&row.get::<_, u32>(0)?)))
                        && conditional.iter().all(|value| *value == 0)),
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
