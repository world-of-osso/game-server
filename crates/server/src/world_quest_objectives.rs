//! Authenticated objective flags/weights beside the existing dynamic quest counts.
use crate::quest_data::{ObjectiveDef, ObjectiveKind, QuestTemplate};

const OPTIONAL: u32 = 0x4;
const HIDDEN: u32 = 0x8;
const CONTRIBUTOR: u32 = 0x40;
const SEQUENCED: u32 = 0x2;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct ObjectiveRule {
    pub flags: u32,
    pub flags2: u32,
    pub weight: f32,
}

pub(crate) fn load(
    conn: &rusqlite::Connection,
    db: &mut crate::quest_data::QuestDb,
) -> rusqlite::Result<()> {
    let installed: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='content_world_quest')",[],|row| row.get(0))?;
    if !installed {
        return Ok(());
    }
    load_creature_labels(conn, db)?;
    load_criterion_bindings(conn, db)?;
    let mut statement=conn.prepare("SELECT o.quest_id,o.kind,o.object_id,o.amount,o.flags,o.flags2,o.weight,o.description,COALESCE(c.name,''),COALESCE(i.name,'') FROM content_world_quest_objective o JOIN content_world_quest q ON q.quest_id=o.quest_id LEFT JOIN tdb_creature_template c ON o.kind=0 AND c.entry=o.object_id LEFT JOIN content_item i ON o.kind=1 AND i.ID=o.object_id ORDER BY o.quest_id,o.ordinal")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u8>(1)?,
            row.get::<_, u32>(2)?,
            row.get::<_, u32>(3)?,
            ObjectiveRule {
                flags: row.get(4)?,
                flags2: row.get(5)?,
                weight: row.get(6)?,
            },
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
        ))
    })?;
    for row in rows {
        let (quest, raw_kind, object_id, amount, rule, description, creature, item) = row?;
        let kind = match raw_kind {
            0 => ObjectiveKind::Monster,
            1 => ObjectiveKind::Item,
            2 => ObjectiveKind::GameObject,
            3 => ObjectiveKind::TalkTo,
            9 => ObjectiveKind::PlayerKills,
            14 => ObjectiveKind::CriteriaTree,
            15 => ObjectiveKind::ProgressBar,
            19 => ObjectiveKind::AreaTriggerEnter,
            21 => ObjectiveKind::KillWithLabel,
            _ => return Err(rusqlite::Error::InvalidQuery),
        };
        let template = db
            .quests
            .get_mut(&quest)
            .ok_or(rusqlite::Error::InvalidQuery)?;
        if template.world_objective_rules.is_none() {
            template.objectives.clear();
            template.world_objective_rules = Some(Vec::new());
        }
        let text = if !description.is_empty() {
            description
        } else {
            match kind {
                ObjectiveKind::Monster if !creature.is_empty() => format!("{creature} slain"),
                ObjectiveKind::Item => item,
                ObjectiveKind::ProgressBar => "Progress".into(),
                _ => String::new(),
            }
        };
        template.objectives.push(ObjectiveDef {
            kind,
            object_id,
            required: match kind {
                ObjectiveKind::ProgressBar => 100,
                ObjectiveKind::AreaTriggerEnter => 1, // Player.cpp17226: stored boolean.
                _ => amount,
            },
            text,
        });
        template
            .world_objective_rules
            .as_mut()
            .expect("initialized objective rules")
            .push(rule);
    }
    Ok(())
}

fn load_criterion_bindings(
    conn: &rusqlite::Connection,
    db: &mut crate::quest_data::QuestDb,
) -> rusqlite::Result<()> {
    let installed: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='content_world_quest_criterion')",
        [],
        |row| row.get(0),
    )?;
    if !installed {
        return Ok(());
    }
    let mut statement =
        conn.prepare("SELECT root_id,kind,asset FROM content_world_quest_criterion")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            row.get::<_, u32>(2)?,
        ))
    })?;
    for row in rows {
        let (root, kind, asset) = row?;
        db.task_criteria.insert(root, (kind, asset));
    }
    Ok(())
}

