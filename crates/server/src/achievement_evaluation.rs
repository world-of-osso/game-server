//! Disposable derived evaluation state. Durable counters/earned IDs remain authoritative.
use super::*;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub(crate) struct EvaluationCache(Option<Arc<EvaluationMemo>>);

// Warming a disposable cache must not change logical player-state equality.
impl PartialEq for EvaluationCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for EvaluationCache {}

#[derive(Clone, Copy)]
struct CompletionBoundary {
    map: Option<u32>,
    race: u8,
    account_scope: bool,
}

#[derive(Debug, Clone, Default)]
struct EvaluationMemo {
    catalog: Arc<()>,
    counters: BTreeMap<u32, u64>,
    earned: BTreeSet<u32>,
    race: u8,
    account_scope: bool,
    progress: HashMap<u32, TreeProgress>,
}

#[derive(Debug, Default)]
#[cfg_attr(test, derive(Clone))]
pub(super) struct EvaluationIndex {
    identity: Arc<()>,
    criteria_definitions: HashMap<u32, BTreeSet<u32>>,
    earned_criteria: HashMap<u32, Vec<u32>>,
    map_definitions: HashMap<u32, BTreeSet<u32>>,
}

impl AchievementCatalog {
    pub(super) fn index_evaluation_dependencies(&mut self) {
        for definition in self.definitions.values() {
            for tree in self.criterion_tree_ids(definition.root) {
                let criterion = self.trees[&tree].criteria_id;
                self.evaluation_index
                    .criteria_definitions
                    .entry(criterion)
                    .or_default()
                    .insert(definition.id);
            }
            if let Some(map) = definition.map_id {
                self.evaluation_index
                    .map_definitions
                    .entry(map)
                    .or_default()
                    .insert(definition.id);
            }
        }
        for (&id, criterion) in &self.criteria {
            if criterion.kind == 8 {
                self.evaluation_index
                    .earned_criteria
                    .entry(criterion.asset)
                    .or_default()
                    .push(id);
            }
        }
    }

    fn evaluation_earned_ids(&self, state: &PlayerAchievementState) -> BTreeSet<u32> {
        let mut earned = state.earned_ids.clone();
        earned.extend(
            state
                .account
                .iter()
                .flat_map(|account| account.earned_ids.iter())
                .filter(|id| self.window.has_account_flag(**id))
                .copied(),
        );
        earned
    }

    fn changed_definitions(
        &self,
        memo: &EvaluationMemo,
        state: &PlayerAchievementState,
        earned: &BTreeSet<u32>,
    ) -> BTreeSet<u32> {
        if !Arc::ptr_eq(&memo.catalog, &self.evaluation_index.identity) {
            return self.definitions.keys().copied().collect();
        }
        let changed_counters = memo
            .counters
            .keys()
            .chain(state.criteria_progress.keys())
            .filter(|id| memo.counters.get(id) != state.criteria_progress.get(id))
            .copied();
        let changed_dependencies = memo
            .earned
            .symmetric_difference(earned)
            .flat_map(|id| {
                self.evaluation_index
                    .earned_criteria
                    .get(id)
                    .into_iter()
                    .flatten()
            })
            .copied();
        self.definitions_for_criteria(changed_counters.chain(changed_dependencies))
    }

    fn definitions_for_criteria(&self, criteria: impl Iterator<Item = u32>) -> BTreeSet<u32> {
        criteria
            .flat_map(|id| {
                self.evaluation_index
                    .criteria_definitions
                    .get(&id)
                    .into_iter()
                    .flatten()
            })
            .copied()
            .collect()
    }

    pub(crate) fn grant_completed(
        &self,
        state: &mut PlayerAchievementState,
        completed_map: Option<u32>,
        race: u8,
        account_scope: bool,
    ) -> Vec<AchievementToastSnapshot> {
        self.set_pet_battle_points(state);
        let boundary = CompletionBoundary {
            map: completed_map,
            race,
            account_scope,
        };
        if self.evaluation_unchanged(state, boundary) {
            return Vec::new();
        }
        let cache = std::mem::take(&mut state.evaluation);
        let mut memo = cache
            .0
            .map(|memo| Arc::unwrap_or_clone(memo))
            .unwrap_or_default();
        let mut toasts = Vec::new();
        loop {
            let completed = self.evaluate_completion_round(state, &mut memo, boundary);
            if completed.is_empty() {
                state.evaluation = EvaluationCache(Some(Arc::new(memo)));
                return toasts;
            }
            toasts.extend(completed);
        }
    }

