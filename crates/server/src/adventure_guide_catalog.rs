use super::conditions::{ConditionRule, load_conditions};
use rusqlite::Connection;
use serde::Deserialize;
use shared::protocol::{AdventureSuggestion, PerksActivityTag, SuggestionCensus};
use std::collections::{BTreeMap, BTreeSet};

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
        let suggestions = read_rows::<Suggestion>(conn, "adventure_journal")?;
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
    fn condition(&self, id: u32, level: u8) -> Result<bool, String> {
        if id == 0 {
            return Ok(true);
        }
        let rule = self
            .conditions
            .get(&id)
            .ok_or_else(|| format!("PlayerCondition {id} missing"))?;
        rule.evaluate(level).map_err(str::to_string)
    }
    pub(super) fn activity_eligibility(&self, id: u32, level: u8) -> Result<bool, String> {
        for &(condition, kind) in self.activity_conditions.get(&id).into_iter().flatten() {
            if kind != 0 {
                return Err(format!(
                    "Activity {id}: unknown source condition kind {kind}"
                ));
            }
            if !self.condition(condition, level)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn suggestions(&self, level: u8) -> (Vec<AdventureSuggestion>, SuggestionCensus) {
        let mut census = SuggestionCensus {
            source_rows: self.suggestions.len() as u32,
            ..Default::default()
        };
        let mut rows: Vec<_> = self.suggestions.iter().collect();
        rows.sort_by_key(|row| (std::cmp::Reverse(row.priority), row.id));
        let mut visible = Vec::new();
        for row in rows {
            let gates = [row.condition, row.bonus0, row.bonus1].map(|id| self.condition(id, level));
            if row.flags != 0 || gates.iter().any(Result::is_err) {
                census.unsupported_conditions += 1;
                continue;
            }
            if !gates[0].as_ref().is_ok_and(|met| *met) {
                census.failed_conditions += 1;
                continue;
            }
            // Keep the authored eligible card, but never infer Quest/RPE/LFG activation.
            // Unsupported actions count a subset of visible cards, not discarded rows.
            if row.action != 0 {
                census.unsupported_actions += 1;
            }
            visible.push(AdventureSuggestion {
                id: row.id,
                name: row.name.clone(),
                description: row.description.clone(),
                reward_description: row.reward.clone(),
                texture_fdid: row.texture,
                button_text: String::new(),
            });
        }
        (visible, census)
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
        let catalog = CatalogData::load(&conn, &criteria).unwrap();
        let csv_path = path
            .parent()
            .unwrap()
            .join("db2/active/AdventureJournal.csv");
        let source_count = csv::Reader::from_path(csv_path)
            .unwrap()
            .records()
            .map(Result::unwrap)
            .count();
        assert_eq!(catalog.suggestions(80).1.source_rows as usize, source_count);
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
        let (rows, census) = catalog.suggestions(80);
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
        let (rows, census) = catalog.suggestions(20);
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
                condition: 33200, bonus0: 33600, bonus1: 33200, action: 3, flags: 0, priority: 250,
            }],
            tags: vec![], activity_tags: BTreeMap::new(),
            conditions: BTreeMap::from([(33200, ConditionRule::Always), (33600, ConditionRule::Always)]),
            activity_conditions: BTreeMap::new(),
        };
        let (rows, census) = catalog.suggestions(80);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            (&*rows[0].name, rows[0].texture_fdid),
            ("Hillsbrad Foothills", 236779)
        );
        assert!(rows[0].button_text.is_empty());
        assert_eq!(census.unsupported_actions, 1);
    }
}
