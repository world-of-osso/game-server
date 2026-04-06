//! XP and leveling formulas.
//!
//! Kill XP scales with creature level and is modified by the level difference
//! between the player and the creature. Grey mobs (too low level) give 0 XP.
//!
//! Ref: AzerothCore `Player.cpp` GiveXP, `Formulas.h` XP calculations.

/// Base XP for killing a creature of a given level.
///
/// Formula: `5 * level + 45` — matches AzerothCore's `MaNGOS::XP::BaseGain`.
pub fn base_kill_xp(creature_level: u8) -> u32 {
    5 * creature_level as u32 + 45
}

/// Grey level threshold: creatures at or below this level give 0 XP.
///
/// Ref: AzerothCore `GetGrayLevel()`.
pub fn grey_level(player_level: u8) -> u8 {
    match player_level {
        0..=5 => 0,
        6..=39 => player_level - 5 - player_level / 10,
        40..=59 => player_level - 1 - player_level / 5,
        _ => player_level.saturating_sub(9),
    }
}

/// Zero-difference XP range: level difference within which XP is not reduced.
///
/// Ref: AzerothCore `GetZeroDifference()`.
fn zero_difference(player_level: u8) -> u8 {
    match player_level {
        0..=7 => 5,
        8..=9 => 6,
        10..=11 => 7,
        12..=15 => 8,
        16..=19 => 9,
        20..=29 => 11,
        30..=39 => 12,
        40..=44 => 13,
        45..=49 => 14,
        50..=54 => 15,
        55..=59 => 16,
        _ => 17,
    }
}

/// Calculate kill XP for a player killing a creature.
///
/// Returns 0 if the creature is grey (too low level).
/// XP is reduced when the creature is lower level than the player
/// (within the non-grey range), and slightly increased for higher-level mobs.
///
/// Ref: AzerothCore `Formulas.h::XP::Gain`.
pub fn kill_xp(player_level: u8, creature_level: u8) -> u32 {
    if creature_level == 0 || player_level == 0 {
        return 0;
    }

    // Grey check
    if creature_level <= grey_level(player_level) {
        return 0;
    }

    let base = base_kill_xp(creature_level);

    let diff = player_level as i32 - creature_level as i32;
    if diff <= 0 {
        // Creature is same level or higher — small bonus
        let bonus = (-diff).min(4) as u32;
        return base + bonus * base / 20; // +5% per level above
    }

    // Creature is lower — reduce XP based on zero-difference window
    let zd = zero_difference(player_level) as i32;
    if diff > zd {
        // Should have been caught by grey check, but safety
        return 0;
    }

    // Linear reduction: XP * (1 - diff / (zd + 1))
    let reduction_denom = zd + 1;
    base * (reduction_denom - diff) as u32 / reduction_denom as u32
}

// --- Group XP ---

/// Maximum distance (yards) from the kill to receive group XP.
pub const GROUP_XP_RANGE: f32 = 100.0;

/// Group XP bonus multiplier by number of nearby members.
///
/// Ref: AzerothCore group XP bonus — compensates for the split so
/// grouping is beneficial rather than punishing.
fn group_bonus_multiplier(nearby_count: u8) -> f32 {
    match nearby_count {
        0 | 1 => 1.0,
        2 => 1.0,
        3 => 1.166,
        4 => 1.3,
        5 => 1.4,
        _ => 1.4, // cap at 5-man party bonus
    }
}

/// A nearby group member eligible for XP.
#[derive(Debug, Clone, Copy)]
pub struct GroupMemberXp {
    /// Player level.
    pub level: u8,
    /// Distance from the kill in yards.
    pub distance: f32,
}

/// Result of group XP distribution for one player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupXpShare {
    pub level: u8,
    pub xp: u32,
}

/// Calculate group XP shares from a kill.
///
/// XP is split among nearby members (within `GROUP_XP_RANGE`) weighted by
/// level, then multiplied by the group bonus. Out-of-range members get 0.
///
/// Ref: AzerothCore `Group::GroupRate`, level-weighted distribution.
pub fn group_kill_xp(creature_level: u8, members: &[GroupMemberXp]) -> Vec<GroupXpShare> {
    let nearby: Vec<&GroupMemberXp> = members
        .iter()
        .filter(|m| m.distance <= GROUP_XP_RANGE)
        .collect();

    if nearby.is_empty() {
        return members
            .iter()
            .map(|m| GroupXpShare { level: m.level, xp: 0 })
            .collect();
    }

    // Use the highest level in range for base XP calculation
    let max_level = nearby.iter().map(|m| m.level).max().unwrap();
    let base = kill_xp(max_level, creature_level);

    let total_levels: u32 = nearby.iter().map(|m| m.level as u32).sum();
    let bonus = group_bonus_multiplier(nearby.len() as u8);
    let pool = (base as f32 * bonus) as u32;

    members
        .iter()
        .map(|m| {
            if m.distance > GROUP_XP_RANGE {
                return GroupXpShare { level: m.level, xp: 0 };
            }
            let share = pool * m.level as u32 / total_levels;
            GroupXpShare { level: m.level, xp: share }
        })
        .collect()
}

