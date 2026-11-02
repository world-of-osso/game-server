//! Client-derived seasonal tree replacement and bonus-list group selection.
use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize)]
struct Season {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "StartTimeEvent")]
    start_event: u32,
}

#[derive(Clone, Debug, Deserialize)]
struct Override {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "SrcItemBonusTreeID")]
    source: u32,
    #[serde(rename = "ItemBonusTreeGroupID")]
    group: u32,
    #[serde(rename = "DstItemBonusTreeID")]
    destination: u32,
    #[serde(rename = "Value")]
    threshold: i32,
    #[serde(rename = "RequiredTimeEventPassed")]
    required: u32,
    #[serde(rename = "RequiredTimeEventNotPassed")]
    excluded: u32,
}

#[derive(Deserialize)]
struct TreeGroup {
    #[serde(rename = "ItemBonusTreeID")]
    tree: u32,
    #[serde(rename = "ItemBonusTreeGroupID")]
    group: u32,
}

#[derive(Clone, Debug, Deserialize)]
struct GroupEntry {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "ItemBonusListGroupID")]
    group: u32,
    #[serde(rename = "ItemBonusListID")]
    list: u32,
    #[serde(rename = "ItemLevelSelectorID")]
    selector: u32,
    #[serde(rename = "SequenceValue")]
    sequence: i32,
}

pub(super) struct Selection {
    overrides: Vec<Override>,
    tree_groups: HashMap<u32, Vec<u32>>,
    groups: HashMap<u32, Vec<GroupEntry>>,
    active_events: Vec<u32>,
}

impl Selection {
    pub(super) fn read(db2: &Path) -> Result<Self, String> {
        let data = db2
            .parent()
            .and_then(Path::parent)
            .ok_or("DB2 data root unavailable")?;
        let weeks = std::env::var_os("GAME_SERVER_MYTHIC_PLUS_WEEKS")
            .map_or_else(|| data.join("mythic_plus_weeks.json"), Into::into);
        let season_id = crate::mythic_plus_weeks::AffixSchedule::read(&weeks)?.season_id;
        let seasons: Vec<_> = super::super::read_rows::<Season>(db2, "MythicPlusSeason.csv")?
            .into_iter()
            .filter(|season| season.id == season_id && season.start_event != 0)
            .collect();
        let [season] = seasons.as_slice() else {
            return Err(format!("season{season_id} has no unique start time event"));
        };
        let mut overrides =
            super::super::read_rows::<Override>(db2, "ChallengeModeItemBonusOverride.csv")?;
        overrides.sort_by_key(|row| row.id);
        let mut tree_groups: HashMap<u32, Vec<u32>> = HashMap::new();
        for row in super::super::read_rows::<TreeGroup>(db2, "ItemBonusTreeGroupEntry.csv")? {
            tree_groups.entry(row.tree).or_default().push(row.group);
        }
        let mut groups: HashMap<u32, Vec<GroupEntry>> = HashMap::new();
        for row in super::super::read_rows::<GroupEntry>(db2, "ItemBonusListGroupEntry.csv")? {
            groups.entry(row.group).or_default().push(row);
        }
        // INFERRED: the configured season's start event is active. Retail M0
        // 250215 at ilvl292 corroborates 1552 for Season 2, but does not prove
        // the full ordered history of client time events.
        // https://gamgem.com/world-of-warcraft/items/freightrunners-flask
        Ok(Self {
            overrides,
            tree_groups,
            groups,
            active_events: vec![season.start_event],
        })
    }

    pub(super) fn tree(&self, tree: u32, context: u32) -> Result<u32, String> {
        if context == 16 {
            return Err("keystone preview level is required for item tree overrides".into());
        }
        let groups = self
            .tree_groups
            .get(&tree)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let candidates: Vec<_> = self
            .overrides
            .iter()
            .filter(|row| row.source == tree && tree != 0)
            .chain(
                self.overrides
                    .iter()
                    .filter(|row| row.group != 0 && groups.contains(&row.group)),
            )
            .collect();
        Ok(select_override(tree, &candidates, &self.active_events, -1))
    }

