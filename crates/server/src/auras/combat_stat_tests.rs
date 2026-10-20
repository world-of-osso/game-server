//! Native70448 inputs; TC a352b1fa StatSystem.cpp / Player.cpp formulas.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness_with};
use crate::stats::{
    CharacterStats, PrimaryStat, StatInputs, StatModifiers, compute_character_stats,
};
use shared::components::{CombatRatings, Health, Mana, UnitStats};

fn fixture(ids: &[u32]) -> Harness {
    let mut h = harness_with(Vec::new(), ids, 8, true).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    h.advance(3);
    let caster = h.caster;
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |_| true);
    h.advance(3);
    h
}

fn apply(h: &mut Harness, spell_id: u32, index: u32) {
    let caster = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target: caster,
            spell_id,
            effect_mask: 1 << index,
            source: AuraSource::Cast,
        },
    );
    h.advance(3);
    assert!(has_aura(h.app.world(), caster, spell_id));
}

fn computed(h: &Harness, ratings: CombatRatings) -> CharacterStats {
    let input = StatInputs {
        class: 8,
        race: 1,
        level: 80,
        primary_stat: Some(PrimaryStat::Intellect),
        mastery_coefficient: 1.0,
        attack_power_per_strength: 1.0,
        attack_power_per_agility: 1.0,
        ranged_attack_power_per_agility: 1.0,
        gear_primary: UnitStats::default(),
        gear_ratings: ratings,
    };
    compute_character_stats(&input, h.app.world().get::<StatModifiers>(h.caster)).unwrap()
}

#[test]
#[ignore = "aura-stat-mods RED: native rating405"]
fn aura_stat_native_405_scales_selected_ratings_not_percent_chance() {
    let mut h = fixture(&[117216, 417489, 1262442]);
    let ratings = CombatRatings {
        crit: 1234.0,
        haste: 1500.0,
        mastery: 1700.0,
        parry: 200.0,
        ..Default::default()
    };
    apply(&mut h, 117216, 1); //10%, mask1792 = all crit ratings.
    assert_eq!(computed(&h, ratings).ratings.crit, 1357.0); //1234 + trunc(123.4)
    assert_eq!(computed(&h, ratings).ratings.haste, 1500.0);
    apply(&mut h, 417489, 1); //5%, mask917504 = all haste ratings.
    assert_eq!(computed(&h, ratings).ratings.haste, 1575.0);
    apply(&mut h, 1262442, 1); //5%, mask33554432 = mastery.
    assert_eq!(computed(&h, ratings).ratings.mastery, 1785.0);
    assert_eq!(computed(&h, ratings).ratings.parry, 200.0);
}

#[test]
#[ignore = "aura-stat-mods RED: native conversion463"]
fn aura_stat_native_463_uses_base_crit_rating_without_consuming_it() {
    let mut h = fixture(&[161797, 117216]);
    let ratings = CombatRatings {
        crit: 1234.0,
        parry: 200.0,
        ..Default::default()
    };
    apply(&mut h, 161797, 0); //100% of base crit rating to parry.
    apply(&mut h, 117216, 1); //405 must not feed converted base crit.
    let actual = computed(&h, ratings).ratings;
    assert_eq!(actual.parry, 1434.0);
    assert_eq!(actual.crit, 1357.0);
    let caster = h.caster;
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |a| {
        a.spell_id == 161797
    });
    h.advance(3);
    assert_eq!(computed(&h, ratings).ratings.parry, 200.0);
}

#[test]
#[ignore = "aura-stat-mods RED: native mana419"]
fn aura_stat_native_419_changes_max_mana_but_not_create_mana_cost_basis() {
    let mut h = fixture(&[429399, 443448]);
    let base = h
        .app
        .world()
        .get::<CharacterStats>(h.caster)
        .unwrap()
        .base_mana;
    apply(&mut h, 429399, 1); //5%
    apply(&mut h, 443448, 1); //10%, independent product.
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().max,
        (base * 1.05 * 1.10).round()
    );
    assert_eq!(
        h.app
            .world()
            .get::<CharacterStats>(h.caster)
            .unwrap()
            .base_mana,
        base
    );
    let caster = h.caster;
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |_| true);
    h.advance(3);
    assert_eq!(h.app.world().get::<Mana>(caster).unwrap().max, base);
}

