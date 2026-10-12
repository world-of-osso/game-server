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

// Retail Blizzard_FrameXMLBase/Constants.lua:123: ACHIEVEMENT_FLAGS_ACCOUNT.
// 0x00010000 is SHOW_CRITERIA_MEMBERS, not account scope.
const ACHIEVEMENT_FLAG_ACCOUNT: u32 = 0x0002_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CriteriaEvent {
    KillCreature(u32),
    ReachLevel(u8),
    SkillRank(u32, u16),
    CurrencyGained(u32, u32),
    ReputationGained(u32, u32),
    AcquireItem(u32, u32),
    WinPetBattle,
    Login,
    ExploreAreaBit(u32),
    TriggerGameEvent(u32),
    GainAura(u32),
    CompleteMythicPlus(MythicCompletion),
    CompleteQuest(u32),
    CastSpell(u32),
    BeSpellTarget(u32),
    LearnSpell(u32),
    DefeatEncounter(u32),
}

impl CriteriaEvent {
    fn kind(self) -> u32 {
        match self {
            Self::KillCreature(_) => 0,
            Self::ReachLevel(_) => 5,
            Self::SkillRank(_, _) => 7,
            Self::CurrencyGained(_, _) => 12,
            Self::ReputationGained(_, _) => 46,
            Self::AcquireItem(_, _) => 36,
            Self::WinPetBattle => 158,
            Self::Login => 74,
            Self::ExploreAreaBit(_) => 43,
            Self::TriggerGameEvent(_) => 73,
            Self::GainAura(_) => 69,
            Self::CompleteMythicPlus(_) => 216,
            Self::CompleteQuest(_) => 27,
            Self::CastSpell(_) => 29,
            Self::BeSpellTarget(_) => 28,
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
            Self::ExploreAreaBit(bit) => criteria.exploration_bits.contains(&bit),
            Self::WinPetBattle | Self::Login => criteria.asset == 0,
            Self::CompleteMythicPlus(completion) => criteria
                .mythic
                .is_some_and(|requirements| requirements.matches(completion)),
            Self::SkillRank(asset, _)
            | Self::CurrencyGained(asset, _)
            | Self::ReputationGained(asset, _)
            | Self::AcquireItem(asset, _) => asset == criteria.asset,
            Self::KillCreature(asset)
            | Self::CompleteQuest(asset)
            | Self::CastSpell(asset)
            | Self::BeSpellTarget(asset)
            | Self::LearnSpell(asset)
            | Self::TriggerGameEvent(asset)
            | Self::GainAura(asset)
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

#[derive(Debug, Clone)]
struct Criteria {
    kind: u32,
    asset: u32,
    supported: bool,
    mythic: Option<MythicRequirements>,
    exploration_bits: Vec<u32>,
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
}

#[derive(Debug, Default)]
pub(crate) struct TreeProgress {
    pub current: u64,
    pub required: u64,
    pub completed: bool,
}

impl AchievementCatalog {
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
    pub(super) fn aggregate(
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
                if !contribution.signed_reputation
                    && self.criteria.get(&id).is_some_and(|row| row.kind == 46)
                {
                    continue;
                }
                let counter = aggregate.criteria_progress.entry(id).or_default();
                *counter = if self.criteria.get(&id).is_some_and(|row| {
                    matches!(
                        row.kind,
                        0 | 12 | 28 | 29 | 36 | 69 | 73 | 74 | 158 | 165 | 216
                    )
                }) {
                    counter.saturating_add(value)
                } else {
                    (*counter).max(value)
                };
            }
        }
        aggregate
    }

    fn update_counter(state: &mut PlayerAchievementState, id: u32, event: CriteriaEvent) {
        let counter = state.criteria_progress.entry(id).or_default();
        *counter = match event {
            CriteriaEvent::ReachLevel(level) => (*counter).max(u64::from(level)),
            CriteriaEvent::SkillRank(_, rank) => u64::from(rank),
            CriteriaEvent::CompleteQuest(_)
            | CriteriaEvent::LearnSpell(_)
            | CriteriaEvent::ExploreAreaBit(_) => 1,
            CriteriaEvent::ReputationGained(_, value) => u64::from(value),
            CriteriaEvent::CurrencyGained(_, amount) | CriteriaEvent::AcquireItem(_, amount) => {
                counter.saturating_add(u64::from(amount))
            }
            _ => counter.saturating_add(1),
        };
    }

    /// Scenario credit is restricted to the active tree and never grants achievements.
    pub(crate) fn update_tree_event(
        &self,
        root: u32,
        state: &mut PlayerAchievementState,
        event: CriteriaEvent,
    ) -> bool {
        let mut changed = false;
        for tree_id in self.criterion_tree_ids(root) {
            let id = self.trees[&tree_id].criteria_id;
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
            TreeOperator::Single => TreeProgress {
                current: counter.unwrap_or(0).min(tree.amount),
                required: tree.amount,
                completed: counter.is_some_and(|counter| counter >= tree.amount),
            },
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
    load_criteria(conn, &mut catalog)?;
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

fn load_criteria(conn: &Connection, catalog: &mut AchievementCatalog) -> rusqlite::Result<()> {
    let mut query = conn.prepare("SELECT c.ID, c.Type, c.Asset, c.ModifierTreeId, c.StartEvent, c.StartTimer, c.FailEvent, c.EligibilityWorldStateID, EXISTS(SELECT 1 FROM spell s WHERE s.ID = c.Asset), EXISTS(SELECT 1 FROM skill_line s WHERE s.ID = c.Asset) FROM criteria c")?;
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
        let supported_mythic =
            mythic.is_some() && conditional.iter().skip(1).all(|value| *value == 0);
        Ok((
            row.get(0)?,
            Criteria {
                kind,
                mythic,
                exploration_bits: exploration_bits.clone(),
                asset: row.get::<_, i32>(2)? as u32,
                // TDB1210 criteria_data:11118-11127 require holiday404;
                // 5258 has data_type0 (no condition). No holiday inference.
                supported: supported_mythic
                    || ((matches!(kind, 0 | 5 | 8 | 12 | 27 | 36 | 46 | 73 | 165)
                        || (kind == 43 && !exploration_bits.is_empty())
                        || (matches!(kind, 74 | 158) && row.get::<_, i32>(2)? == 0)
                        || (kind == 7 && row.get::<_, bool>(9)?)
                        || (matches!(kind, 28 | 34 | 69) && row.get::<_, bool>(8)?)
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