// --- Rested XP ---

/// Rested XP accumulation rate: fraction of a level per 8 hours in a rest area.
const RESTED_RATE_PER_8H: f32 = 0.05;
/// Maximum rested XP as a fraction of 1.5 levels.
const RESTED_MAX_LEVELS: f32 = 1.5;

/// Rested XP state for a player.
///
/// Accumulates while in a rest area (city/inn) or logged off.
/// When active, kill XP is doubled until the rested pool is depleted.
/// Cap: 1.5 levels worth of XP.
///
/// Ref: AzerothCore `Player::SetRestBonus()`, `GetXPRestBonus()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RestedXp {
    /// Current rested XP pool.
    pub amount: u32,
    /// Maximum rested XP (1.5 levels of XP, recalculated on level-up).
    pub max: u32,
    /// Whether the player is currently in a rest area.
    pub in_rest_area: bool,
}

impl RestedXp {
    /// Create a new rested XP tracker for a given XP-to-level value.
    pub fn new(xp_to_level: u32) -> Self {
        Self {
            amount: 0,
            max: (xp_to_level as f32 * RESTED_MAX_LEVELS) as u32,
            in_rest_area: false,
        }
    }

    /// Update the max when the player levels up.
    pub fn update_max(&mut self, xp_to_level: u32) {
        self.max = (xp_to_level as f32 * RESTED_MAX_LEVELS) as u32;
        self.amount = self.amount.min(self.max);
    }

    /// Accumulate rested XP from time spent in a rest area.
    ///
    /// `hours` is real time spent resting (in city/inn or logged off in rest area).
    pub fn accumulate(&mut self, hours: f32, xp_to_level: u32) {
        let gain_per_8h = xp_to_level as f32 * RESTED_RATE_PER_8H;
        let gained = (gain_per_8h * hours / 8.0) as u32;
        self.amount = (self.amount + gained).min(self.max);
    }

    /// Apply rested bonus to a kill XP amount.
    ///
    /// Returns `(total_xp, rested_consumed)`. Total XP is up to 2x the base,
    /// limited by available rested pool.
    pub fn apply_bonus(&mut self, base_xp: u32) -> (u32, u32) {
        if self.amount == 0 {
            return (base_xp, 0);
        }
        let bonus = base_xp.min(self.amount);
        self.amount -= bonus;
        (base_xp + bonus, bonus)
    }

    /// Whether the player has any rested XP.
    pub fn is_rested(&self) -> bool {
        self.amount > 0
    }

    /// Rested XP as a fraction of the current level (0.0–1.5).
    pub fn rested_levels(&self, xp_to_level: u32) -> f32 {
        if xp_to_level == 0 {
            return 0.0;
        }
        self.amount as f32 / xp_to_level as f32
    }
}

// --- Quest XP ---

/// Number of levels above quest level before XP starts reducing.
const QUEST_XP_GRACE_LEVELS: u8 = 5;
/// Number of levels above grace before quest XP reaches zero.
const QUEST_XP_DECAY_LEVELS: u8 = 10;

