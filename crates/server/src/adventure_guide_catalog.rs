use super::conditions::{ConditionRule, load_conditions};
use rusqlite::Connection;
use serde::Deserialize;
use shared::protocol::{AdventureSuggestion, PerksActivityTag, SuggestionCensus};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Activity {
    #[serde(rename = "ID")]
    pub id: u32,
    #[serde(rename = "ActivityName_lang")]
    pub name: String,
    #[serde(rename = "Description_lang")]
    pub description: String,
    #[serde(rename = "CriteriaTreeID")]
    pub root: u32,
    #[serde(rename = "ThresholdContributionAmount")]
    pub points: u32,
    #[serde(rename = "Supersedes")]
    pub supersedes: u32,
    #[serde(rename = "Priority")]
    pub priority: i32,
}
#[derive(Debug, Deserialize)]
struct Suggestion {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "Name_lang")]
    name: String,
    #[serde(rename = "Description_lang")]
    description: String,
    #[serde(rename = "RewardDescription_lang")]
    reward: String,
    #[serde(rename = "TextureFileDataID")]
    texture: u32,
    #[serde(rename = "PlayerConditionID")]
    condition: u32,
    #[serde(rename = "BonusPlayerConditionID_0")]
    bonus0: u32,
    #[serde(rename = "BonusPlayerConditionID_1")]
    bonus1: u32,
    #[serde(rename = "ButtonActionType")]
    action: u32,
    #[serde(rename = "Flags")]
    flags: u32,
    #[serde(rename = "PriorityMin")]
    priority: u32,
    #[serde(rename = "PriorityMax")]
    priority_max: u32,
    #[serde(rename = "BonusValue_0")]
    bonus_value0: i32,
    #[serde(rename = "BonusValue_1")]
    bonus_value1: i32,
}

fn load_suggestions(path: &Path) -> Result<Vec<Suggestion>, String> {
    let mut reader = csv::Reader::from_path(path)
        .map_err(|error| format!("AdventureJournal {}: {error}", path.display()))?;
    let rows: Vec<Suggestion> = reader
        .deserialize()
        .map(|row| row.map_err(|error| format!("AdventureJournal {}: {error}", path.display())))
        .collect::<Result<_, _>>()?;
    for row in &rows {
        if row.priority != row.priority_max {
            return Err(format!(
                "AdventureJournal {}: priority range {}–{} requires unverified selection semantics",
                row.id, row.priority, row.priority_max
            ));
        }
    }
    bevy::log::info!(
        "Loaded {} installed AdventureJournal rows from {}",
        rows.len(),
        path.display()
    );
    Ok(rows)
}
#[derive(Deserialize)]
struct ActivityCondition {
    #[serde(rename = "PerksActivityID")]
    activity: u32,
    #[serde(rename = "PlayerConditionID")]
    condition: u32,
    #[serde(rename = "Field_10_2_5_52206_002")]
    kind: u32,
}
#[derive(Deserialize)]
struct Tag {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "TagName_lang")]
    name: String,
}
#[derive(Deserialize)]
struct TagLink {
    #[serde(rename = "PerksActivityID")]
    activity: u32,
    #[serde(rename = "PerksActivityTagID")]
    tag: u32,
}

#[derive(Debug)]
pub(crate) struct CatalogData {
    pub activities: Vec<Activity>,
    pub thresholds: Vec<crate::trading_post_store::Threshold>,
    pub supported_indices: Vec<usize>,
    pub supported_roots: BTreeSet<u32>,
    suggestions: Vec<Suggestion>,
    pub tags: Vec<PerksActivityTag>,
    pub activity_tags: BTreeMap<u32, Vec<u32>>,
    conditions: BTreeMap<u32, ConditionRule>,
    activity_conditions: BTreeMap<u32, Vec<(u32, u32)>>,
}

fn read_rows<T: serde::de::DeserializeOwned>(
    conn: &Connection,
    table: &str,
) -> Result<Vec<T>, String> {
    let mut query = conn
        .prepare(&format!("SELECT payload FROM {table} ORDER BY ID"))
        .map_err(|e| format!("{table}: {e}"))?;
    let rows = query
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.map(|row| {
        let json = row.map_err(|e| e.to_string())?;
        serde_json::from_str(&json).map_err(|e| format!("{table} payload: {e}"))
    })
    .collect()
}

#[derive(Deserialize)]
struct ThresholdRow {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "BonusTendies")]
    tender: u32,
    #[serde(rename = "Threshold")]
    points: u32,
    #[serde(rename = "PerksActivityThresholdGroupID")]
    group: u32,
}

