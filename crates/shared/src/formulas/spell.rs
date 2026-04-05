/// Base spell miss chance in hundredths of percent (3.00% = 300).
const BASE_SPELL_MISS: i32 = 300;
/// Additional miss per level the target is above the caster (1.00% = 100).
const MISS_PER_LEVEL_DIFF: i32 = 100;

/// Spell miss chance based on caster and target levels.
///
/// Returns a value in 0..=10000 (hundredths of percent). Same base rate as
/// melee (3% at equal level, +1% per level difference) but evaluated
/// separately — spell hit rating reduces this independently of melee hit.
///
/// Clamped to 0 minimum.
pub fn spell_miss_chance(caster_level: u8, target_level: u8) -> u32 {
    let level_diff = target_level as i32 - caster_level as i32;
    let chance = BASE_SPELL_MISS + level_diff * MISS_PER_LEVEL_DIFF;
    chance.max(0) as u32
}

/// Spell hit outcome (binary: hit or miss).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellOutcome {
    Miss,
    Hit,
}

/// Resolve a spell hit check. `roll` must be in 0..10000.
pub fn resolve_spell_hit(miss_chance: u32, roll: u32) -> SpellOutcome {
    if roll < miss_chance {
        SpellOutcome::Miss
    } else {
        SpellOutcome::Hit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_miss_equal_level() {
        assert_eq!(spell_miss_chance(80, 80), 300); // 3%
    }

    #[test]
    fn spell_miss_target_higher() {
        assert_eq!(spell_miss_chance(80, 81), 400);
        assert_eq!(spell_miss_chance(80, 83), 600);
    }

    #[test]
    fn spell_miss_caster_higher() {
        assert_eq!(spell_miss_chance(80, 79), 200);
        assert_eq!(spell_miss_chance(80, 78), 100);
    }

    #[test]
    fn spell_miss_clamps_to_zero() {
        assert_eq!(spell_miss_chance(80, 70), 0);
    }

    #[test]
    fn spell_hit_resolves_miss() {
        assert_eq!(resolve_spell_hit(300, 0), SpellOutcome::Miss);
        assert_eq!(resolve_spell_hit(300, 299), SpellOutcome::Miss);
    }

    #[test]
    fn spell_hit_resolves_hit() {
        assert_eq!(resolve_spell_hit(300, 300), SpellOutcome::Hit);
        assert_eq!(resolve_spell_hit(300, 9999), SpellOutcome::Hit);
    }

    #[test]
    fn spell_hit_zero_miss_always_hits() {
        assert_eq!(resolve_spell_hit(0, 0), SpellOutcome::Hit);
    }
}
