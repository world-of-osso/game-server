/// Melee hit table outcome from a single roll.
///
/// Order matches WoW's single-roll table: miss → dodge → parry → glancing →
/// block → crit → hit. Ref: AzerothCore `RollMeleeOutcomeAgainst()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeleeOutcome {
    Miss,
    Dodge,
    Parry,
    Glancing,
    Block,
    Crit,
    Hit,
}

/// Input chances for the melee hit table, each as a value in 0..=10000
/// (hundredths of a percent, matching AzerothCore's internal scale).
///
/// Example: 300 = 3.00% miss chance.
#[derive(Debug, Clone, Copy, Default)]
pub struct MeleeHitChances {
    pub miss: u32,
    pub dodge: u32,
    pub parry: u32,
    pub glancing: u32,
    pub block: u32,
    pub crit: u32,
}

/// Resolve a melee attack outcome using the single-roll hit table.
///
/// `roll` must be in 0..10000.  The table is evaluated in priority order:
/// miss → dodge → parry → glancing → block → crit → hit.
///
/// Each chance occupies a contiguous range on the 0–10000 number line.
/// The first range the roll falls into determines the outcome.
pub fn resolve_melee_outcome(chances: &MeleeHitChances, roll: u32) -> MeleeOutcome {
    let mut threshold = 0u32;

    threshold += chances.miss;
    if roll < threshold {
        return MeleeOutcome::Miss;
    }

    threshold += chances.dodge;
    if roll < threshold {
        return MeleeOutcome::Dodge;
    }

    threshold += chances.parry;
    if roll < threshold {
        return MeleeOutcome::Parry;
    }

    threshold += chances.glancing;
    if roll < threshold {
        return MeleeOutcome::Glancing;
    }

    threshold += chances.block;
    if roll < threshold {
        return MeleeOutcome::Block;
    }

    threshold += chances.crit;
    if roll < threshold {
        return MeleeOutcome::Crit;
    }

    MeleeOutcome::Hit
}

/// Base melee miss chance in hundredths of percent (3.00% = 300).
const BASE_MISS_CHANCE: i32 = 300;
/// Additional miss per level the target is above the attacker (1.00% = 100).
const MISS_PER_LEVEL_DIFF: i32 = 100;

/// Calculate melee miss chance based on attacker and target levels.
///
/// Returns a value in 0..=10000 (hundredths of percent) for use in
/// `MeleeHitChances::miss`. Clamped to 0 minimum (can't go negative).
///
/// - Equal level: 3%
/// - Target +1: 4%, +2: 5%, +3: 6%
/// - Attacker higher: 2%, 1%, 0%
pub fn miss_chance(attacker_level: u8, target_level: u8) -> u32 {
    let level_diff = target_level as i32 - attacker_level as i32;
    let chance = BASE_MISS_CHANCE + level_diff * MISS_PER_LEVEL_DIFF;
    chance.max(0) as u32
}

// --- Dodge / Parry from ratings with avoidance DR ---

/// Base dodge/parry chance before any rating (percent).
const BASE_AVOIDANCE_PCT: f32 = 5.0;

/// Per-class diminishing returns k-value.
/// Index = class_id - 1. Ref: AzerothCore `m_diminishing_k`.
/// Classes: 1=Warrior, 2=Paladin, 3=Hunter, 4=Rogue, 5=Priest,
///          6=DK, 7=Shaman, 8=Mage, 9=Warlock, 10=unused, 11=Druid
const AVOIDANCE_K: [f32; 11] = [
    0.9560, // Warrior
    0.9560, // Paladin
    0.9880, // Hunter
    0.9880, // Rogue
    0.9830, // Priest
    0.9560, // DK
    0.9880, // Shaman
    0.9830, // Mage
    0.9830, // Warlock
    0.0,    // unused
    0.9720, // Druid
];