fn load_creature_labels(
    conn: &rusqlite::Connection,
    db: &mut crate::quest_data::QuestDb,
) -> rusqlite::Result<()> {
    let installed:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='content_world_quest_creature_label')",[],|row|row.get(0))?;
    if !installed {
        return Ok(());
    }
    let mut statement=conn.prepare("SELECT creature_entry,difficulty_id,label_id FROM content_world_quest_creature_label ORDER BY creature_entry,difficulty_id,label_id")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            row.get::<_, u32>(2)?,
        ))
    })?;
    // Replace, not append: reloading the catalog must not double label credit.
    let mut labels: std::collections::HashMap<(u32, u32), Vec<u32>> = Default::default();
    for row in rows {
        let (entry, difficulty, label) = row?;
        labels.entry((entry, difficulty)).or_default().push(label);
    }
    db.creature_labels = labels;
    Ok(())
}

impl ObjectiveRule {
    pub(crate) fn quest_bound_item(self) -> bool {
        self.flags2 & 1 != 0
    }
    pub(crate) fn required(self) -> bool {
        self.flags & (OPTIONAL | CONTRIBUTOR) == 0
    }
    pub(crate) fn hidden(self) -> bool {
        self.flags & (HIDDEN | CONTRIBUTOR) != 0
    }
    pub(crate) fn optional(self) -> bool {
        self.flags & OPTIONAL != 0
    }
}

pub(crate) fn rule(quest: &QuestTemplate, index: usize) -> ObjectiveRule {
    match &quest.world_objective_rules {
        None => ObjectiveRule::default(), // Ordinary quest representation has no TDB task flags.
        Some(rules) => rules[index],
    }
}

pub(crate) fn weighted_percent(quest: &QuestTemplate, counts: &[u32]) -> f32 {
    let rules = quest
        .world_objective_rules
        .as_ref()
        .expect("progress bar requires authored objective rules");
    rules
        .iter()
        .zip(counts)
        .filter(|(rule, _)| rule.flags & CONTRIBUTOR != 0)
        .map(|(rule, count)| *count as f32 * rule.weight)
        .sum::<f32>()
        .min(100.)
}

pub(crate) fn derive_progress_bars(quest: &QuestTemplate, counts: &mut [u32]) {
    if !quest
        .objectives
        .iter()
        .any(|objective| objective.kind == ObjectiveKind::ProgressBar)
    {
        return;
    }
    let percent = weighted_percent(quest, counts).floor() as u32;
    for (objective, count) in quest.objectives.iter().zip(counts) {
        if objective.kind == ObjectiveKind::ProgressBar {
            *count = percent;
        }
    }
}

/// Pinned Player::IsQuestObjectiveCompletable: contributors delegate to their bar;
/// sequence boundaries apply to credit, not just presentation.
pub(crate) fn creditable(quest: &QuestTemplate, counts: &[u32], index: usize) -> bool {
    let metadata = rule(quest, index);
    if metadata.flags & CONTRIBUTOR != 0 {
        return quest
            .objectives
            .iter()
            .enumerate()
            .find(|(i, objective)| {
                objective.kind == ObjectiveKind::ProgressBar
                    && rule(quest, *i).flags & CONTRIBUTOR == 0
            })
            .is_some_and(|(bar, _)| {
                weighted_percent(quest, counts) < 100. && creditable(quest, counts, bar)
            });
    }
    let mut sequence_satisfied = true;
    for before in (0..index).rev() {
        let previous = rule(quest, before);
        let current = if quest.objectives[before].kind == ObjectiveKind::ProgressBar {
            weighted_percent(quest, counts).floor() as u32
        } else {
            counts[before]
        };
        let complete = current >= quest.objectives[before].required;
        if previous.flags & SEQUENCED != 0 {
            return if metadata.flags & SEQUENCED != 0 {
                complete && sequence_satisfied
            } else {
                complete || creditable(quest, counts, before)
            };
        }
        sequence_satisfied &= complete || !previous.required();
    }
    metadata.flags & SEQUENCED == 0 || sequence_satisfied
}

pub(crate) fn visible(quest: &QuestTemplate, counts: &[u32], index: usize) -> bool {
    let metadata = rule(quest, index);
    if metadata.hidden() {
        return false;
    }
    metadata.flags & SEQUENCED == 0
        || quest.objectives[..index]
            .iter()
            .zip(&counts[..index])
            .enumerate()
            .all(|(before, (objective, count))| {
                !rule(quest, before).required() || *count >= objective.required
            })
}
