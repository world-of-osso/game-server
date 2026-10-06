//! Achievement/Criteria/CriteriaTree DB2 rows imported by scripts/import_achievements.py.
//! Only unconditional character kill/level/encounter criteria are enabled. Trees
//! containing unknown criteria, modifiers, timers or world-state conditions stay disabled.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use bevy::prelude::*;
use rusqlite::Connection;
use shared::protocol::{
    AchievementProgressSnapshot, AchievementSnapshot, AchievementToastSnapshot,
};

use super::PlayerAchievementState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CriteriaEvent {
    KillCreature(u32),
    ReachLevel(u8),
    DefeatEncounter(u32),
}

impl CriteriaEvent {
    fn kind(self) -> u32 {
        match self {
            Self::KillCreature(_) => 0,
            Self::ReachLevel(_) => 5,
            Self::DefeatEncounter(_) => 165,
        }
    }

    fn matches(self, criteria: &Criteria) -> bool {
        if criteria.kind != self.kind() {
            return false;
        }
        match self {
            Self::ReachLevel(_) => true,
            Self::KillCreature(asset) | Self::DefeatEncounter(asset) => asset == criteria.asset,
        }
    }
}

#[derive(Debug, Clone)]
struct Criteria {
    kind: u32,
    asset: u32,
    supported: bool,
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
}

#[derive(Resource, Debug, Default)]
pub(crate) struct AchievementCatalog {
    window: window::WindowData,
    definitions: BTreeMap<u32, AchievementDef>,
    criteria: HashMap<u32, Criteria>,
    pub trees: HashMap<u32, CriteriaTree>,
    pub children: HashMap<u32, Vec<u32>>,
    event_criteria: HashMap<u32, Vec<u32>>,
}

#[derive(Debug, Default)]
pub(crate) struct TreeProgress {
    pub current: u64,
    pub required: u64,
    pub completed: bool,
}

impl AchievementCatalog {
    /// Apply one event and return newly earned toasts. Map achievements are granted
    /// only at the complete-run boundary, not on a partial dungeon's boss kill.
    pub fn update(
        &self,
        state: &mut PlayerAchievementState,
        event: CriteriaEvent,
        completed_map: Option<u32>,
    ) -> Vec<AchievementToastSnapshot> {
        for &id in self.event_criteria.get(&event.kind()).into_iter().flatten() {
            if !event.matches(&self.criteria[&id]) {
                continue;
            }
            let counter = state.criteria_progress.entry(id).or_default();
            *counter = match event {
                CriteriaEvent::ReachLevel(level) => (*counter).max(u64::from(level)),
                _ => counter.saturating_add(1),
            };
        }
        self.grant_completed(state, completed_map)
    }

    fn grant_completed(
        &self,
        state: &mut PlayerAchievementState,
        completed_map: Option<u32>,
    ) -> Vec<AchievementToastSnapshot> {
        let completed: Vec<_> = self
            .definitions
            .values()
            .filter(|definition| {
                let map_eligible =
                    definition.map_id.is_none() || definition.map_id == completed_map;
                map_eligible
                    && !state.earned_ids.contains(&definition.id)
                    && self.tree_progress(definition.root, state).completed
            })
            .collect();
        completed
            .into_iter()
            .map(|definition| {
                state.earned_ids.insert(definition.id);
                AchievementToastSnapshot {
                    achievement_id: definition.id,
                    name: definition.name.clone(),
                    points: definition.points,
                }
            })
            .collect()
    }

    pub fn snapshot(&self, state: &PlayerAchievementState) -> AchievementSnapshot {
        let progress = self
            .definitions
            .values()
            .map(|definition| {
                let progress = self.tree_progress(definition.root, state);
                AchievementProgressSnapshot {
                    achievement_id: definition.id,
                    current: progress.current.min(u64::from(u32::MAX)) as u32,
                    required: progress.required.min(u64::from(u32::MAX)) as u32,
                    completed: state.earned_ids.contains(&definition.id),
                }
            })
            .collect();
        AchievementSnapshot {
            earned_ids: state.earned_ids.iter().copied().collect(),
            progress,
        }
    }

    pub fn tree_progress(&self, id: u32, state: &PlayerAchievementState) -> TreeProgress {
        let tree = &self.trees[&id];
        let counter = state.criteria_progress.get(&tree.criteria_id).copied();
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
                    state
                        .criteria_progress
                        .get(&criteria)
                        .is_some_and(|counter| *counter >= 1)
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
        let counter = state
            .criteria_progress
            .get(&tree.criteria_id)
            .copied()
            .unwrap_or(0);
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
        let own = state
            .criteria_progress
            .get(&self.trees[&id].criteria_id)
            .copied()
            .unwrap_or(0);
        self.children
            .get(&id)
            .into_iter()
            .flatten()
            .fold(own, |max, child| max.max(self.max_counter(*child, state)))
    }

    fn supports_tree(&self, id: u32, visiting: &mut BTreeSet<u32>) -> bool {
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
    load_criteria(conn, &mut catalog)?;
    load_trees(conn, &mut catalog)?;
    load_definitions(conn, &mut catalog)?;
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
    let mut query = conn.prepare("SELECT ID, Type, Asset, ModifierTreeId, StartEvent, StartTimer, FailEvent, EligibilityWorldStateID FROM criteria")?;
    let rows = query.query_map([], |row| {
        let kind = row.get(1)?;
        let conditional = (3..8)
            .map(|index| row.get::<_, u32>(index))
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok((
            row.get(0)?,
            Criteria {
                kind,
                asset: row.get::<_, i32>(2)? as u32,
                supported: matches!(kind, 0 | 5 | 165)
                    && conditional.iter().all(|value| *value == 0),
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
    // Faction-specific, account-wide and counter/statistic achievements require
    // eligibility/account tracking outside phase 1. Do not grant them as character achievements.
    const ACCOUNT_OR_COUNTER: u32 = 0x1 | 0x10000;
    let mut query = conn.prepare("SELECT ID, Title_lang, Points, CriteriaTree, Instance_ID FROM achievement WHERE CriteriaTree != 0 AND Faction = -1 AND (Flags & ?1) = 0")?;
    let rows = query.query_map([ACCOUNT_OR_COUNTER], |row| {
        let map_id: i32 = row.get(4)?;
        Ok(AchievementDef {
            id: row.get(0)?,
            name: row.get(1)?,
            points: row.get(2)?,
            root: row.get(3)?,
            map_id: (map_id >= 0).then_some(map_id as u32),
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