/// Per-class dodge cap. Index = class_id - 1.
const DODGE_CAP: [f32; 11] = [
    88.129_02,  // Warrior
    88.129_02,  // Paladin
    145.560_41, // Hunter
    145.560_41, // Rogue
    150.375_95, // Priest
    88.129_02,  // DK
    145.560_41, // Shaman
    150.375_95, // Mage
    150.375_95, // Warlock
    0.0,        // unused
    116.890_71, // Druid
];

/// Per-class parry cap. Index = class_id - 1. Zero = class cannot parry.
const PARRY_CAP: [f32; 11] = [
    47.003525,  // Warrior
    47.003525,  // Paladin
    145.560_41, // Hunter
    145.560_41, // Rogue
    0.0,        // Priest (can't parry)
    47.003525,  // DK
    145.560_41, // Shaman
    0.0,        // Mage (can't parry)
    0.0,        // Warlock (can't parry)
    0.0,        // unused
    0.0,        // Druid (can't parry)
];

/// Avoidance diminishing returns formula.
///
/// `effective = base + diminishing * cap / (diminishing + cap * k)`
///
/// Returns 0 if cap is 0 (class cannot dodge/parry).
fn avoidance_dr(base: f32, diminishing_pct: f32, cap: f32, k: f32) -> f32 {
    if cap <= 0.0 || diminishing_pct <= 0.0 {
        return base.max(0.0);
    }
    let dr_portion = diminishing_pct * cap / (diminishing_pct + cap * k);
    (base + dr_portion).max(0.0)
}

/// Calculate dodge chance from target's dodge rating.
///
/// Returns value in 0..=10000 (hundredths of percent) for `MeleeHitChances`.
pub fn dodge_chance(class: u8, level: u8, dodge_rating: f32) -> u32 {
    let idx = class.wrapping_sub(1) as usize;
    if idx >= DODGE_CAP.len() || DODGE_CAP[idx] <= 0.0 {
        return 0;
    }
    let rating_pct = super::rating_to_percent(dodge_rating, level, super::RatingType::Dodge)
        .unwrap_or(0.0);
    let pct = avoidance_dr(BASE_AVOIDANCE_PCT, rating_pct, DODGE_CAP[idx], AVOIDANCE_K[idx]);
    (pct * 100.0) as u32
}

