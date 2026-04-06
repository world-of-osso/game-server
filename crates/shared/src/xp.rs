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
}