fn load_thresholds(conn: &Connection) -> Result<Vec<crate::trading_post_store::Threshold>, String> {
    // Explicit Osso policy: sourced default group1, not an inferred seasonal epoch.
    let default_group = 1;
    let rows = read_rows::<ThresholdRow>(conn, "perks_activity_threshold")?;
    let thresholds: Vec<_> = rows
        .into_iter()
        .filter(|row| row.group == default_group)
        .map(|row| crate::trading_post_store::Threshold {
            id: row.id,
            points: row.points,
            tender: row.tender,
        })
        .collect();
    if thresholds.is_empty() {
        return Err("Default PerksActivityThreshold group1 is unavailable".into());
    }
    Ok(thresholds)
}

impl CatalogData {
    pub(super) fn load(
        conn: &Connection,
        criteria: &crate::achievement::AchievementCatalog,
        journal_csv: &Path,
    ) -> Result<Self, String> {
        crate::db2_source_build::read_source_build(conn, "adventure_guide_source")?;
        let activities = read_rows::<Activity>(conn, "perks_activity")?;
        let thresholds = load_thresholds(conn)?;
        let supported_roots: BTreeSet<_> = activities
            .iter()
            .filter(|a| criteria.supports_tree(a.root, &mut BTreeSet::new()))
            .map(|a| a.root)
            .collect();
        let supported_indices = activities
            .iter()
            .enumerate()
            .filter(|(_, a)| supported_roots.contains(&a.root))
            .map(|(index, _)| index)
            .collect();
        let suggestions = load_suggestions(journal_csv)?;
        let gates = read_rows::<ActivityCondition>(conn, "perks_activity_condition")?;
        let ids: BTreeSet<_> = suggestions
            .iter()
            .flat_map(|s| [s.condition, s.bonus0, s.bonus1])
            .chain(gates.iter().map(|g| g.condition))
            .collect();
        let conditions = load_conditions(conn, ids)?;
        let tags = read_rows::<Tag>(conn, "perks_activity_tag")?
            .into_iter()
            .map(|tag| PerksActivityTag {
                id: tag.id,
                name: tag.name,
            })
            .collect();
        let mut activity_tags: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for link in read_rows::<TagLink>(conn, "perks_activity_x_tag")? {
            activity_tags
                .entry(link.activity)
                .or_default()
                .push(link.tag);
        }
        let mut activity_conditions: BTreeMap<u32, Vec<(u32, u32)>> = BTreeMap::new();
        for gate in gates {
            activity_conditions
                .entry(gate.activity)
                .or_default()
                .push((gate.condition, gate.kind));
        }
        Ok(Self {
            activities,
            thresholds,
            supported_indices,
            supported_roots,
            suggestions,
            tags,
            activity_tags,
            conditions,
            activity_conditions,
        })
    }
    fn condition(
        &self,
        id: u32,
        level: u8,
        earned: Option<&BTreeSet<u32>>,
    ) -> Result<bool, String> {
        if id == 0 {
            return Ok(true);
        }
        let rule = self
            .conditions
            .get(&id)
            .ok_or_else(|| format!("PlayerCondition {id} missing"))?;
        rule.evaluate(level, earned).map_err(str::to_string)
    }
    pub(super) fn activity_eligibility(&self, id: u32, level: u8) -> Result<bool, String> {
        for &(condition, kind) in self.activity_conditions.get(&id).into_iter().flatten() {
            if kind != 0 {
                return Err(format!(
                    "Activity {id}: unknown source condition kind {kind}"
                ));
            }
            if !self.condition(condition, level, None)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn suggestions(
        &self,
        level: u8,
        earned: &BTreeSet<u32>,
    ) -> (Vec<AdventureSuggestion>, SuggestionCensus) {
        let mut census = SuggestionCensus {
            source_rows: self.suggestions.len() as u32,
            ..Default::default()
        };
        let mut ranked = Vec::new();
        for row in &self.suggestions {
            match self.suggestion_score(row, level, earned) {
                Err(_) => census.unsupported_conditions += 1,
                Ok(None) => census.failed_conditions += 1,
                Ok(Some(score)) => ranked.push((score, row)),
            }
        }
        // Named inference: Retail Lua forwards opaque C_AdventureJournal order unchanged.
        // This provider ranks authored fixed priority + met bonuses descending; ID breaks ties.
        ranked.sort_by_key(|(score, row)| (std::cmp::Reverse(*score), row.id));
        let visible = ranked
            .into_iter()
            .map(|(_, row)| {
                if row.action != 0 {
                    census.unsupported_actions += 1;
                }
                AdventureSuggestion {
                    id: row.id,
                    name: row.name.clone(),
                    description: row.description.clone(),
                    reward_description: row.reward.clone(),
                    texture_fdid: row.texture,
                    button_text: String::new(),
                }
            })
            .collect();
        (visible, census)
    }

    fn suggestion_score(
        &self,
        row: &Suggestion,
        level: u8,
        earned: &BTreeSet<u32>,
    ) -> Result<Option<i64>, String> {
        if row.flags != 0 {
            return Err(format!(
                "AdventureJournal {}: unsupported flags {}",
                row.id, row.flags
            ));
        }
        if !self.condition(row.condition, level, Some(earned))? {
            return Ok(None);
        }
        let mut score = i64::from(row.priority);
        for (condition, value) in [
            (row.bonus0, row.bonus_value0),
            (row.bonus1, row.bonus_value1),
        ] {
            // A zero-valued bonus cannot alter ordering and is not an eligibility gate.
            if value != 0 && self.condition(condition, level, Some(earned))? {
                score += i64::from(value);
            }
        }
        Ok(Some(score))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed_suggestions(ids: &[u32]) -> Vec<Suggestion> {
        let path = crate::test_data::world_db_path()
            .parent()
            .unwrap()
            .join("db2/active/AdventureJournal.csv");
        csv::Reader::from_path(path)
            .unwrap()
            .deserialize::<Suggestion>()
            .map(Result::unwrap)
            .filter(|row| ids.contains(&row.id))
            .collect()
    }

    fn suggestion_catalog(
        suggestions: Vec<Suggestion>,
        conditions: BTreeMap<u32, ConditionRule>,
    ) -> CatalogData {
        CatalogData {
            activities: vec![],
            thresholds: vec![],
            supported_indices: vec![],
            supported_roots: BTreeSet::new(),
            suggestions,
            tags: vec![],
            activity_tags: BTreeMap::new(),
            conditions,
            activity_conditions: BTreeMap::new(),
        }
    }

    #[test]
    fn suggested_provider_follows_installed_csv_instead_of_old_sqlite_payloads() {
        let path = crate::test_data::world_db_path();
        let conn =
            Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let criteria = crate::achievement::load_achievement_catalog(&conn).unwrap();
        let catalog = CatalogData::load(
            &conn,
            &criteria,
            &path
                .parent()
                .unwrap()
                .join("db2/active/AdventureJournal.csv"),
        )
        .unwrap();
        let csv_path = path
            .parent()
            .unwrap()
            .join("db2/active/AdventureJournal.csv");
        let source_count = csv::Reader::from_path(csv_path)
            .unwrap()
            .records()
            .map(Result::unwrap)
            .count();
        assert_eq!(
            catalog.suggestions(80, &BTreeSet::new()).1.source_rows as usize,
            source_count
        );
    }

    #[test]
    fn suggested_provider_orders_equal_base_priorities_by_evaluated_authored_bonus() {
        // Actual AdventureJournal7/8 both have base250; their first bonus is1.
        let catalog = suggestion_catalog(
            installed_suggestions(&[7, 8]),
            BTreeMap::from([
                (33200, ConditionRule::Always),
                (33201, ConditionRule::Always),
                (33600, ConditionRule::Level { min: 90, max: 0 }),
                (33636, ConditionRule::Always),
            ]),
        );
        let (rows, census) = catalog.suggestions(80, &BTreeSet::new());
        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![8, 7]
        );
        assert_eq!(census.unsupported_conditions, 0);
        assert!(rows.iter().all(|row| row.button_text.is_empty()));
    }

    #[test]
    fn suggested_provider_zero_value_bonus_does_not_hide_base_eligible_record() {
        // Actual Wailing Caverns9 bonus conditions have authored value0.
        let catalog = suggestion_catalog(
            installed_suggestions(&[9]),
            BTreeMap::from([
                (33956, ConditionRule::Always),
                (
                    33955,
                    ConditionRule::Unsupported("Unevaluated bonus predicate".into()),
                ),
            ]),
        );
        let (rows, census) = catalog.suggestions(20, &BTreeSet::new());
        assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), vec![9]);
        assert_eq!(census.unsupported_conditions, 0);
    }

