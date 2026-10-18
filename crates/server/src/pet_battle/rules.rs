use super::{Catalog, Effect};
use std::collections::BTreeSet;

pub const RANDOM_SCALE: u32 = 1_000_000;
pub const BASE_VARIANCE: i32 = 20; // Blizzard5.3 accuracy article: +/-10%.
pub const POWER_COEFFICIENT: f64 = 0.05; // Retail SharedPetBattleTemplates AttackBonus.
pub const CRITICAL_MULTIPLIER: f64 = 1.5; // Blizzard5.3 accuracy article.

const TWO_WAY_WALLS: [(u32, u32); 7] = [
    (325, 326),
    (433, 434),
    (444, 443),
    (465, 464),
    (479, 480),
    (2318, 464),
    (2398, 2397),
];

pub(super) fn is_two_way_wall(ability: u32) -> bool {
    TWO_WAY_WALLS.iter().any(|(_, aura)| *aura == ability)
}

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
            | 44
            | 59
            | 66
            | 67
            | 75
            | 76
            | 77
            | 96
            | 100
            | 131
            | 172
            | 226
            | 159
            | 45
            | 65
            | 97
            | 149
            | 254
            | 117
            | 129
            | 168
            | 107
            | 150
            | 112
            | 370
            | 223
            | 194
    )
}
/// DB2 descriptions identify first-active/other-enemy damage for these three-row attacks.
pub(super) fn damages_enemy_team(ability: u32) -> bool {
    matches!(
        ability,
        189 | 258
            | 299
            | 319
            | 404
            | 419
            | 649
            | 774
            | 920
            | 923
            | 979
            | 1013
            | 1025
            | 1031
            | 1035
            | 1531
            | 1587
            | 1738
            | 1761
            | 1764
            | 1989
            | 2104
            | 2303
            | 2309
            | 2346
            | 2356
            | 2364
            | 2378
            | 2385
            | 2410
            | 2417
            | 2523
            | 2539
    )
}

