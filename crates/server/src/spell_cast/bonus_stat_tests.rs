//! TC a352b1fa Unit.cpp9882: aura467 scales bonus stats, never create/base stats.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{ActiveSpec, Level, Player, UnitStats};

const STRENGTH: u32 = 13_372;
const AGILITY: u32 = 13_365;
const INTELLECT: u32 = 13_366;
const FLAT_STAT_SPELLS: [u32; 3] = [STRENGTH, AGILITY, INTELLECT];

fn bonus_harness(negation: u32, class: u8, spec: u32) -> Harness {
    let ids: Vec<_> = FLAT_STAT_SPELLS.into_iter().chain([negation]).collect();
    let mut h = harness_with(Vec::new(), &ids, class, true).unwrap();
    let data = world_db().unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &data.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), ActiveSpec(spec)));
    h.app.world_mut().get_mut::<Player>(h.caster).unwrap().race = if class == 11 { 4 } else { 1 };
    h.advance(4);
    h
}

#[test]
fn class_completeness_bonus_stat_percent_scales_gear_and_flat_before_total_percent() {
    let input = crate::stats::StatInputs {
        class: 8,
        race: 1,
        level: 80,
        primary_stat: Some(crate::stats::PrimaryStat::Intellect),
        mastery_coefficient: 0.0,
        attack_power_per_strength: 0.0,
        attack_power_per_agility: 0.0,
        gear_primary: UnitStats {
            intellect: 40.0,
            ..default()
        },
        gear_ratings: default(),
    };
    let mods = crate::stats::StatModifiers {
        primary_flat: UnitStats {
            intellect: 20.0,
            ..default()
        },
        primary_bonus_pct: UnitStats {
            intellect: native_stat_pct(162697, 3),
            ..default()
        },
        primary_pct: UnitStats {
            intellect: 20.0,
            ..default()
        },
        ..default()
    };
    let base = crate::stats::base_primary_stats(8, 1, 80).unwrap();
    let suppressed = crate::stats::compute_character_stats(&input, Some(&mods)).unwrap();
    assert_eq!(suppressed.primary.intellect, base.intellect * 1.2);
    let restored = crate::stats::compute_character_stats(
        &input,
        Some(&crate::stats::StatModifiers {
            primary_bonus_pct: default(),
            ..mods
        }),
    )
    .unwrap();
    assert_eq!(
        restored.primary.intellect,
        (base.intellect + 40.0 + 20.0) * 1.2
    );
}

#[test]
fn class_completeness_bonus_stat_percent_preserves_unselected_and_spirit_base_values() {
    let input = crate::stats::StatInputs {
        class: 8,
        race: 1,
        level: 80,
        primary_stat: None,
        mastery_coefficient: 0.0,
        attack_power_per_strength: 0.0,
        attack_power_per_agility: 0.0,
        gear_primary: UnitStats {
            strength: 7.0,
            agility: 11.0,
            stamina: 13.0,
            intellect: 17.0,
            spirit: 19.0,
        },
        gear_ratings: default(),
    };
    let mods = crate::stats::StatModifiers {
        primary_flat: UnitStats {
            spirit: 3.0,
            ..default()
        },
        primary_bonus_pct: UnitStats {
            spirit: native_stat_pct(162697, 3),
            ..default()
        },
        ..default()
    };
    let result = crate::stats::compute_character_stats(&input, Some(&mods)).unwrap();
    let mut expected = crate::stats::base_primary_stats(8, 1, 80).unwrap();
    expected.strength += 7.0;
    expected.agility += 11.0;
    expected.stamina += 13.0;
    expected.intellect += 17.0;
    assert_eq!(result.primary, expected);
}

fn native_flat(spell: u32) -> f32 {
    world_db().unwrap().store.get(spell).unwrap().effects[0].base_points
}

fn native_stat_pct(spell: u32, stat: i32) -> f32 {
    world_db()
        .unwrap()
        .store
        .get(spell)
        .unwrap()
        .effects
        .iter()
        .filter(|effect| effect.aura == 467 && effect.misc_values[0] == stat)
        .map(|effect| effect.base_points)
        .sum()
}

fn expected_primary(h: &Harness, negation: u32) -> UnitStats {
    let player = h.app.world().get::<Player>(h.caster).unwrap();
    let mut base = crate::stats::base_primary_stats(player.class, player.race, 80).unwrap();
    base.strength += native_flat(STRENGTH) * (1.0 + native_stat_pct(negation, 0) / 100.0);
    base.agility += native_flat(AGILITY) * (1.0 + native_stat_pct(negation, 1) / 100.0);
    base.intellect += native_flat(INTELLECT) * (1.0 + native_stat_pct(negation, 3) / 100.0);
    base
}

#[test]
fn class_completeness_native_bonus_stat_negation_preserves_base_for_all_six_spec_categories() {
    for (negation, class, spec) in [
        (162697, 3, 253),
        (162698, 1, 71),
        (162699, 8, 62),
        (162701, 5, 257),
        (162702, 1, 73),
        (162700, 11, 104),
    ] {
        let h = bonus_harness(negation, class, spec);
        let stats = h
            .app
            .world()
            .get::<crate::stats::CharacterStats>(h.caster)
            .unwrap();
        assert_eq!(
            stats.primary,
            expected_primary(&h, negation),
            "native negation{negation}, spec{spec}"
        );
        let published = h.app.world().get::<UnitStats>(h.caster).unwrap();
        assert_eq!(*published, stats.primary, "replicated stat sheet");
    }
}

#[test]
fn class_completeness_native_bonus_stat_negation_removal_restores_only_bonus_points() {
    const NEGATION: u32 = 162_697;
    let mut h = bonus_harness(NEGATION, 3, 253);
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
            .primary,
        expected_primary(&h, NEGATION)
    );
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(
            FLAT_STAT_SPELLS.to_vec(),
        ));
    h.advance(4);
    let player = h.app.world().get::<Player>(h.caster).unwrap();
    let mut expected = crate::stats::base_primary_stats(player.class, player.race, 80).unwrap();
    expected.strength += native_flat(STRENGTH);
    expected.agility += native_flat(AGILITY);
    expected.intellect += native_flat(INTELLECT);
    assert_eq!(*h.app.world().get::<UnitStats>(h.caster).unwrap(), expected);
}
