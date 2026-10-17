//! Bounded, on-demand catalog presentation; grant eligibility remains phase 1's.
use super::*;
use shared::protocol::{
    ACHIEVEMENT_CATEGORY_PAGE_SIZE, ACHIEVEMENT_CRITERIA_PAGE_SIZE,
    ACHIEVEMENT_DESCRIPTION_MAX_BYTES, ACHIEVEMENT_NAME_MAX_BYTES, ACHIEVEMENT_PAGE_SIZE,
    ACHIEVEMENT_SEARCH_MAX_CHARS, AchievementCatalogEntry, AchievementCatalogPage,
    AchievementCategoryEntry, AchievementCategoryTotals, AchievementCriterionLine,
    AchievementScope, AchievementSummary, QueryAchievementCatalog,
};

// TrinityCore a352b1fa DBCEnums.h:73-93: hidden, guild and hidden tracking copies.
const NON_PLAYER_UI_FLAGS: u32 = 0x2 | 0x4000 | 0x100000;
const STATISTIC_FLAG: u32 = 0x1;

#[derive(Debug)]
#[cfg_attr(test, derive(Clone))]
struct WindowDefinition {
    id: u32,
    category: u32,
    name: String,
    description: String,
    points: u32,
    icon: u32,
    root: u32,
    faction: i32,
    flags: u32,
}

#[derive(Debug, Default)]
#[cfg_attr(test, derive(Clone))]
pub(super) struct WindowData {
    categories: BTreeMap<u32, AchievementCategoryEntry>,
    definitions: BTreeMap<u32, WindowDefinition>,
    tree_labels: HashMap<u32, (String, i32, u32)>,
    /// (achievement, points) in the Pet Battles category tree.
    pet_battle: Vec<(u32, u32)>,
}

/// Retail Blizzard_PetCollection.lua:4,222: PET_ACHIEVEMENT_CATEGORY, summed with
/// subcategories by GetCategoryAchievementPoints for the Pet Journal total.
const PET_ACHIEVEMENT_CATEGORY: u32 = 15117;

impl WindowData {
    /// Points of earned achievements in the Pet Battles category tree.
    pub(super) fn pet_battle_points(&self, earned: impl Fn(u32) -> bool) -> u32 {
        self.pet_battle
            .iter()
            .filter(|(id, _)| earned(*id))
            .map(|(_, points)| points)
            .sum()
    }

    fn in_category(&self, mut category: u32, ancestor: u32) -> bool {
        let mut visited = BTreeSet::new();
        while visited.insert(category) {
            if category == ancestor {
                return true;
            }
            let Some(parent) = self.categories.get(&category).map(|row| row.parent_id) else {
                return false;
            };
            let Ok(parent) = u32::try_from(parent) else {
                return false;
            };
            category = parent;
        }
        false
    }

    pub(super) fn icon_fdid(&self, id: u32) -> u32 {
        self.definitions[&id].icon
    }

    pub(super) fn has_account_flag(&self, id: u32) -> bool {
        self.definitions
            .get(&id)
            .is_some_and(|row| row.flags & ACHIEVEMENT_FLAG_ACCOUNT != 0)
    }
}

pub(super) fn load_window_data(conn: &Connection) -> rusqlite::Result<WindowData> {
    let mut data = WindowData {
        categories: load_categories(conn)?,
        definitions: load_window_definitions(conn)?,
        tree_labels: load_tree_labels(conn)?,
        pet_battle: Vec::new(),
    };
    data.pet_battle = data
        .definitions
        .values()
        .filter(|row| data.in_category(row.category, PET_ACHIEVEMENT_CATEGORY))
        .map(|row| (row.id, row.points))
        .collect();
    Ok(data)
}

fn load_categories(conn: &Connection) -> rusqlite::Result<BTreeMap<u32, AchievementCategoryEntry>> {
    let mut query =
        conn.prepare("SELECT ID, Name_lang, Parent, Ui_order FROM achievement_category")?;
    let rows = query.query_map([], |row| {
        let id = row.get(0)?;
        Ok((
            id,
            AchievementCategoryEntry {
                category_id: id,
                name: bound_text(&row.get::<_, String>(1)?, ACHIEVEMENT_NAME_MAX_BYTES),
                parent_id: row.get(2)?,
                order_index: row.get(3)?,
            },
        ))
    })?;
    rows.collect()
}

