use super::*;

#[test]
fn damage_done_no_auras() {
    let auras = Auras::default();
    assert_eq!(auras.damage_done_multiplier(), 1.0);
}

#[test]
fn damage_done_single_buff() {
    let mut auras = Auras::default();
    auras.active.push(make_damage_done_buff(100, 1, 0.10));
    assert!((auras.damage_done_multiplier() - 1.10).abs() < 0.001);
}

#[test]
fn damage_done_multiplicative_stacking() {
    let mut auras = Auras::default();
    auras.active.push(make_damage_done_buff(100, 1, 0.10));
    auras.active.push(make_damage_done_buff(200, 2, 0.20));
    assert!((auras.damage_done_multiplier() - 1.32).abs() < 0.001);
}

#[test]
fn damage_done_negative_reduces() {
    let mut auras = Auras::default();
    auras.active.push(make_damage_done_buff(100, 1, -0.20));
    assert!((auras.damage_done_multiplier() - 0.80).abs() < 0.001);
}

#[test]
fn damage_taken_single_debuff() {
    let mut auras = Auras::default();
    auras.active.push(make_damage_taken_debuff(100, 1, 0.20));
    assert!((auras.damage_taken_multiplier() - 1.20).abs() < 0.001);
}

#[test]
fn damage_taken_reduction() {
    let mut auras = Auras::default();
    auras.active.push(make_damage_taken_debuff(100, 1, -0.10));
    assert!((auras.damage_taken_multiplier() - 0.90).abs() < 0.001);
}

#[test]
fn damage_taken_multiplicative_stacking() {
    let mut auras = Auras::default();
    auras.active.push(make_damage_taken_debuff(100, 1, -0.10));
    auras.active.push(make_damage_taken_debuff(200, 2, -0.20));
    assert!((auras.damage_taken_multiplier() - 0.72).abs() < 0.001);
}

#[test]
fn damage_modifiers_ignore_other_effects() {
    let mut auras = Auras::default();
    auras.active.push(make_dot(100, 1));
    auras
        .active
        .push(make_shield(200, 1, SpellSchool::Fire, 500.0));
    assert_eq!(auras.damage_done_multiplier(), 1.0);
    assert_eq!(auras.damage_taken_multiplier(), 1.0);
}

#[test]
fn threat_multiplier_no_auras() {
    let auras = Auras::default();
    assert_eq!(auras.threat_multiplier(), 1.0);
}

#[test]
fn threat_multiplier_righteous_fury() {
    let mut auras = Auras::default();
    auras.active.push(make_threat_aura(100, 1, 0.43));
    assert!((auras.threat_multiplier() - 1.43).abs() < 0.001);
}

#[test]
fn threat_multiplier_stacks_multiplicatively() {
    let mut auras = Auras::default();
    auras.active.push(make_threat_aura(100, 1, 0.30));
    auras.active.push(make_threat_aura(200, 1, 0.10));
    assert!((auras.threat_multiplier() - 1.43).abs() < 0.001);
}

#[test]
fn threat_multiplier_reduction() {
    let mut auras = Auras::default();
    auras.active.push(make_threat_aura(100, 1, -0.30));
    assert!((auras.threat_multiplier() - 0.70).abs() < 0.001);
}

#[test]
fn cdr_no_auras_returns_base() {
    let auras = Auras::default();
    assert_eq!(auras.effective_cooldown(100, 10.0), 10.0);
}

#[test]
fn cdr_specific_spell_reduction() {
    let mut auras = Auras::default();
    auras.active.push(make_cdr_aura(1000, 1, 100, 2.0));
    assert_eq!(auras.effective_cooldown(100, 10.0), 8.0);
    assert_eq!(auras.effective_cooldown(200, 10.0), 10.0);
}

#[test]
fn cdr_global_reduction() {
    let mut auras = Auras::default();
    auras.active.push(make_cdr_aura(1000, 1, 0, 1.0));
    assert_eq!(auras.effective_cooldown(100, 8.0), 7.0);
    assert_eq!(auras.effective_cooldown(200, 5.0), 4.0);
}

#[test]
fn cdr_stacks_additively() {
    let mut auras = Auras::default();
    auras.active.push(make_cdr_aura(1000, 1, 100, 2.0));
    auras.active.push(make_cdr_aura(1001, 1, 100, 1.5));
    assert!((auras.effective_cooldown(100, 10.0) - 6.5).abs() < 0.001);
}

#[test]
fn cdr_clamps_to_zero() {
    let mut auras = Auras::default();
    auras.active.push(make_cdr_aura(1000, 1, 100, 20.0));
    assert_eq!(auras.effective_cooldown(100, 8.0), 0.0);
}

#[test]
fn cdr_global_and_specific_stack() {
    let mut auras = Auras::default();
    auras.active.push(make_cdr_aura(1000, 1, 0, 1.0));
    auras.active.push(make_cdr_aura(1001, 1, 100, 2.0));
    assert_eq!(auras.effective_cooldown(100, 10.0), 7.0);
    assert_eq!(auras.effective_cooldown(200, 10.0), 9.0);
}

#[test]
fn speed_no_auras_normal() {
    let auras = Auras::default();
    assert_eq!(auras.movement_speed_multiplier(), 1.0);
    assert!(!auras.is_rooted());
}

#[test]
fn speed_snare_50_percent() {
    let mut auras = Auras::default();
    auras.active.push(make_speed_aura(100, 1, -0.50));
    assert!((auras.movement_speed_multiplier() - 0.50).abs() < 0.001);
}

#[test]
fn speed_root_is_zero() {
    let mut auras = Auras::default();
    auras.active.push(make_speed_aura(100, 1, -1.0));
    assert_eq!(auras.movement_speed_multiplier(), 0.0);
    assert!(auras.is_rooted());
}

#[test]
fn speed_strongest_snare_wins() {
    let mut auras = Auras::default();
    auras.active.push(make_speed_aura(100, 1, -0.30));
    auras.active.push(make_speed_aura(200, 2, -0.70));
    assert!((auras.movement_speed_multiplier() - 0.30).abs() < 0.001);
}

#[test]
fn speed_buff_stacks_additively() {
    let mut auras = Auras::default();
    auras.active.push(make_speed_aura(100, 1, 0.30));
    auras.active.push(make_speed_aura(200, 2, 0.20));
    assert!((auras.movement_speed_multiplier() - 1.50).abs() < 0.001);
}

#[test]
fn speed_buff_and_snare_combined() {
    let mut auras = Auras::default();
    auras.active.push(make_speed_aura(100, 1, -0.50));
    auras.active.push(make_speed_aura(200, 2, 0.20));
    assert!((auras.movement_speed_multiplier() - 0.70).abs() < 0.001);
}

#[test]
fn speed_root_overrides_buff() {
    let mut auras = Auras::default();
    auras.active.push(make_speed_aura(100, 1, -1.0));
    auras.active.push(make_speed_aura(200, 2, 0.50));
    assert_eq!(auras.movement_speed_multiplier(), 0.0);
}