/// Calculate quest XP reward for a player.
///
/// - `base_xp`: the quest's fixed XP reward (from quest data).
/// - `quest_level`: the quest's recommended level.
/// - `player_level`: the player's current level.
///
/// If the player is within `QUEST_XP_GRACE_LEVELS` above the quest level,
/// full XP is awarded. Beyond that, XP decays linearly over
/// `QUEST_XP_DECAY_LEVELS`, reaching 0 when the player is 15+ levels above.
///
/// Players below the quest level always get full XP.
///
/// Ref: AzerothCore `Player::GetQuestRate()`, retail quest XP scaling.
pub fn quest_xp(base_xp: u32, quest_level: u8, player_level: u8) -> u32 {
    if base_xp == 0 {
        return 0;
    }
    let over = player_level.saturating_sub(quest_level);
    if over <= QUEST_XP_GRACE_LEVELS {
        return base_xp;
    }
    let decay = over - QUEST_XP_GRACE_LEVELS;
    if decay >= QUEST_XP_DECAY_LEVELS {
        return 0;
    }
    base_xp * (QUEST_XP_DECAY_LEVELS - decay) as u32 / QUEST_XP_DECAY_LEVELS as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_xp_level_1() {
        assert_eq!(base_kill_xp(1), 50); // 5*1 + 45
    }

    #[test]
    fn base_xp_level_80() {
        assert_eq!(base_kill_xp(80), 445); // 5*80 + 45
    }

    #[test]
    fn grey_level_at_10() {
        assert_eq!(grey_level(10), 4); // 10 - 5 - 1 = 4
    }

    #[test]
    fn grey_level_at_80() {
        assert_eq!(grey_level(80), 71); // 80 - 9 = 71
    }

    #[test]
    fn grey_level_at_5() {
        assert_eq!(grey_level(5), 0);
    }

    #[test]
    fn kill_xp_same_level() {
        let xp = kill_xp(80, 80);
        assert_eq!(xp, base_kill_xp(80)); // no reduction
    }

    #[test]
    fn kill_xp_higher_creature_bonus() {
        let same = kill_xp(78, 78);
        let higher = kill_xp(78, 80);
        assert!(higher > same, "higher level creature should give more XP");
    }

    #[test]
    fn kill_xp_lower_creature_reduced() {
        let same = kill_xp(80, 80);
        let lower = kill_xp(80, 75);
        assert!(lower < same, "lower level creature should give less XP");
        assert!(lower > 0, "non-grey should still give some XP");
    }

    #[test]
    fn kill_xp_grey_is_zero() {
        // Level 80 player, grey threshold = 71
        assert_eq!(kill_xp(80, 70), 0);
        assert_eq!(kill_xp(80, 50), 0);
        assert_eq!(kill_xp(80, 1), 0);
    }

    #[test]
    fn kill_xp_just_above_grey() {
        // Level 80, grey = 71, so 72 should give some XP
        let xp = kill_xp(80, 72);
        assert!(xp > 0);
    }

    #[test]
    fn kill_xp_zero_levels() {
        assert_eq!(kill_xp(0, 10), 0);
        assert_eq!(kill_xp(10, 0), 0);
    }

    #[test]
    fn kill_xp_low_level_no_grey() {
        // Level 3 player: grey = 0, so level 1 creature is NOT grey
        assert!(kill_xp(3, 1) > 0);
    }

    // --- Quest XP tests ---

    #[test]
    fn quest_xp_same_level() {
        assert_eq!(quest_xp(1000, 50, 50), 1000);
    }

    #[test]
    fn quest_xp_below_quest_level() {
        // Player lower than quest — full XP
        assert_eq!(quest_xp(1000, 50, 40), 1000);
    }

    #[test]
    fn quest_xp_within_grace() {
        // 5 levels above quest — still full XP
        assert_eq!(quest_xp(1000, 50, 55), 1000);
    }

    #[test]
    fn quest_xp_starts_decaying() {
        // 6 levels above: 1 into decay zone → 1000 * 9/10 = 900
        assert_eq!(quest_xp(1000, 50, 56), 900);
    }

    #[test]
    fn quest_xp_mid_decay() {
        // 10 levels above: 5 into decay → 1000 * 5/10 = 500
        assert_eq!(quest_xp(1000, 50, 60), 500);
    }

    #[test]
    fn quest_xp_fully_decayed() {
        // 15 levels above: 10 into decay → 0
        assert_eq!(quest_xp(1000, 50, 65), 0);
    }

    #[test]
    fn quest_xp_way_over_level() {
        assert_eq!(quest_xp(1000, 10, 80), 0);
    }

    #[test]
    fn quest_xp_zero_base() {
        assert_eq!(quest_xp(0, 50, 50), 0);
    }

    // --- Rested XP tests ---

    #[test]
    fn rested_new_starts_empty() {
        let rested = RestedXp::new(10000);
        assert_eq!(rested.amount, 0);
        assert_eq!(rested.max, 15000); // 1.5 * 10000
        assert!(!rested.is_rested());
    }

    #[test]
    fn rested_accumulate_in_rest_area() {
        let mut rested = RestedXp::new(10000);
        rested.accumulate(8.0, 10000); // 8 hours = 5% of level
        assert_eq!(rested.amount, 500); // 10000 * 0.05
        assert!(rested.is_rested());
    }

    #[test]
    fn rested_caps_at_max() {
        let mut rested = RestedXp::new(10000);
        rested.accumulate(1000.0, 10000); // way more than needed
        assert_eq!(rested.amount, 15000); // capped at 1.5 levels
    }

    #[test]
    fn rested_apply_bonus_doubles_xp() {
        let mut rested = RestedXp::new(10000);
        rested.amount = 500;
        let (total, consumed) = rested.apply_bonus(200);
        assert_eq!(total, 400); // 200 base + 200 bonus
        assert_eq!(consumed, 200);
        assert_eq!(rested.amount, 300); // 500 - 200
    }

    #[test]
    fn rested_bonus_limited_by_pool() {
        let mut rested = RestedXp::new(10000);
        rested.amount = 100;
        let (total, consumed) = rested.apply_bonus(500);
        assert_eq!(total, 600); // 500 + 100 (only 100 available)
        assert_eq!(consumed, 100);
        assert_eq!(rested.amount, 0);
    }

    #[test]
    fn rested_no_bonus_when_empty() {
        let mut rested = RestedXp::new(10000);
        let (total, consumed) = rested.apply_bonus(500);
        assert_eq!(total, 500);
        assert_eq!(consumed, 0);
    }

    #[test]
    fn rested_levels_fraction() {
        let mut rested = RestedXp::new(10000);
        rested.amount = 5000;
        assert!((rested.rested_levels(10000) - 0.5).abs() < 0.001);
    }

    #[test]
    fn rested_update_max_on_level_up() {
        let mut rested = RestedXp::new(10000);
        rested.amount = 15000; // at cap
        rested.update_max(20000); // level up
        assert_eq!(rested.max, 30000);
        assert_eq!(rested.amount, 15000); // still within new cap
    }

    // --- Group XP tests ---

    #[test]
    fn group_xp_solo_gets_full() {
        let members = vec![GroupMemberXp { level: 80, distance: 5.0 }];
        let shares = group_kill_xp(80, &members);
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].xp, kill_xp(80, 80)); // no bonus, no split
    }

    #[test]
    fn group_xp_two_equal_split() {
        let members = vec![
            GroupMemberXp { level: 80, distance: 5.0 },
            GroupMemberXp { level: 80, distance: 10.0 },
        ];
        let shares = group_kill_xp(80, &members);
        // 2 members, 1.0x bonus, equal levels → each gets half
        let base = kill_xp(80, 80);
        assert_eq!(shares[0].xp, base / 2);
        assert_eq!(shares[1].xp, base / 2);
    }

    #[test]
    fn group_xp_three_member_bonus() {
        let members = vec![
            GroupMemberXp { level: 80, distance: 5.0 },
            GroupMemberXp { level: 80, distance: 5.0 },
            GroupMemberXp { level: 80, distance: 5.0 },
        ];
        let shares = group_kill_xp(80, &members);
        let base = kill_xp(80, 80);
        // 3 members, 1.166x bonus → pool = base * 1.166, each gets pool/3
        let pool = (base as f32 * 1.166) as u32;
        let expected_each = pool / 3;
        assert_eq!(shares[0].xp, expected_each);
    }

    #[test]
    fn group_xp_level_weighted() {
        let members = vec![
            GroupMemberXp { level: 80, distance: 5.0 },
            GroupMemberXp { level: 40, distance: 5.0 },
        ];
        let shares = group_kill_xp(80, &members);
        // Level 80 should get more than level 40
        assert!(shares[0].xp > shares[1].xp);
        // 80/(80+40) = 2/3, 40/(80+40) = 1/3
        assert_eq!(shares[0].xp, shares[1].xp * 2);
    }

    #[test]
    fn group_xp_out_of_range_gets_zero() {
        let members = vec![
            GroupMemberXp { level: 80, distance: 5.0 },
            GroupMemberXp { level: 80, distance: 200.0 }, // out of range
        ];
        let shares = group_kill_xp(80, &members);
        assert!(shares[0].xp > 0);
        assert_eq!(shares[1].xp, 0);
    }

    #[test]
    fn group_xp_all_out_of_range() {
        let members = vec![
            GroupMemberXp { level: 80, distance: 200.0 },
        ];
        let shares = group_kill_xp(80, &members);
        assert_eq!(shares[0].xp, 0);
    }
}
