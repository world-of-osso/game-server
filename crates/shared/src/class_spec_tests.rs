use super::*;
use crate::aura::Auras;

#[test]
fn spec_total_count() {
    assert_eq!(SPECIALIZATIONS.len(), 39);
}

#[test]
fn spec_by_id_known() {
    let arms = spec_by_id(71).unwrap();
    assert_eq!(arms.name, "Arms");
    assert_eq!(arms.class, Class::Warrior);
    assert_eq!(arms.role, Role::MeleeDps);
}

#[test]
fn spec_by_id_unknown() {
    assert!(spec_by_id(9999).is_none());
}

#[test]
fn specs_for_warrior() {
    let specs = specs_for_class(Class::Warrior);
    assert_eq!(specs.len(), 3);
    let names: Vec<&str> = specs.iter().map(|s| s.name).collect();
    assert!(names.contains(&"Arms"));
    assert!(names.contains(&"Fury"));
    assert!(names.contains(&"Protection"));
}

#[test]
fn specs_for_druid_has_4() {
    let specs = specs_for_class(Class::Druid);
    assert_eq!(specs.len(), 4);
}

#[test]
fn specs_for_demon_hunter_has_2() {
    let specs = specs_for_class(Class::DemonHunter);
    assert_eq!(specs.len(), 2);
}

#[test]
fn every_class_has_specs() {
    for id in 1..=13 {
        if let Some(class) = Class::from_id(id) {
            let specs = specs_for_class(class);
            assert!(
                specs.len() >= 2,
                "{class:?} has {} specs, expected >= 2",
                specs.len()
            );
        }
    }
}

#[test]
fn all_spec_ids_unique() {
    let mut ids: Vec<u32> = SPECIALIZATIONS.iter().map(|s| s.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), SPECIALIZATIONS.len());
}

#[test]
fn tank_specs_exist() {
    let tanks: Vec<_> = SPECIALIZATIONS
        .iter()
        .filter(|s| s.role == Role::Tank)
        .collect();
    assert_eq!(tanks.len(), 6);
}

#[test]
fn healer_specs_exist() {
    let healers: Vec<_> = SPECIALIZATIONS
        .iter()
        .filter(|s| s.role == Role::Healer)
        .collect();
    assert_eq!(healers.len(), 7);
}

#[test]
fn mastery_for_known_spec() {
    let m = mastery_for_spec(71).unwrap();
    assert_eq!(m.name, "Striking");
    assert_eq!(m.coefficient, 2.0);
}

#[test]
fn mastery_for_unknown_spec() {
    assert!(mastery_for_spec(9999).is_none());
}

#[test]
fn effective_mastery_arms_warrior() {
    let bonus = effective_mastery(71, 10.0);
    assert!((bonus - 20.0).abs() < 0.001);
}

#[test]
fn effective_mastery_frost_mage() {
    let bonus = effective_mastery(64, 8.0);
    assert!((bonus - 18.0).abs() < 0.001);
}

#[test]
fn effective_mastery_unknown_spec_returns_zero() {
    assert_eq!(effective_mastery(9999, 10.0), 0.0);
}

#[test]
fn every_spec_has_mastery() {
    for spec in SPECIALIZATIONS {
        assert!(
            mastery_for_spec(spec.id).is_some(),
            "spec {} ({}) has no mastery",
            spec.id,
            spec.name
        );
    }
}

#[test]
fn mastery_defs_count_matches_specs() {
    assert_eq!(MASTERY_DEFS.len(), SPECIALIZATIONS.len());
}

#[test]
fn mastery_integrates_with_rating_system() {
    use crate::formulas::{RatingType, apply_secondary_dr, rating_to_percent};
    let raw_points = rating_to_percent(500.0, 80, RatingType::Mastery).unwrap();
    let after_dr = apply_secondary_dr(raw_points, RatingType::Mastery);
    let bonus = effective_mastery(71, after_dr);
    assert!(bonus > 0.0, "should produce positive mastery bonus");
    assert!(bonus > after_dr, "coeff 2.0 should amplify points");
}

fn sample_talents() -> Vec<TalentDef> {
    vec![
        TalentDef {
            id: 1001,
            name: "Impale".into(),
            spec_id: 71,
            effects: vec![AuraEffect::ModCritChance { percent: 0.10 }],
        },
        TalentDef {
            id: 1002,
            name: "Deep Wounds".into(),
            spec_id: 71,
            effects: vec![AuraEffect::ModDamageDone { percent: 0.05 }],
        },
        TalentDef {
            id: 1003,
            name: "Warbringer".into(),
            spec_id: 73,
            effects: vec![AuraEffect::ModThreat { percent: 0.10 }],
        },
    ]
}