#[test]
#[ignore = "aura-stat-mods RED: native AP override404"]
fn aura_stat_native_404_replaces_melee_and_ranged_ap_with_spell_power_percent() {
    let mut h = fixture(&[137012]);
    apply(&mut h, 137012, 7); //104%, not ordinary +104% AP.
    let actual = computed(&h, CombatRatings::default());
    assert_eq!(
        actual.attack_power,
        actual.spell_power.min(actual.healing_power) * 1.04
    );
    assert_eq!(actual.ranged_attack_power, actual.attack_power);
    let caster = h.caster;
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |_| true);
    h.advance(3);
    assert_ne!(
        computed(&h, CombatRatings::default()).attack_power,
        actual.attack_power
    );
}

#[test]
#[ignore = "aura-stat-mods RED: native health145"]
fn aura_stat_native_145_preserves_health_fraction_and_restores_on_remove() {
    //Census22842's145 has amount0. Native182775 supplies a nonzero50% oracle.
    let mut h = fixture(&[22842, 182775]);
    let caster = h.caster;
    let before = *h.app.world().get::<Health>(caster).unwrap();
    h.app.world_mut().get_mut::<Health>(caster).unwrap().current = before.max / 2.0;
    apply(&mut h, 22842, 2);
    assert_eq!(h.app.world().get::<Health>(caster).unwrap().max, before.max);
    apply(&mut h, 182775, 0);
    assert_eq!(
        h.app.world().get::<Health>(caster).unwrap().max,
        (before.max * 1.5).floor()
    );
    assert_eq!(
        h.app.world().get::<Health>(caster).unwrap().current,
        (before.max * 1.5).floor() / 2.0
    );
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |_| true);
    h.advance(3);
    assert_eq!(h.app.world().get::<Health>(caster).unwrap().max, before.max);
}

#[test]
#[ignore = "aura-stat-mods RED: native absorb taken422"]
fn aura_stat_native_422_scales_incoming_shield_capacity_once_not_each_hit() {
    let mut h = fixture(&[391571, 17]);
    let caster = h.caster;
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(caster)
        .unwrap()
        .healing_power = 1000.0;
    apply(&mut h, 391571, 0); //15% absorb received.
    //Stats recompute is done; freeze only the coefficient input for this shield.
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(caster)
        .unwrap()
        .healing_power = 1000.0;
    apply(&mut h, 17, 0); //native4.638×SP =4638, ×1.15 =5334 rounded.
    let (remaining, absorbed) = absorb_damage(h.app.world_mut(), caster, 1, 5000.0);
    assert_eq!((remaining, absorbed), (0.0, 5000.0));
    let (remaining, absorbed) = absorb_damage(h.app.world_mut(), caster, 1, 1000.0);
    assert_eq!((remaining, absorbed), (666.0, 334.0));
    assert!(!has_aura(h.app.world(), caster, 17));
}

#[test]
#[ignore = "aura-stat-mods RED: native temporary threat103"]
fn aura_stat_native_103_reselects_without_erasing_base_or_new_threat() {
    use shared::threat::ThreatTable;
    let mut h = fixture(&[586]); //NativeFade: -90000000 temporary flat threat.
    let caster = h.caster;
    let other = crate::spell_cast::test_harness::spawn_player(&mut h.app, 8, 1.0);
    let mut table = ThreatTable::default();
    table.add_threat(caster.to_bits(), 200.0);
    table.add_threat(other.to_bits(), 100.0);
    h.app.world_mut().entity_mut(h.npc).insert(table);
    assert_eq!(
        crate::creature_threat::current_victim(h.app.world_mut(), h.npc),
        Some(caster)
    );
    apply(&mut h, 586, 0);
    assert_eq!(
        crate::creature_threat::current_victim(h.app.world_mut(), h.npc),
        Some(other)
    );
    h.app
        .world_mut()
        .get_mut::<ThreatTable>(h.npc)
        .unwrap()
        .add_threat(caster.to_bits(), 100.0);
    assert_eq!(
        h.app
            .world()
            .get::<ThreatTable>(h.npc)
            .unwrap()
            .threat_for(caster.to_bits()),
        300.0
    );
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |a| {
        a.spell_id == 586
    });
    assert_eq!(
        crate::creature_threat::current_victim(h.app.world_mut(), h.npc),
        Some(caster)
    );
    assert_eq!(
        h.app
            .world()
            .get::<ThreatTable>(h.npc)
            .unwrap()
            .threat_for(caster.to_bits()),
        300.0
    );
}
