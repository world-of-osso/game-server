//! Hand-crafted spell definitions for proof-of-concept abilities.
//!
//! 5 Warrior abilities with WotLK-era spell IDs and SimC-derived coefficients.
//! These serve as the template for how abilities flow through the system:
//! SpellData → process_spell_effects → EffectResult → server applies.
//!
//! When the full import pipeline is ready, these will be loaded from
//! spell_dbc + spell_bonus_data instead.

use crate::components::SpellSchool;
use crate::spell_data::{ResourceType, SpellCost, SpellData, SpellEffectDef, SpellTarget};

/// Mortal Strike (Rank 8) — Arms Warrior's main attack.
/// Spell ID 47486. 30 rage, instant, 6s CD. Deals weapon damage + AP scaling.
pub fn mortal_strike() -> SpellData {
    SpellData {
        id: 47486,
        name: "Mortal Strike".into(),
        school: SpellSchool::Physical,
        cast_time: 0.0,
        cooldown: 6.0,
        cost: Some(SpellCost {
            resource: ResourceType::Rage,
            amount: 30.0,
        }),
        target: SpellTarget::Hostile,
        range: 0.0,
        cast_while_moving: true,
        interruptible: false,
        effects: [
            Some(SpellEffectDef::SchoolDamage {
                base_min: 380.0,
                base_max: 380.0,
                ap_coefficient: 1.68,
                sp_coefficient: 0.0,
            }),
            None,
            None,
        ],
    }
}

/// Execute (Rank 9) — Finisher usable below 20% HP.
/// Spell ID 47471. 15 rage base (consumes up to 30 extra), instant, no CD.
pub fn execute() -> SpellData {
    SpellData {
        id: 47471,
        name: "Execute".into(),
        school: SpellSchool::Physical,
        cast_time: 0.0,
        cooldown: 0.0,
        cost: Some(SpellCost {
            resource: ResourceType::Rage,
            amount: 15.0,
        }),
        target: SpellTarget::Hostile,
        range: 0.0,
        cast_while_moving: true,
        interruptible: false,
        effects: [
            Some(SpellEffectDef::SchoolDamage {
                base_min: 1456.0,
                base_max: 1456.0,
                ap_coefficient: 2.0,
                sp_coefficient: 0.0,
            }),
            None,
            None,
        ],
    }
}

/// Shield Slam (Rank 8) — Prot Warrior's high-threat attack.
/// Spell ID 47488. 20 rage, instant, 6s CD.
pub fn shield_slam() -> SpellData {
    SpellData {
        id: 47488,
        name: "Shield Slam".into(),
        school: SpellSchool::Physical,
        cast_time: 0.0,
        cooldown: 6.0,
        cost: Some(SpellCost {
            resource: ResourceType::Rage,
            amount: 20.0,
        }),
        target: SpellTarget::Hostile,
        range: 0.0,
        cast_while_moving: true,
        interruptible: false,
        effects: [
            Some(SpellEffectDef::SchoolDamage {
                base_min: 990.0,
                base_max: 1040.0,
                ap_coefficient: 0.6,
                sp_coefficient: 0.0,
            }),
            None,
            None,
        ],
    }
}

/// Whirlwind (Rank 2) — AoE attack hitting all nearby enemies.
/// Spell ID 1680. 25 rage, instant, 10s CD.
pub fn whirlwind() -> SpellData {
    SpellData {
        id: 1680,
        name: "Whirlwind".into(),
        school: SpellSchool::Physical,
        cast_time: 0.0,
        cooldown: 10.0,
        cost: Some(SpellCost {
            resource: ResourceType::Rage,
            amount: 25.0,
        }),
        target: SpellTarget::Hostile,
        range: 0.0,
        cast_while_moving: true,
        interruptible: false,
        effects: [
            Some(SpellEffectDef::SchoolDamage {
                base_min: 0.0,
                base_max: 0.0,
                ap_coefficient: 0.5,
                sp_coefficient: 0.0,
            }),
            None,
            None,
        ],
    }
}

