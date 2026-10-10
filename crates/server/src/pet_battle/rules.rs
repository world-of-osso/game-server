use super::{Catalog, Effect};
use std::collections::BTreeSet;

pub const RANDOM_SCALE: u32 = 1_000_000;
pub const BASE_VARIANCE: i32 = 20; // Blizzard5.3 accuracy article: +/-10%.
pub const POWER_COEFFICIENT: f64 = 0.05; // Retail SharedPetBattleTemplates AttackBonus.
pub const CRITICAL_MULTIPLIER: f64 = 1.5; // Blizzard5.3 accuracy article.

pub fn supported_property(property: u32) -> bool {
    matches!(
        property,
        22 | 23
            | 24
            | 26
            | 27
            | 28
            | 29
            | 31
            | 32
            | 49
            | 50
            | 52
            | 53
            | 54
            | 62
            | 63
            | 68
            | 79
            | 80
            | 85
            | 104
            | 111
            | 116
            | 135
            | 136
            | 139
            | 145
            | 157
            | 141
            | 158
            | 177
            | 178
            | 169
            | 170
            | 171
            | 246
            | 373
            | 222
            | 384
            | 385
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
            if name == "Mechanic_IsObject" && ability != 333 {
                return Err(format!("ability {ability}: unsupported object/team aura"));
            }
            if !matches!(
                *state,
                21 | 22 | 23 | 24 | 25 | 29 | 30 | 33 | 34 | 35 | 36 | 40 | 41 | 42
                    ..=51
                        | 52..=62
                        | 87
                        | 88
                        | 89
                        | 99
                        | 64
                        | 65
                        | 66
                        | 67
                        | 69
                        | 74
                        | 75
                        | 20
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
            if matches!(effect.property, 24 | 27 | 29 | 141 | 222 | 384 | 385)
                && catalog.abilities[&ability].family < 0
            {
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
        22 => true, // Points fields supply tooltip variables, not an HP operation.
        27 => {
            p[0] >= 0
                && p[2] >= 0
                && p[3] >= p[0]
                && (p[4] == 0 || catalog.states.contains_key(&(p[4] as u32)))
                && p[5] == 0
        }
        32 => p[2..].iter().skip(1).all(|v| *v == 0),
        79 | 157 | 177 => catalog.states.contains_key(&(p[0] as u32)),
        28 => effect.aura != 0 && p[2] >= -1 && p[4..].iter().all(|v| *v == 0),
        68 => p[2..].iter().all(|v| *v == 0),
        104 => {
            [p[2], p[3]]
                .iter()
                .all(|s| *s == 0 || catalog.states.contains_key(&(*s as u32)))
                && p[4] >= 0
                && p[5] == 0
        }
        135 => {
            matches!(p[1], 0 | 100) && p[2] == 0 && p[3] == 0 && matches!(p[4], 0 | 1) && p[5] == 0
        }
        136 => matches!(p[0], 0 | 1) && p[1..].iter().all(|v| *v == 0),
        139 | 145 => p[1..].iter().all(|v| *v == 0),
        85 => catalog.states.contains_key(&(p[0] as u32)),
        111 => p[0] > 0 && p[0] <= 100 && p[2..].iter().all(|v| *v == 0),
        169 | 171 => {
            [p[2], p[3]]
                .iter()
                .all(|s| *s == 0 || catalog.states.contains_key(&(*s as u32)))
                && p[5] == 0
        }
        170 => catalog.states.contains_key(&(p[2] as u32)) && p[5] == 0,
        246 => (1..=3).contains(&p[0]) && p[1..].iter().all(|v| *v == 0),
        373 => {
            (0..3).contains(&p[0])
                && catalog.abilities.contains_key(&(p[1] as u32))
                && p[2..].iter().all(|v| *v == 0)
        }
        116 => true, // No named parameters; recalculates priority from current speed.
        158 => p.iter().all(|v| *v == 0),
        24 => p[3..].iter().all(|v| *v == 0),
        29 => {
            [p[2], p[3]]
                .iter()
                .all(|s| *s == 0 || catalog.states.contains_key(&(*s as u32)))
                && p[5] == 0
        }
        62 | 384 | 385 => p[2..].iter().all(|v| *v == 0),
        80 => effect.aura != 0 && p[2] > 0 && p[4..].iter().all(|v| *v == 0),
        141 => {
            (p[2] == 0 || catalog.states.contains_key(&(p[2] as u32)))
                && p[4..].iter().all(|v| *v == 0)
        }
        178 => {
            effect.aura != 0
                && p[2] >= 0
                && (p[3] == 0 || catalog.states.contains_key(&(p[3] as u32)))
        }
        222 => p[3..].iter().all(|v| *v == 0) && p[2] >= 0,
        31 => catalog.states.contains_key(&(p[0] as u32)),
        49 => effect.aura != 0 && p[2..].iter().all(|v| *v == 0),
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
