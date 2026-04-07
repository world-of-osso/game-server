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

// --- Spell crit ---

/// Base spell crit damage multiplier (150% = 1.5×, unlike melee's 2.0×).
const BASE_SPELL_CRIT_MULTIPLIER: f32 = 1.5;

/// Spell critical strike damage multiplier.
///
/// Base is 1.5× (50% bonus damage). `aura_modifier` is an additive bonus
/// from talents/buffs (e.g. 0.1 for +10% crit damage → 1.6×).
pub fn spell_crit_multiplier(aura_modifier: f32) -> f32 {
    BASE_SPELL_CRIT_MULTIPLIER + aura_modifier
}

/// Apply spell crit multiplier to a damage value.
pub fn apply_spell_crit(damage: f32, aura_modifier: f32) -> f32 {
    damage * spell_crit_multiplier(aura_modifier)
}

// --- Healing ---

/// Result of applying a heal to a target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HealResult {
    /// Effective healing applied (capped at missing health).
    pub effective: f32,
    /// Wasted healing beyond max health.
    pub overheal: f32,
}

/// Apply healing to a health pool. Returns effective healing and overheal.
///
/// `heal_amount` is the raw heal (from `spell_damage` formula or similar).
/// Health is clamped to `max`.
pub fn apply_heal(current: f32, max: f32, heal_amount: f32) -> HealResult {
    let missing = max - current;
    let effective = heal_amount.min(missing).max(0.0);
    HealResult {
        effective,
        overheal: (heal_amount - effective).max(0.0),
    }
}

// --- Cast time / GCD ---

/// Base global cooldown in seconds (most classes).
const BASE_GCD: f32 = 1.5;
/// Base GCD for Rogues, Monks, and Demon Hunters (energy/melee classes).
const SHORT_GCD: f32 = 1.0;
/// Minimum GCD floor in seconds (haste cannot reduce below this).
const MIN_GCD: f32 = 0.75;

/// Apply haste to a cast time: `base_cast / (1 + haste_pct)`.
///
/// `haste_pct` is a fraction (e.g. 0.20 for 20% haste).
pub fn hasted_cast_time(base_cast: f32, haste_pct: f32) -> f32 {
    base_cast / (1.0 + haste_pct)
}

/// Calculate auto-attack swing timer after haste.
///
/// `base_speed` is the weapon speed in seconds (e.g. 3.3 for a 2H sword).
/// `haste_pct` is a fraction (e.g. 0.30 for 30% haste).
pub fn hasted_swing_timer(base_speed: f32, haste_pct: f32) -> f32 {
    hasted_cast_time(base_speed, haste_pct)
}

/// Calculate the global cooldown after haste, floored at 0.75s.
/// Uses the standard 1.5s base GCD.
///
/// `haste_pct` is a fraction (e.g. 0.30 for 30% haste).
pub fn hasted_gcd(haste_pct: f32) -> f32 {
    hasted_cast_time(BASE_GCD, haste_pct).max(MIN_GCD)
}

/// Base GCD for a given class ID.
///
/// Rogues (4), Monks (10), and Demon Hunters (12) use 1.0s base.
/// All other classes use 1.5s base.
pub fn base_gcd_for_class(class_id: u8) -> f32 {
    match class_id {
        4 | 10 | 12 => SHORT_GCD,
        _ => BASE_GCD,
    }
}

/// Calculate the GCD for a specific class after haste, floored at 0.75s.
///
/// Rogues/Monks/DH start at 1.0s; all others at 1.5s.
/// Haste reduces proportionally, minimum 0.75s.
pub fn class_hasted_gcd(class_id: u8, haste_pct: f32) -> f32 {
    let base = base_gcd_for_class(class_id);
    hasted_cast_time(base, haste_pct).max(MIN_GCD)
}

// --- DoT haste scaling ---

/// Result of applying haste to a periodic (DoT/HoT) effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HastePeriodicResult {
    /// Tick interval after haste (seconds).
    pub tick_interval: f32,
    /// Total number of ticks (may be more than base due to haste).
    pub num_ticks: u32,
    /// Total duration after haste (seconds).
    pub duration: f32,
}