#[test]
fn build_passive_auras_from_talents() {
    let defs = sample_talents();
    let selected = SelectedTalents {
        talent_ids: [1001, 1002].into_iter().collect(),
    };
    let auras = selected.build_passive_auras(&defs);
    assert_eq!(auras.len(), 2);
    assert_eq!(auras[0].spell_id, 1001);
    assert_eq!(auras[0].remaining, f32::MAX);
    assert_eq!(
        auras[0].effects[0],
        Some(AuraEffect::ModCritChance { percent: 0.10 })
    );
}

#[test]
fn build_passive_auras_skips_unknown() {
    let defs = sample_talents();
    let selected = SelectedTalents {
        talent_ids: [1001, 9999].into_iter().collect(),
    };
    let auras = selected.build_passive_auras(&defs);
    assert_eq!(auras.len(), 1);
}

#[test]
fn build_passive_auras_empty() {
    let defs = sample_talents();
    let selected = SelectedTalents::default();
    let auras = selected.build_passive_auras(&defs);
    assert!(auras.is_empty());
}

#[test]
fn talent_aura_integrates_with_damage_multiplier() {
    let defs = sample_talents();
    let selected = SelectedTalents {
        talent_ids: [1002].into_iter().collect(),
    };
    let passive_auras = selected.build_passive_auras(&defs);

    let mut auras = Auras::default();
    for aura in passive_auras {
        auras.apply(aura);
    }
    assert!((auras.damage_done_multiplier() - 1.05).abs() < 0.001);
}

#[test]
fn talent_aura_integrates_with_threat_multiplier() {
    let defs = sample_talents();
    let selected = SelectedTalents {
        talent_ids: [1003].into_iter().collect(),
    };
    let passive_auras = selected.build_passive_auras(&defs);

    let mut auras = Auras::default();
    for aura in passive_auras {
        auras.apply(aura);
    }
    assert!((auras.threat_multiplier() - 1.10).abs() < 0.001);
}

#[test]
fn class_from_id_valid() {
    assert_eq!(Class::from_id(1), Some(Class::Warrior));
    assert_eq!(Class::from_id(6), Some(Class::DeathKnight));
    assert_eq!(Class::from_id(11), Some(Class::Druid));
    assert_eq!(Class::from_id(13), Some(Class::Evoker));
}

#[test]
fn class_from_id_invalid() {
    assert_eq!(Class::from_id(0), None);
    assert_eq!(Class::from_id(14), None);
    assert_eq!(Class::from_id(255), None);
}

#[test]
fn class_id_round_trip() {
    for id in 1..=13 {
        if let Some(class) = Class::from_id(id) {
            assert_eq!(class.id(), id);
        }
    }
}

#[test]
fn class_uses_mana() {
    assert!(!Class::Warrior.uses_mana());
    assert!(!Class::Rogue.uses_mana());
    assert!(!Class::DeathKnight.uses_mana());
    assert!(Class::Paladin.uses_mana());
    assert!(Class::Mage.uses_mana());
    assert!(Class::Priest.uses_mana());
    assert!(Class::Druid.uses_mana());
}

#[test]
fn class_primary_resource() {
    use crate::spell_data::ResourceType;
    assert_eq!(Class::Warrior.primary_resource(), ResourceType::Rage);
    assert_eq!(Class::Rogue.primary_resource(), ResourceType::Energy);
    assert_eq!(
        Class::DeathKnight.primary_resource(),
        ResourceType::RunicPower
    );
    assert_eq!(Class::Hunter.primary_resource(), ResourceType::Focus);
    assert_eq!(Class::Mage.primary_resource(), ResourceType::Mana);
    assert_eq!(Class::DemonHunter.primary_resource(), ResourceType::Energy);
}

#[test]
fn class_all_13_variants() {
    let classes: [Class; 13] = [
        Class::Warrior,
        Class::Paladin,
        Class::Hunter,
        Class::Rogue,
        Class::Priest,
        Class::DeathKnight,
        Class::Shaman,
        Class::Mage,
        Class::Warlock,
        Class::Monk,
        Class::Druid,
        Class::DemonHunter,
        Class::Evoker,
    ];
    assert_eq!(classes.len(), 13);
    let mut ids: Vec<u8> = classes.iter().map(|c| c.id()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 13);
}