    fn evaluate_completion_round(
        &self,
        state: &mut PlayerAchievementState,
        memo: &mut EvaluationMemo,
        boundary: CompletionBoundary,
    ) -> Vec<AchievementToastSnapshot> {
        self.set_pet_battle_points(state);
        let earned = self.evaluation_earned_ids(state);
        let changed = self.changed_definitions(memo, state, &earned);
        for &id in &changed {
            memo.progress
                .insert(id, self.tree_progress(self.definitions[&id].root, state));
        }
        let candidates = self.completion_candidates(memo, changed, boundary);
        memo.catalog = self.evaluation_index.identity.clone();
        memo.counters.clone_from(&state.criteria_progress);
        memo.earned = earned;
        memo.race = boundary.race;
        memo.account_scope = boundary.account_scope;
        self.complete_candidates(state, memo, candidates, boundary)
    }

    fn evaluation_unchanged(
        &self,
        state: &PlayerAchievementState,
        boundary: CompletionBoundary,
    ) -> bool {
        state.evaluation.0.as_deref().is_some_and(|memo| {
            boundary.map.is_none()
                && memo.race == boundary.race
                && memo.account_scope == boundary.account_scope
                && self.cached_progress(state).is_some()
        })
    }

    fn completion_candidates(
        &self,
        memo: &EvaluationMemo,
        mut changed: BTreeSet<u32>,
        boundary: CompletionBoundary,
    ) -> BTreeSet<u32> {
        if memo.race != boundary.race || memo.account_scope != boundary.account_scope {
            changed.extend(self.definitions.keys().copied());
        }
        if let Some(map) = boundary.map {
            changed.extend(
                self.evaluation_index
                    .map_definitions
                    .get(&map)
                    .into_iter()
                    .flatten()
                    .copied(),
            );
        }
        changed
    }

    fn complete_candidates(
        &self,
        state: &mut PlayerAchievementState,
        memo: &EvaluationMemo,
        candidates: BTreeSet<u32>,
        boundary: CompletionBoundary,
    ) -> Vec<AchievementToastSnapshot> {
        candidates
            .into_iter()
            .filter_map(|id| self.grant_candidate(state, memo, id, boundary))
            .collect()
    }

    fn grant_candidate(
        &self,
        state: &mut PlayerAchievementState,
        memo: &EvaluationMemo,
        id: u32,
        boundary: CompletionBoundary,
    ) -> Option<AchievementToastSnapshot> {
        #[cfg(test)]
        EVALUATION_WORK.with(|work| {
            let (definitions, trees) = work.get();
            work.set((definitions + 1, trees));
        });
        let definition = &self.definitions[&id];
        if self.window.has_account_flag(id) != boundary.account_scope
            || state.earned_ids.contains(&id)
            || !self.faction_matches(definition.faction, boundary.race)
            || definition
                .map_id
                .is_some_and(|map| boundary.map != Some(map))
            || !memo.progress[&id].completed
        {
            return None;
        }
        state.earned_ids.insert(id);
        Some(AchievementToastSnapshot {
            achievement_id: id,
            name: definition.name.clone(),
            points: definition.points,
            icon_fdid: self.window.icon_fdid(id),
        })
    }

    pub(super) fn cached_progress<'a>(
        &self,
        state: &'a PlayerAchievementState,
    ) -> Option<&'a HashMap<u32, TreeProgress>> {
        let memo = state.evaluation.0.as_deref()?;
        (Arc::ptr_eq(&memo.catalog, &self.evaluation_index.identity)
            && memo.counters == state.criteria_progress
            && memo.earned == self.evaluation_earned_ids(state))
        .then_some(&memo.progress)
    }
}
