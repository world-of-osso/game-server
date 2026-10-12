//! Authenticated objective flags/weights beside the existing dynamic quest counts.
use crate::quest_data::{ObjectiveKind, QuestTemplate};

const OPTIONAL: u32 = 0x4;
const HIDDEN: u32 = 0x8;
const CONTRIBUTOR: u32 = 0x40;
const SEQUENCED: u32 = 0x2;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct ObjectiveRule {
    pub flags: u32,
    pub weight: f32,
}

impl ObjectiveRule {
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