fn load_window_definitions(conn: &Connection) -> rusqlite::Result<BTreeMap<u32, WindowDefinition>> {
    let mut query = conn.prepare("SELECT ID, Category, Title_lang, Description_lang, Points, IconFileID, CriteriaTree, Faction, Flags FROM achievement")?;
    let rows = query.query_map([], |row| {
        let id = row.get(0)?;
        Ok((
            id,
            WindowDefinition {
                id,
                category: row.get(1)?,
                name: bound_text(&row.get::<_, String>(2)?, ACHIEVEMENT_NAME_MAX_BYTES),
                description: bound_text(
                    &row.get::<_, String>(3)?,
                    ACHIEVEMENT_DESCRIPTION_MAX_BYTES,
                ),
                points: row.get(4)?,
                icon: row.get(5)?,
                root: row.get(6)?,
                faction: row.get(7)?,
                flags: row.get(8)?,
            },
        ))
    })?;
    rows.collect()
}

fn load_tree_labels(conn: &Connection) -> rusqlite::Result<HashMap<u32, (String, i32, u32)>> {
    let mut query =
        conn.prepare("SELECT ID, Description_lang, OrderIndex, Flags FROM criteria_tree")?;
    let rows = query.query_map([], |row| {
        Ok((
            row.get(0)?,
            (
                bound_text(&row.get::<_, String>(1)?, ACHIEVEMENT_NAME_MAX_BYTES),
                row.get(2)?,
                row.get(3)?,
            ),
        ))
    })?;
    rows.collect()
}

fn matches_scope(flags: u32, scope: AchievementScope) -> bool {
    let account = flags & ACHIEVEMENT_FLAG_ACCOUNT != 0;
    match scope {
        AchievementScope::All => true,
        AchievementScope::Account => account,
        AchievementScope::Character => !account,
    }
}