/// Recursive admission: every turn and referenced aura must be executable before RNG draws.
/// Reactive events remain excluded except bounded Leech Life383 healing and
/// Nevermore738/Spell Gulp1859 wearer-owned ability-use callbacks.
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
        for (state, value) in states {
            let name = &catalog.states[state];
            let known_object = matches!(ability, 189 | 294 | 333) || is_two_way_wall(ability);
            if name == "Mechanic_IsObject" && !known_object {
                return Err(format!("ability {ability}: unsupported object/team aura"));
            }
            let wall_state = is_two_way_wall(ability) && matches!(*state, 123 | 126);
            let level_health_state =
                *state == 2 && matches!((ability, *value), (577 | 610, 500) | (1990, 1000));
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
            ) && !wall_state
                && !level_health_state
            {
                return Err(format!(
                    "ability {ability}: unsupported aura state {state} {name}"
                ));
            }
        }
    }
    for turn in catalog.ability_turns(ability) {
        if (turn.kind == 0 && (turn.event != 255 || turn.order == 0))
            || (turn.kind == 1
                && !matches!(turn.event, 0 | 5 | 6 | 7 | 8 | 10 | 11)
                && !(ability == 383 && turn.event == 2)
                && !(matches!(ability, 738 | 1859) && turn.event == 9))
            || turn.kind > 1
        {
            return Err(format!(
                "ability {ability}: unsupported turn {} kind{} event{}",
                turn.id, turn.kind, turn.event
            ));
        }
        for effect in catalog.turn_effects(turn.id) {
            validate_effect(catalog, effect)?;
            let description = catalog.abilities[&ability].description.to_ascii_lowercase();
            let ambiguous_team_target = [
                "all enemy pets",
                "all enemies",
                "enemy team",
                "enemy's pets",
                "backline",
                "back line",
                "both teams",
            ]
            .iter()
            .any(|phrase| description.contains(phrase));
            if matches!(effect.property, 24 | 222)
                && ambiguous_team_target
                && !damages_enemy_team(ability)
                || matches!(effect.property, 23 | 53 | 111) && matches!(ability, 70 | 295 | 714)
            {
                return Err(format!(
                    "ability {ability}: undetermined team target routing"
                ));
            }
            if effect.property == 135 && !matches!(ability, 282 | 663 | 1025 | 2209) {
                return Err(format!(
                    "ability {ability}: undetermined kill target/passive semantics"
                ));
            }
            if effect.property == 80
                && catalog
                    .ability_states
                    .get(&effect.aura)
                    .is_some_and(|s| s.contains_key(&71) || s.contains_key(&72))
            {
                return Err(format!(
                    "ability {ability}: undetermined weather flat-modifier power ownership"
                ));
            }
            if matches!(
                effect.property,
                24 | 27
                    | 29
                    | 45
                    | 65
                    | 97
                    | 149
                    | 59
                    | 62
                    | 66
                    | 68
                    | 75
                    | 76
                    | 77
                    | 96
                    | 141
                    | 169
                    | 170
                    | 222
                    | 226
                    | 384
                    | 385
                    | 370
            ) && catalog.abilities[&ability].family < 0
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
        // Mind Games offers the wearer its own detriment, not an enemy attack.
        117 => {
            catalog.turns[&effect.turn].ability == 2390
                && (1..=3).contains(&p[0])
                && p[1] >= 0
                && p[2..].iter().all(|v| *v == 0)
        }
        129 => {
            matches!(catalog.turns[&effect.turn].ability, 738 | 1859)
                && catalog.turns[&effect.turn].event == 9
                && p[0] >= 0
                && p[1..].iter().all(|v| *v == 0)
        }
        168 => {
            matches!(catalog.turns[&effect.turn].ability, 522 | 1858)
                && matches!(effect.aura, 738 | 1859)
                && p[0] == 0
                && p[2] > 0
                && p[3..].iter().all(|v| *v == 0)
        }
        // Stench retains its team routing; these two miss branches explicitly select self.
        159 => {
            let ability = catalog.turns[&effect.turn].ability;
            let self_after_miss =
                matches!(ability, 762 | 989) && effect.order == 3 && effect.aura == 0;
            p.iter().all(|v| *v == 0) && (matches!(ability, 527 | 1010) || self_after_miss)
        }
        194 => {
            matches!(catalog.turns[&effect.turn].ability, 762 | 989)
                && effect.order == 2
                && effect.aura == 0
                && p.iter().all(|v| *v == 0)
        }
        // Life Exchange has an Accuracy field, not damage/healing Points.
        67 => {
            catalog.turns[&effect.turn].ability == 277
                && effect.order == 1
                && effect.aura == 0
                && p == [100, 0, 0, 0, 0, 0]
        }
        // Lock-On marks the enemy for one round before its second-use blast.
        77 => {
            catalog.turns[&effect.turn].ability == 301
                && effect.order == 1
                && effect.aura == 300
                && p == [45, 100, 1, 0, 0, 0]
        }
        // Launch Rocket's first use sets up294; its second use launches the rocket.
        75 => {
            catalog.turns[&effect.turn].ability == 293
                && effect.order == 1
                && effect.aura == 294
                && p == [45, 100, 0, 0, 0, 0]
        }
        // Fade explicitly swaps its user randomly after removing the dodge aura.
        // Other107 descriptions do not establish random versus selected routing.
        107 => {
            catalog.turns[&effect.turn].ability == 1084
                && p[0] == 100
                && p[1..].iter().all(|v| *v == 0)
        }
        150 => {
            let ability = catalog.turns[&effect.turn].ability;
            TWO_WAY_WALLS.contains(&(ability, effect.aura))
                && p[0] == 0
                && p[1] == 100
                && p[2..].iter().all(|v| *v == 0)
        }
        // Refuge's other two friendly slots require the authored Critter family.
        223 => {
            catalog.turns[&effect.turn].ability == 1344
                && (2..=3).contains(&effect.order)
                && effect.aura == 759
                && p == [0, 100, 6, 5, 0, 0]
        }
        // Dragon's Call's authored bonus counts additional teammates of the same species.
        370 => {
            catalog.turns[&effect.turn].ability == 2283
                && effect.order == 1
                && effect.aura == 0
                && p == [25, 100, 7, 0, 0, 0]
        }
        // Raise Dead and Mass Resurrection explicitly revive the two other allies.
        112 => {
            matches!(catalog.turns[&effect.turn].ability, 2347 | 2348)
                && (1..=2).contains(&effect.order)
                && effect.aura == 0
                && p == [5, 100, 0, 1, 0, 0]
        }
        131 => effect.aura != 0 && p[0] == 0 && p[2] >= 0 && p[3..].iter().all(|v| *v == 0),
        172 => {
            effect.aura != 0
                && p[0] == 0
                && p[2] >= 0
                && matches!(p[4], 0 | 1)
                && [p[1], p[3]]
                    .iter()
                    .all(|s| *s == 0 || catalog.states.contains_key(&(*s as u32)))
        }
        45 | 97 => p[0] == 20 && p[2] == 150 && p[3..].iter().all(|v| *v == 0),
        149 => p[0] >= 0 && p[2..].iter().all(|v| *v == 0),
        254 => {
            p[0] >= 0
                && (1..=100).contains(&p[2])
                && matches!(p[3], 100 | 300)
                && p[4] >= 0
                && p[5] == 0
        }
        65 | 226 => {
            p[0] >= 0
                && p[2] >= 0
                && catalog.states.contains_key(&(p[3] as u32))
                && matches!(p[4], 0 | 1)
                && p[5] == 0
        }
        44 | 59 => p[0] >= 0 && p[2..].iter().all(|v| *v == 0),
        // The three authenticated low-health attacks all specify below25%/double.
        66 => p[0] >= 0 && p[2] == 100 && p[3..].iter().all(|v| *v == 0),
        76 => effect.aura != 0 && p[0] >= 0 && p[2..].iter().all(|v| *v == 0),
        96 => {
            [p[2], p[3]]
                .iter()
                .all(|s| *s == 0 || catalog.states.contains_key(&(*s as u32)))
                && matches!(p[4], 0 | 1)
                && p[5] == 0
        }
        100 => {
            (0..=10).contains(&p[2])
                && (1..=10).contains(&p[3])
                && p[4] >= 0
                && p[5] == 0
                && (2..=3).contains(&effect.order)
                && matches!(catalog.turns[&effect.turn].ability, 533 | 922 | 1956)
        }
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
        135 if catalog.turns[&effect.turn].ability == 1025 => {
            effect.order == 4 && effect.aura == 0 && p == [200, 100, 0, 0, 0, 0]
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
                && (p[1] != 0 || effect.aura == 1904)
                && p[2] >= -1
                && matches!(p[3], 0 | 1)
                && p[4..].iter().all(|v| *v == 0)
        }
        50 => {
            effect.aura != 0
                && p[2] >= -1
                && p[3] >= 0
                && [p[4], p[5]]
                    .iter()
                    .all(|s| *s == 0 || catalog.states.contains_key(&(*s as u32)))
        }
        54 => effect.aura != 0 && p[2] >= -1 && p[3] >= 0 && p[4..].iter().all(|v| *v == 0),
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
