//! FOLLOWER_EXPECTED_STAT_PLAYER_BUDGET: approved project policy, not Retail NPC gear.
use shared::components::UnitStats;

use crate::character_stats_data::EXPECTED_PLAYER_PRIMARY;
use crate::stats::{PrimaryStat, base_primary_stats};

/// Put only the level's total primary-budget delta through the existing gear path.
pub(crate) fn primary_budget_bonus(
    class: u8,
    level: u8,
    primary: PrimaryStat,
) -> Option<UnitStats> {
    let index = usize::from(level.checked_sub(1)?);
    let expected = *EXPECTED_PLAYER_PRIMARY.get(index)?;
    let base = base_primary_stats(class, 0, level)?;
    Some(match primary {
        PrimaryStat::Strength => UnitStats {
            strength: expected - base.strength,
            ..Default::default()
        },
        PrimaryStat::Agility => UnitStats {
            agility: expected - base.agility,
            ..Default::default()
        },
        PrimaryStat::Intellect => UnitStats {
            intellect: expected - base.intellect,
            ..Default::default()
        },
    })
}
