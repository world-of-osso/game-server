//! Authored decor-reward quests. TDB objective14 stays CriteriaTree, not an item hand-in.
//! TC a352b1fa: AcquireItem36 accumulates the actual acquired quantity; its item asset
//! must match. CompleteAll4 with one unconditional leaf is the supported tree boundary.
use crate::persistence::quest_store::QuestLogData;
use crate::quest_data::{ObjectiveDef, ObjectiveKind, QuestDb};
use rusqlite::Connection;
use std::collections::BTreeSet;

const ACQUIRE_ITEM: u32 = 36;

pub(crate) fn load(source: &Connection, quests: &mut QuestDb) -> rusqlite::Result<()> {
    let installed: bool = source.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='housing_catalog_record')",
        [],
        |row| row.get(0),
    )?;
    if !installed {
        return Ok(());
    }
    let build_key = crate::housing_catalog::read_build_key(source).map_err(|error| {
        bevy::log::error!("Housing quest catalog: {error}");
        rusqlite::Error::InvalidQuery
    })?;
    let mut query=source.prepare("SELECT CAST(json_extract(fields_json,'$.ItemID') AS INTEGER) FROM housing_catalog_record WHERE build_key=? AND table_name='HouseDecor'")?;
    let items: BTreeSet<u32> = query
        .query_map([build_key], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let candidates: Vec<_> = quests
        .quests
        .values()
        .filter(|quest| {
            quest
                .reward_items
                .iter()
                .chain(&quest.choice_items)
                .any(|(item, _)| items.contains(item))
        })
        .map(|quest| quest.id)
        .collect();
    for id in candidates {
        load_objectives(source, quests, id)?;
    }
    Ok(())
}

fn load_objectives(source: &Connection, quests: &mut QuestDb, id: u32) -> rusqlite::Result<()> {
    let mut query=source.prepare("SELECT Type,ObjectID,Amount,Flags,Flags2,COALESCE(Description,'') FROM tdb_quest_objectives WHERE QuestID=? ORDER BY \"Order\",ID")?;
    let rows = query
        .query_map([id], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, u32>(3)?,
                row.get::<_, u32>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut objectives = Vec::new();
    let mut supported = !rows.is_empty();
    for (kind, root, amount, flags, flags2, text) in rows {
        let asset = if kind == 14 && amount == 1 && flags == 0 && flags2 == 0 {
            acquisition_asset(source, root)?
        } else {
            None
        };
        if let Some(item) = asset {
            quests.task_criteria.insert(root, (ACQUIRE_ITEM, item));
            objectives.push(ObjectiveDef {
                kind: ObjectiveKind::CriteriaTree,
                object_id: root,
                required: amount,
                text,
            });
        } else {
            supported = false;
            bevy::log::warn!(
                "Housing quest{id} objective{root}: unsupported authored criteria/conditions; completion blocked"
            );
        }
    }
    let quest = quests.quests.get_mut(&id).expect("candidate quest exists");
    quest.completion_blocked = !supported;
    quest.objectives = objectives;
    quest.world_objective_rules = None;
    Ok(())
}

fn acquisition_asset(source: &Connection, root: u32) -> rusqlite::Result<Option<u32>> {
    let mut query=source.prepare("SELECT c.Asset FROM criteria_tree r JOIN criteria_tree t ON t.Parent=r.ID JOIN criteria c ON c.ID=t.CriteriaID
        WHERE r.ID=? AND r.Parent=0 AND r.Operator=4 AND r.CriteriaID=0 AND r.Amount=0 AND r.Flags=0
        AND t.Operator=0 AND t.Amount=0 AND t.Flags=0
        AND c.Type=36 AND c.ModifierTreeId=0 AND c.StartEvent=0 AND c.StartTimer=0 AND c.FailEvent=0
        AND c.Flags=32 AND c.EligibilityWorldStateID=0 AND c.EligibilityWorldStateValue=0
        AND (SELECT COUNT(*) FROM criteria_tree children WHERE children.Parent=r.ID)=1
        AND NOT EXISTS(SELECT 1 FROM criteria_tree children WHERE children.Parent=t.ID)")?;
    let items = query
        .query_map([root], |row| row.get::<_, u32>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(items.first().copied())
}

/// Called only after the normal paid vendor transaction has delivered `quantity`.
/// No bag item, decor, price, criterion or quest completion is fabricated here.
pub(crate) fn credit_acquired_item(
    quests: &QuestDb,
    log: &mut QuestLogData,
    item: u32,
    quantity: u32,
) -> bool {
    let mut changed = false;
    for active in &mut log.active {
        let Some(quest) = quests.quests.get(&active.quest_id) else {
            continue;
        };
        if quest.completion_blocked || active.failed {
            continue;
        }
        active.counts.resize(quest.objectives.len(), 0);
        for (index, objective) in quest.objectives.iter().enumerate() {
            if objective.kind != ObjectiveKind::CriteriaTree
                || quests.task_criteria.get(&objective.object_id) != Some(&(ACQUIRE_ITEM, item))
            {
                continue;
            }
            let count = active.counts[index]
                .saturating_add(quantity)
                .min(objective.required);
            changed |= count != active.counts[index];
            active.counts[index] = count;
        }
    }
    changed
}