/// Victory Rush — Free attack after killing an enemy. No cost, no CD.
/// Spell ID 34428. Deals AP-scaled damage and heals 20% of max HP.
pub fn victory_rush() -> SpellData {
    SpellData {
        id: 34428,
        name: "Victory Rush".into(),
        school: SpellSchool::Physical,
        cast_time: 0.0,
        cooldown: 0.0,
        cost: None,
        target: SpellTarget::Hostile,
        range: 0.0,
        cast_while_moving: true,
        interruptible: false,
        effects: [
            Some(SpellEffectDef::SchoolDamage {
                base_min: 0.0,
                base_max: 0.0,
                ap_coefficient: 0.56,
                sp_coefficient: 0.0,
            }),
            Some(SpellEffectDef::Heal {
                base_min: 0.0,
                base_max: 0.0,
                ap_coefficient: 0.0,
                sp_coefficient: 0.0,
                // Note: the 20% max HP heal is a special mechanic, not SP/AP scaled.
                // The server handles this as a percent-of-max heal; the base/coeff
                // fields are zero because the scaling is special-cased.
            }),
            None,
        ],
    }
}

/// All proof-of-concept Warrior abilities.
pub fn warrior_abilities() -> Vec<SpellData> {
    vec![
        mortal_strike(),
        execute(),
        shield_slam(),
        whirlwind(),
        victory_rush(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell_data::{CasterStats, EffectResult, process_spell_effects};

    fn warrior_stats() -> CasterStats {
        CasterStats {
            attack_power: 4000.0,
            spell_power: 0.0,
        }
    }

    #[test]
    fn mortal_strike_damage() {
        let ms = mortal_strike();
        assert_eq!(ms.id, 47486);
        assert_eq!(ms.cooldown, 6.0);
        let cost = ms.cost.unwrap();
        assert_eq!(cost.resource, ResourceType::Rage);
        assert_eq!(cost.amount, 30.0);

        let results = process_spell_effects(&ms, &warrior_stats(), 0.5);
        assert_eq!(results.len(), 1);
        // 380 + 4000 * 1.68 = 380 + 6720 = 7100
        assert_eq!(results[0], EffectResult::Damage { amount: 7100.0 });
    }

    #[test]
    fn execute_damage() {
        let ex = execute();
        let results = process_spell_effects(&ex, &warrior_stats(), 0.5);
        // 1456 + 4000 * 2.0 = 1456 + 8000 = 9456
        assert_eq!(results[0], EffectResult::Damage { amount: 9456.0 });
    }

    #[test]
    fn shield_slam_damage_range() {
        let ss = shield_slam();
        let min = process_spell_effects(&ss, &warrior_stats(), 0.0);
        let max = process_spell_effects(&ss, &warrior_stats(), 1.0);
        // min: 990 + 4000 * 0.6 = 3390
        // max: 1040 + 4000 * 0.6 = 3440
        assert_eq!(min[0], EffectResult::Damage { amount: 3390.0 });
        assert_eq!(max[0], EffectResult::Damage { amount: 3440.0 });
    }

    #[test]
    fn whirlwind_pure_ap_scaling() {
        let ww = whirlwind();
        let results = process_spell_effects(&ww, &warrior_stats(), 0.5);
        // 0 + 4000 * 0.5 = 2000
        assert_eq!(results[0], EffectResult::Damage { amount: 2000.0 });
    }

    #[test]
    fn victory_rush_has_damage_and_heal() {
        let vr = victory_rush();
        assert!(vr.cost.is_none(), "Victory Rush is free");
        let results = process_spell_effects(&vr, &warrior_stats(), 0.5);
        assert_eq!(results.len(), 2);
        // damage: 4000 * 0.56 = 2240
        assert_eq!(results[0], EffectResult::Damage { amount: 2240.0 });
        // heal: 0 (special-cased percent heal, not AP/SP scaled)
        assert_eq!(results[1], EffectResult::Heal { amount: 0.0 });
    }

    #[test]
    fn warrior_catalog_has_5_abilities() {
        let abilities = warrior_abilities();
        assert_eq!(abilities.len(), 5);
        let ids: Vec<u32> = abilities.iter().map(|s| s.id).collect();
        assert!(ids.contains(&47486)); // Mortal Strike
        assert!(ids.contains(&47471)); // Execute
        assert!(ids.contains(&47488)); // Shield Slam
        assert!(ids.contains(&1680)); // Whirlwind
        assert!(ids.contains(&34428)); // Victory Rush
    }

    #[test]
    fn all_warrior_abilities_are_physical_instant_melee() {
        for spell in warrior_abilities() {
            assert_eq!(spell.school, SpellSchool::Physical);
            assert_eq!(spell.cast_time, 0.0);
            assert_eq!(spell.range, 0.0);
            assert!(spell.cast_while_moving);
        }
    }
}