/// A UTF-8 prefix, bounded in bytes rather than characters.
pub(crate) fn bound_text(text: &str, max_bytes: usize) -> String {
    let mut end = text.len().min(max_bytes);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

fn paginate<T>(rows: impl Iterator<Item = (u32, T)>, size: usize) -> (Vec<T>, Option<u32>) {
    let mut selected: Vec<_> = rows.take(size + 1).collect();
    let has_more = selected.len() > size;
    selected.truncate(size);
    let next_id = has_more.then(|| selected.last().expect("nonempty page").0);
    (
        selected.into_iter().map(|(_, value)| value).collect(),
        next_id,
    )
}

impl AchievementCatalog {
    /// TC AchievementMgr.cpp475: real earned achievements need SHOW_IN_GUILD_NEWS.
    pub(crate) fn build_guild_news_achievement(
        &self,
        id: u32,
        member: &str,
    ) -> Option<(shared::protocol::GuildNewsKind, bool)> {
        const SHOW_IN_GUILD_NEWS: u32 = 0x1000;
        const SHOW_IN_GUILD_HEADER: u32 = 0x2000;
        let definition = self.window.definitions.get(&id)?;
        if definition.flags & SHOW_IN_GUILD_NEWS == 0 {
            return None;
        }
        Some((
            shared::protocol::GuildNewsKind::PlayerAchievement {
                member: member.into(),
                achievement_id: id,
                name: definition.name.clone(),
                description: definition.description.clone(),
            },
            definition.flags & SHOW_IN_GUILD_HEADER != 0,
        ))
    }

    pub fn page(
        &self,
        state: &PlayerAchievementState,
        request: QueryAchievementCatalog,
        race: u8,
    ) -> AchievementCatalogPage {
        match request {
            QueryAchievementCatalog::Comparison { target_entity, .. } => {
                AchievementCatalogPage::Comparison {
                    target_entity,
                    target_name: String::new(),
                    page: None,
                    error: Some("comparison requires an authenticated online target".into()),
                }
            }
            QueryAchievementCatalog::Statistics {
                category_id,
                after_id,
            } => self.statistics_page(state, category_id, after_id, race),
            QueryAchievementCatalog::Search {
                text,
                scope,
                after_id,
            } => self.search_page(state, &text, scope, after_id, race),
            QueryAchievementCatalog::Summary { scope } => {
                AchievementCatalogPage::Summary(self.summary(state, race, scope))
            }
            QueryAchievementCatalog::Categories { after_id } => self.categories_page(after_id),
            QueryAchievementCatalog::Achievement { achievement_id } => {
                let entry = self
                    .window
                    .definitions
                    .get(&achievement_id)
                    .filter(|row| self.faction_matches(row.faction, race))
                    .map(|row| self.window_entry(state, row, race));
                AchievementCatalogPage::Achievement {
                    achievement_id,
                    entry,
                }
            }
            QueryAchievementCatalog::Category {
                category_id,
                after_id,
            } => self.category_page(state, category_id, after_id, race),
            QueryAchievementCatalog::Criteria {
                achievement_id,
                after_id,
            } => {
                let (criteria, next_id) = self.criteria_page(state, achievement_id, after_id, race);
                AchievementCatalogPage::Criteria {
                    achievement_id,
                    criteria,
                    next_id,
                }
            }
        }
    }

    /// Retail AchievementUI.lua:2334-2348,2508-2512: full counts and three latest earned rows.
    fn summary(
        &self,
        state: &PlayerAchievementState,
        race: u8,
        scope: AchievementScope,
    ) -> AchievementSummary {
        let eligible: Vec<_> = self
            .window
            .definitions
            .values()
            .filter(|row| {
                row.flags & (NON_PLAYER_UI_FLAGS | STATISTIC_FLAG) == 0
                    && self.faction_matches(row.faction, race)
                    && matches_scope(row.flags, scope)
            })
            .collect();
        let mut summary = self.count_summary_rows(state, &eligible, scope);
        summary.recent = self.latest_earned_rows(state, &eligible, race);
        summary
    }

    fn count_summary_rows(
        &self,
        state: &PlayerAchievementState,
        rows: &[&WindowDefinition],
        scope: AchievementScope,
    ) -> AchievementSummary {
        let mut summary = AchievementSummary {
            scope,
            points: 0,
            total: rows.len() as u32,
            completed: 0,
            categories: Vec::new(),
            recent: Vec::new(),
        };
        let mut categories = BTreeMap::new();
        for row in rows {
            let completed = self
                .definition_state(state, row.id)
                .earned_ids
                .contains(&row.id);
            summary.completed += u32::from(completed);
            if completed {
                summary.points += row.points;
            }
            self.accumulate_category_totals(&mut categories, row.category, completed);
        }
        summary.categories = categories.into_values().collect();
        summary
    }

    fn latest_earned_rows(
        &self,
        state: &PlayerAchievementState,
        rows: &[&WindowDefinition],
        race: u8,
    ) -> Vec<AchievementCatalogEntry> {
        let mut earned: Vec<_> = rows
            .iter()
            .copied()
            .filter(|row| {
                self.definition_state(state, row.id)
                    .earned_ids
                    .contains(&row.id)
            })
            .collect();
        earned.sort_by_key(|row| {
            let date = self
                .definition_state(state, row.id)
                .earned_at
                .get(&row.id)
                .copied();
            (std::cmp::Reverse(date), row.id)
        });
        earned
            .into_iter()
            .take(3)
            .map(|row| self.window_entry(state, row, race))
            .collect()
    }

    fn accumulate_category_totals(
        &self,
        totals: &mut BTreeMap<u32, AchievementCategoryTotals>,
        category: u32,
        completed: bool,
    ) {
        let Some(row) = self.window.categories.get(&category) else {
            return;
        };
        let counts = totals.entry(category).or_insert(AchievementCategoryTotals {
            category_id: category,
            total: 0,
            completed: 0,
        });
        counts.total += 1;
        counts.completed += u32::from(completed);
        if row.parent_id >= 0 {
            self.accumulate_category_totals(totals, row.parent_id as u32, completed);
        }
    }

    fn categories_page(&self, after_id: u32) -> AchievementCatalogPage {
        let rows = self
            .window
            .categories
            .iter()
            .filter(|(id, _)| **id > after_id);
        let (categories, next_id) = paginate(
            rows.map(|(id, row)| (*id, row.clone())),
            ACHIEVEMENT_CATEGORY_PAGE_SIZE,
        );
        AchievementCatalogPage::Categories {
            categories,
            next_id,
        }
    }

    fn category_page(
        &self,
        state: &PlayerAchievementState,
        category_id: u32,
        after_id: u32,
        race: u8,
    ) -> AchievementCatalogPage {
        let rows = self
            .window
            .definitions
            .values()
            .filter(|row| row.category == category_id && row.id > after_id)
            .filter(|row| row.flags & (NON_PLAYER_UI_FLAGS | STATISTIC_FLAG) == 0)
            .filter(|row| self.faction_matches(row.faction, race));
        let (definitions, next_id) = paginate(rows.map(|row| (row.id, row)), ACHIEVEMENT_PAGE_SIZE);
        let achievements = definitions
            .into_iter()
            .map(|row| self.window_entry(state, row, race))
            .collect();
        AchievementCatalogPage::Category {
            category_id,
            achievements,
            next_id,
        }
    }

    /// Retail AchievementUI.lua:2235: raw quantities, never target-clamped progress.
    fn statistics_page(
        &self,
        state: &PlayerAchievementState,
        category_id: u32,
        after_id: u32,
        race: u8,
    ) -> AchievementCatalogPage {
        let rows = self.window.definitions.values().filter(|row| {
            row.id > after_id
                && row.category == category_id
                && row.flags & STATISTIC_FLAG != 0
                && row.flags & NON_PLAYER_UI_FLAGS == 0
                && self.faction_matches(row.faction, race)
        });
        let (rows, next_id) = paginate(
            rows.map(|row| (row.id, self.window_entry(state, row, race))),
            ACHIEVEMENT_PAGE_SIZE,
        );
        AchievementCatalogPage::Statistics {
            category_id,
            achievements: rows,
            next_id,
        }
    }

    /// Retail AchievementUI.lua:3447-3460,3632-3648: full catalog, including statistics.
    fn search_page(
        &self,
        state: &PlayerAchievementState,
        text: &str,
        scope: AchievementScope,
        after_id: u32,
        race: u8,
    ) -> AchievementCatalogPage {
        let text: String = text
            .trim()
            .chars()
            .take(ACHIEVEMENT_SEARCH_MAX_CHARS)
            .collect();
        let needle = text.to_lowercase();
        let rows = self.window.definitions.values().filter(|row| {
            !needle.is_empty()
                && row.id > after_id
                && row.flags & NON_PLAYER_UI_FLAGS == 0
                && matches_scope(row.flags, scope)
                && self.faction_matches(row.faction, race)
                && (row.name.to_lowercase().contains(&needle)
                    || row.description.to_lowercase().contains(&needle))
        });
        let (achievements, next_id) = paginate(
            rows.map(|row| (row.id, self.window_entry(state, row, race))),
            ACHIEVEMENT_PAGE_SIZE,
        );
        AchievementCatalogPage::Search {
            text,
            scope,
            achievements,
            next_id,
        }
    }

    fn statistic_value(
        &self,
        state: &PlayerAchievementState,
        row: &WindowDefinition,
    ) -> Option<u64> {
        if row.flags & 1 == 0
            || (row.flags & ACHIEVEMENT_FLAG_ACCOUNT != 0 && state.account.is_none())
            || !self.supports_tree(row.root, &mut BTreeSet::new())
        {
            return None;
        }
        let state = self.definition_state(state, row.id);
        self.raw_statistic_counter(row.root, state)
    }

    fn raw_statistic_counter(&self, id: u32, state: &PlayerAchievementState) -> Option<u64> {
        match self.trees[&id].operator {
            TreeOperator::Single => Some(
                self.criterion_counter(self.trees[&id].criteria_id, state)
                    .unwrap_or(0),
            ),
            TreeOperator::SumChildren => Some(self.sum_counters(id, state, false)),
            TreeOperator::SumChildrenWeight => Some(self.sum_counters(id, state, true)),
            TreeOperator::MaxChild => Some(self.max_counter(id, state)),
            TreeOperator::CountDirectChildren => Some(self.count_started_children(id, state)),
            TreeOperator::All => {
                let children = self.children.get(&id)?;
                if children.len() == 1 {
                    self.raw_statistic_counter(children[0], state)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn window_entry(
        &self,
        state: &PlayerAchievementState,
        row: &WindowDefinition,
        race: u8,
    ) -> AchievementCatalogEntry {
        let (criteria, next_criteria_id) = self.criteria_page(state, row.id, 0, race);
        let has_scope = row.flags & ACHIEVEMENT_FLAG_ACCOUNT == 0 || state.account.is_some();
        let statistic_value = self.statistic_value(state, row);
        let state = self.definition_state(state, row.id);
        let earned = state.earned_ids.contains(&row.id);
        AchievementCatalogEntry {
            achievement_id: row.id,
            name: row.name.clone(),
            description: row.description.clone(),
            points: row.points,
            icon_fdid: row.icon,
            flags: row.flags,
            statistic_value,
            earned,
            earned_at: earned
                .then(|| state.earned_at.get(&row.id).copied())
                .flatten(),
            progress_supported: has_scope && self.definitions.contains_key(&row.id),
            criteria,
            next_criteria_id,
        }
    }

    fn criteria_page(
        &self,
        state: &PlayerAchievementState,
        achievement_id: u32,
        after_id: u32,
        race: u8,
    ) -> (Vec<AchievementCriterionLine>, Option<u32>) {
        let Some(definition) = self.window.definitions.get(&achievement_id) else {
            return (Vec::new(), None);
        };
        if !self.faction_matches(definition.faction, race) {
            return (Vec::new(), None);
        }
        let ids = self.criterion_tree_ids(definition.root);
        let rows = ids.into_iter().filter(|id| *id > after_id);
        paginate(
            rows.map(|id| {
                (
                    id,
                    self.criterion_line(
                        self.definition_state(state, achievement_id),
                        id,
                        definition.flags & ACHIEVEMENT_FLAG_ACCOUNT == 0 || state.account.is_some(),
                    ),
                )
            }),
            ACHIEVEMENT_CRITERIA_PAGE_SIZE,
        )
    }

    pub(crate) fn criterion_tree_ids(&self, root: u32) -> BTreeSet<u32> {
        let mut pending = vec![root];
        let mut visited = BTreeSet::new();
        let mut criteria = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            if self
                .trees
                .get(&id)
                .is_some_and(|tree| tree.criteria_id != 0)
            {
                criteria.insert(id);
            }
            pending.extend(self.children.get(&id).into_iter().flatten().copied());
        }
        criteria
    }

    pub(crate) fn criterion_line(
        &self,
        state: &PlayerAchievementState,
        id: u32,
        character_scope: bool,
    ) -> AchievementCriterionLine {
        let tree = &self.trees[&id];
        let progress = self.tree_progress(id, state);
        let supported = character_scope
            && self
                .criteria
                .get(&tree.criteria_id)
                .is_some_and(|criteria| criteria.supported)
            && matches!(tree.operator, TreeOperator::Single);
        let (description, order_index, flags) = &self.window.tree_labels[&id];
        AchievementCriterionLine {
            tree_id: id,
            order_index: *order_index,
            criteria_id: tree.criteria_id,
            description: description.clone(),
            flags: *flags,
            current: if character_scope { progress.current } else { 0 },
            required: progress.required,
            completed: supported && progress.completed,
            progress_supported: supported,
        }
    }
}
