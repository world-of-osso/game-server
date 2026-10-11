//! Bounded, on-demand catalog presentation; grant eligibility remains phase 1's.
use super::*;
use shared::protocol::{
    ACHIEVEMENT_CATEGORY_PAGE_SIZE, ACHIEVEMENT_CRITERIA_PAGE_SIZE,
    ACHIEVEMENT_DESCRIPTION_MAX_BYTES, ACHIEVEMENT_NAME_MAX_BYTES, ACHIEVEMENT_PAGE_SIZE,
    AchievementCatalogEntry, AchievementCatalogPage, AchievementCategoryEntry,
    AchievementCriterionLine, QueryAchievementCatalog,
};

#[derive(Debug)]
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
pub(super) struct WindowData {
    categories: BTreeMap<u32, AchievementCategoryEntry>,
    definitions: BTreeMap<u32, WindowDefinition>,
    tree_labels: HashMap<u32, (String, i32)>,
}

impl WindowData {
    pub(super) fn has_account_flag(&self, id: u32) -> bool {
        self.definitions
            .get(&id)
            .is_some_and(|row| row.flags & ACHIEVEMENT_FLAG_ACCOUNT != 0)
    }
}

pub(super) fn load_window_data(conn: &Connection) -> rusqlite::Result<WindowData> {
    Ok(WindowData {
        categories: load_categories(conn)?,
        definitions: load_window_definitions(conn)?,
        tree_labels: load_tree_labels(conn)?,
    })
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

fn load_tree_labels(conn: &Connection) -> rusqlite::Result<HashMap<u32, (String, i32)>> {
    let mut query = conn.prepare("SELECT ID, Description_lang, OrderIndex FROM criteria_tree")?;
    let rows = query.query_map([], |row| {
        Ok((
            row.get(0)?,
            (
                bound_text(&row.get::<_, String>(1)?, ACHIEVEMENT_NAME_MAX_BYTES),
                row.get(2)?,
            ),
        ))
    })?;
    rows.collect()
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
    pub fn page(
        &self,
        state: &PlayerAchievementState,
        request: QueryAchievementCatalog,
        race: u8,
    ) -> AchievementCatalogPage {
        match request {
            QueryAchievementCatalog::Categories { after_id } => self.categories_page(after_id),
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

    fn window_entry(
        &self,
        state: &PlayerAchievementState,
        row: &WindowDefinition,
        race: u8,
    ) -> AchievementCatalogEntry {
        let (criteria, next_criteria_id) = self.criteria_page(state, row.id, 0, race);
        let character_scope = row.flags & ACHIEVEMENT_FLAG_ACCOUNT == 0;
        let earned = character_scope && state.earned_ids.contains(&row.id);
        AchievementCatalogEntry {
            achievement_id: row.id,
            name: row.name.clone(),
            description: row.description.clone(),
            points: row.points,
            icon_fdid: row.icon,
            earned,
            earned_at: earned
                .then(|| state.earned_at.get(&row.id).copied())
                .flatten(),
            progress_supported: self.definitions.contains_key(&row.id),
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
                        state,
                        id,
                        definition.flags & ACHIEVEMENT_FLAG_ACCOUNT == 0,
                    ),
                )
            }),
            ACHIEVEMENT_CRITERIA_PAGE_SIZE,
        )
    }

    fn criterion_tree_ids(&self, root: u32) -> BTreeSet<u32> {
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

    fn criterion_line(
        &self,
        state: &PlayerAchievementState,
        id: u32,
        character_scope: bool,
    ) -> AchievementCriterionLine {
        let tree = &self.trees[&id];
        let counter = character_scope
            .then(|| self.criterion_counter(tree.criteria_id, state))
            .flatten();
        let supported = character_scope
            && self
                .criteria
                .get(&tree.criteria_id)
                .is_some_and(|criteria| criteria.supported)
            && matches!(tree.operator, TreeOperator::Single);
        let (description, order_index) = &self.window.tree_labels[&id];
        AchievementCriterionLine {
            tree_id: id,
            order_index: *order_index,
            criteria_id: tree.criteria_id,
            description: description.clone(),
            current: counter.unwrap_or(0).min(tree.amount),
            required: tree.amount,
            completed: supported && counter.is_some_and(|current| current >= tree.amount),
            progress_supported: supported,
        }
    }
}
