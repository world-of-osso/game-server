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
}
