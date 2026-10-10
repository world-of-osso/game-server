use super::{Catalog, Effect};
use std::collections::BTreeSet;

pub const RANDOM_SCALE: u32 = 1_000_000;
pub const BASE_VARIANCE: i32 = 20; // Blizzard5.3 accuracy article: +/-10%.
pub const POWER_COEFFICIENT: f64 = 0.05; // Retail SharedPetBattleTemplates AttackBonus.
pub const CRITICAL_MULTIPLIER: f64 = 1.5; // Blizzard5.3 accuracy article.

pub fn supported_property(property: u32) -> bool {
    matches!(
        property,
        22 | 23 | 24 | 26 | 31 | 49 | 50 | 52 | 53 | 54 | 63 | 222
    )
}
/// Recursive admission: every turn and referenced aura must be executable before RNG draws.
/// Event1..4 etc intentionally excluded; their causal ordering isn't established here.
pub fn validate_ability(catalog: &Catalog, ability: u32) -> Result<(), String> {
    validate_ability_graph(catalog, ability, &mut BTreeSet::new())
}
fn validate_ability_graph(
    catalog: &Catalog,
    ability: u32,
    seen: &mut BTreeSet<u32>,
) -> Result<(), String> {
    if !seen.insert(ability) {
        return Ok(());
    }
    if !catalog.abilities.contains_key(&ability) {
        return Err(format!("missing ability {ability}"));
    }
    if let Some(states) = catalog.ability_states.get(&ability) {
        for state in states.keys() {
            let name = &catalog.states[state];
            if name == "Mechanic_IsObject" {
                return Err(format!("ability {ability}: unsupported object/team aura"));
            }
            if !matches!(
                *state,
                21 | 22 | 23 | 24 | 25 | 29 | 30 | 33 | 34 | 35 | 36 | 40 | 41 | 42
                    ..=51
                        | 64
                        | 65
                        | 66
                        | 68
                        | 71
                        | 72
                        | 73
                        | 77
                        | 82
                        | 84
                        | 85
                        | 98
                        | 113
                        | 114
                        | 128
                        | 129
                        | 149
                        | 158
                        | 176
                        | 196
                        | 199
            ) {
                return Err(format!(
                    "ability {ability}: unsupported aura state {state} {name}"
                ));
            }
        }
    }
    for turn in catalog.ability_turns(ability) {
        if (turn.kind == 0 && (turn.event != 255 || turn.order == 0))
            || (turn.kind == 1 && !matches!(turn.event, 0 | 5 | 6 | 7 | 8 | 10 | 11))
            || turn.kind > 1
        {
            return Err(format!(
                "ability {ability}: unsupported turn {} kind{} event{}",
                turn.id, turn.kind, turn.event
            ));
        }
        for effect in catalog.turn_effects(turn.id) {
            validate_effect(catalog, effect)?;
            if matches!(effect.property, 24 | 222) && catalog.abilities[&ability].family < 0 {
                return Err(format!("ability {ability}: damage has no family"));
            }
            if effect.aura != 0 {
                validate_ability_graph(catalog, effect.aura, seen)?;
            }
        }
    }
    Ok(())
}
fn validate_effect(catalog: &Catalog, effect: &Effect) -> Result<(), String> {
    let p = effect.params;
    if !supported_property(effect.property) {
        return Err(format!(
            "unsupported property {} at effect {}",
            effect.property, effect.id
        ));
    }
    let supported = match effect.property {
        22 => p[0] == 0 && p[2..].iter().all(|v| *v == 0),
        24 => p[3..].iter().all(|v| *v == 0),
        222 => p[3..].iter().all(|v| *v == 0) && p[2] >= 0,
        31 => catalog.states.contains_key(&(p[0] as u32)),
        49 => effect.aura != 0 && p[1..].iter().all(|v| *v == 0),
        26 => {
            effect.aura != 0
                && p[2] >= -1
                && matches!(p[3], 0 | 1)
                && p[4..].iter().all(|v| *v == 0)
        }
        50 | 54 => effect.aura != 0 && p[2] >= -1 && p[3] >= 0 && p[4..].iter().all(|v| *v == 0),
        52 | 63 => effect.aura != 0 && p[2] >= -1 && p[3..].iter().all(|v| *v == 0),
        23 => p[2] >= 0 && p[3..].iter().all(|v| *v == 0),
        53 => p[2..].iter().all(|v| *v == 0),
        _ => false,
    };
    if !supported {
        return Err(format!(
            "unsupported parameters for property {} effect {}: {p:?}",
            effect.property, effect.id
        ));
    }
    Ok(())
}

/// Zero-based DB2 families. +50%/-33% from local family ability236..245 descriptions.
pub fn type_multiplier(attack: usize, defender: usize) -> Result<f64, String> {
    const STRONG: [usize; 10] = [1, 5, 8, 0, 3, 2, 9, 4, 6, 7];
    const WEAK: [usize; 10] = [7, 3, 1, 8, 0, 9, 4, 2, 5, 6];
    if attack >= 10 || defender >= 10 {
        return Err("invalid damage family".into());
    }
    Ok(if STRONG[attack] == defender {
        1.5
    } else if WEAK[attack] == defender {
        0.67
    } else {
        1.0
    })
}