    #[test]
    fn eligible_authored_suggestion_keeps_card_without_unsupported_activation() {
        // Authored AdventureJournal7; all three condition results are test inputs.
        let catalog = CatalogData {
            activities: vec![], thresholds: vec![], supported_indices: vec![], supported_roots: BTreeSet::new(),
            suggestions: vec![Suggestion {
                id: 7, name: "Hillsbrad Foothills".into(),
                description: "Join the Forsaken war machine as it marches on Hillsbrad Foothills for the Horde!".into(),
                reward: String::new(), texture: 236779,
                condition: 33200, bonus0: 33600, bonus1: 33200, action: 3, flags: 0, priority: 250, priority_max: 250, bonus_value0: 1, bonus_value1: 0,
            }],
            tags: vec![], activity_tags: BTreeMap::new(),
            conditions: BTreeMap::from([(33200, ConditionRule::Always), (33600, ConditionRule::Always)]),
            activity_conditions: BTreeMap::new(),
        };
        let (rows, census) = catalog.suggestions(80, &BTreeSet::new());
        assert_eq!(rows.len(), 1);
        assert_eq!(
            (&*rows[0].name, rows[0].texture_fdid),
            ("Hillsbrad Foothills", 236779)
        );
        assert!(rows[0].button_text.is_empty());
        assert_eq!(census.unsupported_actions, 1);
    }
}