/// Apply haste to a periodic effect (DoT/HoT).
///
/// In Pandaria+ WoW, haste reduces the tick interval and the total duration
/// shortens proportionally — but if enough haste accumulates to fit an extra
/// tick, it's added (keeping the original duration approximately the same).
///
/// `base_tick_interval`: time between ticks without haste (e.g. 3.0s).
/// `base_num_ticks`: number of ticks without haste (e.g. 4 ticks).
/// `haste_pct`: haste as a fraction (e.g. 0.25 for 25%).
pub fn hasted_periodic(
    base_tick_interval: f32,
    base_num_ticks: u32,
    haste_pct: f32,
) -> HastePeriodicResult {
    let hasted_tick = hasted_cast_time(base_tick_interval, haste_pct);
    let base_duration = base_tick_interval * base_num_ticks as f32;
    // Extra ticks fit when hasted duration would be shorter than base
    let num_ticks = (base_duration / hasted_tick).round() as u32;
    let num_ticks = num_ticks.max(1);
    HastePeriodicResult {
        tick_interval: hasted_tick,
        num_ticks,
        duration: hasted_tick * num_ticks as f32,
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

    // --- Spell crit tests ---

    #[test]
    fn spell_crit_base_multiplier() {
        assert_eq!(spell_crit_multiplier(0.0), 1.5);
    }

    #[test]
    fn spell_crit_with_aura() {
        assert!((spell_crit_multiplier(0.2) - 1.7).abs() < 0.001);
    }

    #[test]
    fn apply_spell_crit_base() {
        // 1000 damage * 1.5 = 1500
        assert_eq!(apply_spell_crit(1000.0, 0.0), 1500.0);
    }

    #[test]
    fn apply_spell_crit_with_modifier() {
        // 1000 damage * 1.7 (0.2 aura) = 1700
        assert!((apply_spell_crit(1000.0, 0.2) - 1700.0).abs() < 0.01);
    }

    #[test]
    fn spell_crit_lower_than_melee() {
        let spell = spell_crit_multiplier(0.0);
        let melee = super::super::melee::crit_damage_multiplier(0.0);
        assert!(spell < melee, "spell crit (1.5x) < melee crit (2.0x)");
    }

    // --- Healing tests ---

    #[test]
    fn heal_partial_missing() {
        // 7000/10000 HP, heal 2000 → effective 2000, overheal 0
        let r = apply_heal(7000.0, 10000.0, 2000.0);
        assert_eq!(r.effective, 2000.0);
        assert_eq!(r.overheal, 0.0);
    }

    #[test]
    fn heal_with_overheal() {
        // 9000/10000 HP, heal 2000 → effective 1000, overheal 1000
        let r = apply_heal(9000.0, 10000.0, 2000.0);
        assert_eq!(r.effective, 1000.0);
        assert_eq!(r.overheal, 1000.0);
    }

    #[test]
    fn heal_at_full_health() {
        let r = apply_heal(10000.0, 10000.0, 5000.0);
        assert_eq!(r.effective, 0.0);
        assert_eq!(r.overheal, 5000.0);
    }

    #[test]
    fn heal_zero_amount() {
        let r = apply_heal(5000.0, 10000.0, 0.0);
        assert_eq!(r.effective, 0.0);
        assert_eq!(r.overheal, 0.0);
    }

    #[test]
    fn heal_exact_to_full() {
        let r = apply_heal(8000.0, 10000.0, 2000.0);
        assert_eq!(r.effective, 2000.0);
        assert_eq!(r.overheal, 0.0);
    }

    // --- Cast time / GCD tests ---

    #[test]
    fn cast_time_no_haste() {
        assert_eq!(hasted_cast_time(2.0, 0.0), 2.0);
    }

    #[test]
    fn cast_time_with_haste() {
        // 2.0s / (1 + 0.25) = 1.6s
        assert!((hasted_cast_time(2.0, 0.25) - 1.6).abs() < 0.001);
    }

    #[test]
    fn cast_time_high_haste() {
        // 2.0s / (1 + 1.0) = 1.0s
        assert!((hasted_cast_time(2.0, 1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn gcd_no_haste() {
        assert_eq!(hasted_gcd(0.0), 1.5);
    }

    #[test]
    fn gcd_with_haste() {
        // 1.5 / 1.3 ≈ 1.1538
        assert!((hasted_gcd(0.3) - 1.1538).abs() < 0.001);
    }

    #[test]
    fn gcd_floors_at_075() {
        // 1.5 / (1 + 2.0) = 0.5, floored to 0.75
        assert_eq!(hasted_gcd(2.0), 0.75);
    }

    #[test]
    fn gcd_exactly_at_floor() {
        // 1.5 / (1 + 1.0) = 0.75, exactly at floor
        assert_eq!(hasted_gcd(1.0), 0.75);
    }

    // --- Class GCD tests ---

    #[test]
    fn base_gcd_standard_classes() {
        // Warrior, Paladin, Mage, etc. = 1.5s
        assert_eq!(base_gcd_for_class(1), 1.5);
        assert_eq!(base_gcd_for_class(2), 1.5);
        assert_eq!(base_gcd_for_class(8), 1.5);
    }

    #[test]
    fn base_gcd_short_classes() {
        // Rogue=4, Monk=10, DemonHunter=12 → 1.0s
        assert_eq!(base_gcd_for_class(4), 1.0);
        assert_eq!(base_gcd_for_class(10), 1.0);
        assert_eq!(base_gcd_for_class(12), 1.0);
    }

    #[test]
    fn class_hasted_gcd_rogue_no_haste() {
        // Rogue base 1.0s, no haste → 1.0s
        assert_eq!(class_hasted_gcd(4, 0.0), 1.0);
    }

    #[test]
    fn class_hasted_gcd_rogue_with_haste() {
        // Rogue 1.0 / (1 + 0.25) = 0.8s
        let gcd = class_hasted_gcd(4, 0.25);
        assert!((gcd - 0.8).abs() < 0.001);
    }

    #[test]
    fn class_hasted_gcd_rogue_floors_at_075() {
        // Rogue 1.0 / (1 + 1.0) = 0.5 → floored to 0.75
        assert_eq!(class_hasted_gcd(4, 1.0), 0.75);
    }

    #[test]
    fn class_hasted_gcd_mage_standard() {
        // Mage 1.5 / (1 + 0.20) = 1.25s
        let gcd = class_hasted_gcd(8, 0.20);
        assert!((gcd - 1.25).abs() < 0.001);
    }

    // --- Auto-attack swing timer haste tests ---

    #[test]
    fn swing_timer_no_haste() {
        // 3.3s weapon, 0% haste → 3.3s
        assert!((hasted_swing_timer(3.3, 0.0) - 3.3).abs() < 0.001);
    }

    #[test]
    fn swing_timer_30_percent_haste() {
        // 3.3s weapon, 30% haste → 3.3 / 1.3 ≈ 2.538s
        let timer = hasted_swing_timer(3.3, 0.30);
        assert!((timer - 2.538).abs() < 0.01, "got {timer}");
    }

    #[test]
    fn swing_timer_dagger_with_haste() {
        // 1.7s dagger, 25% haste → 1.7 / 1.25 = 1.36s
        let timer = hasted_swing_timer(1.7, 0.25);
        assert!((timer - 1.36).abs() < 0.01, "got {timer}");
    }

    #[test]
    fn swing_timer_high_haste() {
        // 2.4s 1H sword, 100% haste → 2.4 / 2.0 = 1.2s
        assert!((hasted_swing_timer(2.4, 1.0) - 1.2).abs() < 0.001);
    }

    // --- DoT haste scaling tests ---

    #[test]
    fn periodic_no_haste() {
        // 3s tick, 4 ticks, 0% haste → unchanged
        let r = hasted_periodic(3.0, 4, 0.0);
        assert_eq!(r.tick_interval, 3.0);
        assert_eq!(r.num_ticks, 4);
        assert_eq!(r.duration, 12.0);
    }

    #[test]
    fn periodic_haste_reduces_interval() {
        // 3s tick, 4 ticks, 50% haste → 2s tick
        let r = hasted_periodic(3.0, 4, 0.5);
        assert!((r.tick_interval - 2.0).abs() < 0.001);
    }

    #[test]
    fn periodic_haste_adds_extra_ticks() {
        // 3s tick, 4 ticks (12s), 50% haste → 2s tick, 12/2=6 ticks
        let r = hasted_periodic(3.0, 4, 0.5);
        assert_eq!(r.num_ticks, 6);
    }

    #[test]
    fn periodic_moderate_haste_no_extra_tick() {
        // 3s tick, 4 ticks (12s), 10% haste → 2.727s tick, 12/2.727=4.4 → rounds to 4
        let r = hasted_periodic(3.0, 4, 0.1);
        assert_eq!(r.num_ticks, 4);
    }

    #[test]
    fn periodic_haste_breakpoint_adds_tick() {
        // 3s tick, 5 ticks (15s), 25% haste → 2.4s tick, 15/2.4=6.25 → rounds to 6
        let r = hasted_periodic(3.0, 5, 0.25);
        assert_eq!(r.num_ticks, 6);
        assert!((r.tick_interval - 2.4).abs() < 0.001);
    }

    #[test]
    fn periodic_minimum_one_tick() {
        let r = hasted_periodic(3.0, 1, 0.0);
        assert_eq!(r.num_ticks, 1);
    }
}