    pub(super) fn group_list(
        &self,
        group: u32,
        key: u32,
        context: u32,
    ) -> Result<Option<u32>, String> {
        if context == 108 {
            return Err("item context108 group sequence source is unresolved".into());
        }
        let entries = self
            .groups
            .get(&group)
            .ok_or_else(|| format!("item bonus group{group} unavailable"))?;
        let sequence = match key {
            3091 => 0,
            2962..=2971 => (key - 2961) as i32,
            _ => 0,
        };
        // INFERRED: native enumeration is DB2 ID order. Decompilation confirms
        // first eligible entry, but not that order (R-item-bonus-season-override).
        let entry = entries
            .iter()
            .filter(|entry| entry.sequence == sequence || sequence <= 0 && entry.sequence > 0)
            .min_by_key(|entry| entry.id);
        if let Some(entry) = entry {
            if entry.selector != 0 {
                return Err(format!(
                    "item bonus group{group} level selector{} is unresolved",
                    entry.selector
                ));
            }
            return Ok((entry.list != 0).then_some(entry.list));
        }
        Ok(None)
    }
}

/// Build 70688 context-aware override selection (R-item-bonus-season-override).
fn select_override(tree: u32, candidates: &[&Override], active: &[u32], threshold: i32) -> u32 {
    let relevant = active
        .iter()
        .rev()
        .find(|event| {
            candidates
                .iter()
                .any(|row| row.required != 0 && row.required == **event)
        })
        .and_then(|event| active.iter().position(|entry| entry == event));
    let Some(relevant) = relevant else {
        return tree;
    };
    let mut winner = None;
    for (order, row) in candidates.iter().enumerate() {
        let Some(position) = (row.required != 0)
            .then(|| active.iter().position(|event| *event == row.required))
            .flatten()
        else {
            continue;
        };
        if position > relevant
            || row.threshold > threshold
            || row.excluded != 0
                && active
                    .iter()
                    .position(|event| *event == row.excluded)
                    .is_some_and(|index| index > 0)
        {
            continue;
        }
        let key = (position, row.threshold, order);
        if winner.as_ref().is_none_or(|(old, _)| key > *old) {
            winner = Some((key, row.destination));
        }
    }
    match winner.map(|(_, destination)| destination) {
        Some(destination) if destination != 0 => destination,
        _ => tree,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: u32, required: u32, excluded: u32, threshold: i32, destination: u32) -> Override {
        Override {
            id,
            source: 1,
            group: 0,
            destination,
            threshold,
            required,
            excluded,
        }
    }

    #[test]
    fn override_uses_event_order_threshold_and_later_ties() {
        let rows = [
            row(1, 11, 0, -1, 2),
            row(2, 22, 0, -1, 3),
            row(3, 22, 0, -1, 4),
            row(4, 22, 33, -1, 5),
            row(5, 22, 0, 0, 6),
            row(6, 0, 0, -1, 7),
        ];
        let refs: Vec<_> = rows.iter().collect();
        // Excluded event index zero does not veto; event at index two does.
        assert_eq!(select_override(1, &refs, &[33, 11, 22], -1), 5);
        assert_eq!(select_override(1, &refs, &[11, 22, 33], -1), 4);
        assert_eq!(select_override(1, &refs, &[11], -1), 2);
        assert_eq!(select_override(1, &refs, &[44], -1), 1);
        let zero = row(7, 22, 0, -1, 0);
        assert_eq!(select_override(1, &[&rows[1], &zero], &[22], -1), 1);
    }

    #[test]
    fn group_selects_exact_sequence_or_first_eligible_zero_sequence() {
        let entry = |id, sequence, list| GroupEntry {
            id,
            group: 616,
            list,
            selector: 0,
            sequence,
        };
        let selection = Selection {
            overrides: vec![],
            tree_groups: HashMap::new(),
            active_events: vec![],
            groups: HashMap::from([(
                616,
                vec![entry(3, 2, 12834), entry(1, 1, 12833), entry(2, 0, 12832)],
            )]),
        };
        assert_eq!(selection.group_list(616, 2962, 23).unwrap(), Some(12833));
        assert_eq!(selection.group_list(616, 2963, 23).unwrap(), Some(12834));
        assert_eq!(selection.group_list(616, 3091, 23).unwrap(), Some(12833));
        assert_eq!(selection.group_list(616, 2971, 23).unwrap(), None);
        assert!(selection.group_list(616, 2962, 108).is_err());
    }
}