/// Calculate parry chance from target's parry rating.
///
/// Returns value in 0..=10000 (hundredths of percent) for `MeleeHitChances`.
/// Returns 0 for classes that cannot parry (Priest, Mage, Warlock, Druid).
pub fn parry_chance(class: u8, level: u8, parry_rating: f32) -> u32 {
    let idx = class.wrapping_sub(1) as usize;
    if idx >= PARRY_CAP.len() || PARRY_CAP[idx] <= 0.0 {
        return 0;
    }
    let rating_pct = super::rating_to_percent(parry_rating, level, super::RatingType::Parry)
        .unwrap_or(0.0);
    let pct = avoidance_dr(BASE_AVOIDANCE_PCT, rating_pct, PARRY_CAP[idx], AVOIDANCE_K[idx]);
    (pct * 100.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_chances() -> MeleeHitChances {
        MeleeHitChances {
            miss: 300,    // 3%
            dodge: 500,   // 5%
            parry: 500,   // 5%
            glancing: 0,
            block: 0,
            crit: 1000,   // 10%
        }
    }

    #[test]
    fn roll_miss() {
        let c = default_chances();
        assert_eq!(resolve_melee_outcome(&c, 0), MeleeOutcome::Miss);
        assert_eq!(resolve_melee_outcome(&c, 299), MeleeOutcome::Miss);
    }

    #[test]
    fn roll_dodge() {
        let c = default_chances();
        // dodge range: 300..800
        assert_eq!(resolve_melee_outcome(&c, 300), MeleeOutcome::Dodge);
        assert_eq!(resolve_melee_outcome(&c, 799), MeleeOutcome::Dodge);
    }

    #[test]
    fn roll_parry() {
        let c = default_chances();
        // parry range: 800..1300
        assert_eq!(resolve_melee_outcome(&c, 800), MeleeOutcome::Parry);
        assert_eq!(resolve_melee_outcome(&c, 1299), MeleeOutcome::Parry);
    }

    #[test]
    fn roll_crit() {
        let c = default_chances();
        // crit range: 1300..2300 (glancing=0, block=0)
        assert_eq!(resolve_melee_outcome(&c, 1300), MeleeOutcome::Crit);
        assert_eq!(resolve_melee_outcome(&c, 2299), MeleeOutcome::Crit);
    }

    #[test]
    fn roll_hit() {
        let c = default_chances();
        // everything above 2300
        assert_eq!(resolve_melee_outcome(&c, 2300), MeleeOutcome::Hit);
        assert_eq!(resolve_melee_outcome(&c, 9999), MeleeOutcome::Hit);
    }

    #[test]
    fn roll_glancing() {
        let c = MeleeHitChances {
            miss: 300,
            dodge: 0,
            parry: 0,
            glancing: 2000, // 20%
            block: 0,
            crit: 1000,
        };
        // glancing range: 300..2300
        assert_eq!(resolve_melee_outcome(&c, 300), MeleeOutcome::Glancing);
        assert_eq!(resolve_melee_outcome(&c, 2299), MeleeOutcome::Glancing);
    }

    #[test]
    fn roll_block() {
        let c = MeleeHitChances {
            miss: 0,
            dodge: 0,
            parry: 0,
            glancing: 0,
            block: 1500, // 15%
            crit: 1000,
        };
        assert_eq!(resolve_melee_outcome(&c, 0), MeleeOutcome::Block);
        assert_eq!(resolve_melee_outcome(&c, 1499), MeleeOutcome::Block);
        assert_eq!(resolve_melee_outcome(&c, 1500), MeleeOutcome::Crit);
    }

    #[test]
    fn all_zero_chances_always_hit() {
        let c = MeleeHitChances::default();
        assert_eq!(resolve_melee_outcome(&c, 0), MeleeOutcome::Hit);
        assert_eq!(resolve_melee_outcome(&c, 5000), MeleeOutcome::Hit);
    }

    #[test]
    fn full_table_boundaries() {
        let c = MeleeHitChances {
            miss: 300,
            dodge: 500,
            parry: 400,
            glancing: 1000,
            block: 500,
            crit: 1500,
        };
        // miss: 0..300
        assert_eq!(resolve_melee_outcome(&c, 299), MeleeOutcome::Miss);
        // dodge: 300..800
        assert_eq!(resolve_melee_outcome(&c, 300), MeleeOutcome::Dodge);
        // parry: 800..1200
        assert_eq!(resolve_melee_outcome(&c, 800), MeleeOutcome::Parry);
        // glancing: 1200..2200
        assert_eq!(resolve_melee_outcome(&c, 1200), MeleeOutcome::Glancing);
        // block: 2200..2700
        assert_eq!(resolve_melee_outcome(&c, 2200), MeleeOutcome::Block);
        // crit: 2700..4200
        assert_eq!(resolve_melee_outcome(&c, 2700), MeleeOutcome::Crit);
        // hit: 4200+
        assert_eq!(resolve_melee_outcome(&c, 4200), MeleeOutcome::Hit);
    }

    #[test]
    fn crit_pushes_off_table() {
        // If miss+dodge+parry+glancing+block+crit >= 10000, no room for hit
        let c = MeleeHitChances {
            miss: 2000,
            dodge: 2000,
            parry: 2000,
            glancing: 2000,
            block: 1000,
            crit: 1000,
        };
        // Total = 10000, so roll 9999 → crit (last slot)
        assert_eq!(resolve_melee_outcome(&c, 9999), MeleeOutcome::Crit);
    }

    // --- Miss chance tests ---

    #[test]
    fn miss_equal_level() {
        assert_eq!(miss_chance(80, 80), 300); // 3%
    }

    #[test]
    fn miss_target_higher() {
        assert_eq!(miss_chance(80, 81), 400); // 4%
        assert_eq!(miss_chance(80, 83), 600); // 6%
    }

    #[test]
    fn miss_attacker_higher() {
        assert_eq!(miss_chance(80, 79), 200); // 2%
        assert_eq!(miss_chance(80, 78), 100); // 1%
    }

    #[test]
    fn miss_clamps_to_zero() {
        // attacker 10 levels above → would be -7%, clamped to 0
        assert_eq!(miss_chance(80, 70), 0);
    }

    #[test]
    fn miss_low_levels() {
        assert_eq!(miss_chance(1, 1), 300);
        assert_eq!(miss_chance(1, 3), 500);
    }

    // --- Dodge/Parry tests ---

    #[test]
    fn dodge_base_with_zero_rating() {
        // Warrior (class 1) with 0 dodge rating: just base 5%
        let d = dodge_chance(1, 80, 0.0);
        assert_eq!(d, 500); // 5.00%
    }

    #[test]
    fn dodge_increases_with_rating() {
        let low = dodge_chance(1, 80, 500.0);
        let high = dodge_chance(1, 80, 2000.0);
        assert!(high > low, "more rating = more dodge");
        assert!(low > 500, "should exceed base 5%");
    }

    #[test]
    fn dodge_diminishes_at_high_rating() {
        // Marginal gain should decrease as rating increases
        let at_1000 = dodge_chance(1, 80, 1000.0);
        let at_2000 = dodge_chance(1, 80, 2000.0);
        let at_3000 = dodge_chance(1, 80, 3000.0);
        let gain_first = at_2000 - at_1000;
        let gain_second = at_3000 - at_2000;
        assert!(
            gain_second < gain_first,
            "diminishing returns: second 1000 rating ({gain_second}) should give less than first ({gain_first})"
        );
    }

    #[test]
    fn parry_base_for_warrior() {
        let p = parry_chance(1, 80, 0.0);
        assert_eq!(p, 500); // 5.00% base
    }

    #[test]
    fn parry_zero_for_non_parry_class() {
        // Priest (5), Mage (8), Warlock (9), Druid (11) can't parry
        assert_eq!(parry_chance(5, 80, 1000.0), 0);
        assert_eq!(parry_chance(8, 80, 1000.0), 0);
        assert_eq!(parry_chance(9, 80, 1000.0), 0);
        assert_eq!(parry_chance(11, 80, 1000.0), 0);
    }

    #[test]
    fn parry_works_for_parry_classes() {
        // Warrior(1), Paladin(2), DK(6) can parry
        assert!(parry_chance(1, 80, 500.0) > 500);
        assert!(parry_chance(2, 80, 500.0) > 500);
        assert!(parry_chance(6, 80, 500.0) > 500);
    }

    #[test]
    fn dodge_invalid_class_returns_zero() {
        assert_eq!(dodge_chance(0, 80, 500.0), 0);
        assert_eq!(dodge_chance(12, 80, 500.0), 0);
    }

    #[test]
    fn avoidance_dr_formula_no_rating() {
        // Pure base, no diminishing portion
        assert_eq!(avoidance_dr(5.0, 0.0, 88.0, 0.956), 5.0);
    }

    #[test]
    fn avoidance_dr_formula_with_rating() {
        // 10% from rating, cap 88, k 0.956
        // DR = 10 * 88 / (10 + 88 * 0.956) = 880 / 94.128 ≈ 9.349
        // Total = 5.0 + 9.349 ≈ 14.349
        let result = avoidance_dr(5.0, 10.0, 88.0, 0.956);
        assert!((result - 14.349).abs() < 0.1);
    }
}
