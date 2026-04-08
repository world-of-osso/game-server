use super::*;

#[test]
fn apply_aura_first_time() {
    let mut auras = Auras::default();
    let result = auras.apply(make_aura(100, 1, 3));
    assert_eq!(result, AuraApplyResult::Applied);
    assert_eq!(auras.active.len(), 1);
}

#[test]
fn apply_aura_same_caster_refreshes_duration() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 3));
    auras.active[0].remaining = 3.0;

    let result = auras.apply(make_aura(100, 1, 3));
    assert_eq!(result, AuraApplyResult::Refreshed);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].remaining, 10.0, "duration should reset");
}

#[test]
fn apply_aura_same_caster_increments_stacks() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 5));
    assert_eq!(auras.active[0].stacks, 1);

    auras.apply(make_aura(100, 1, 5));
    assert_eq!(auras.active[0].stacks, 2);

    auras.apply(make_aura(100, 1, 5));
    assert_eq!(auras.active[0].stacks, 3);
}

#[test]
fn apply_aura_same_caster_caps_at_max_stacks() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 2));
    auras.apply(make_aura(100, 1, 2));
    auras.apply(make_aura(100, 1, 2));
    assert_eq!(auras.active[0].stacks, 2, "should not exceed max_stacks");
}

#[test]
fn apply_aura_different_caster_non_periodic_rejected() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    let result = auras.apply(make_aura(100, 2, 1));
    assert_eq!(result, AuraApplyResult::Rejected);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].caster, 1, "original caster kept");
}

#[test]
fn apply_dot_different_casters_stack() {
    let mut auras = Auras::default();
    auras.apply(make_dot(200, 1));
    let result = auras.apply(make_dot(200, 2));
    assert_eq!(result, AuraApplyResult::Applied);
    assert_eq!(auras.active.len(), 2, "DOTs from different casters stack");
}

#[test]
fn apply_dot_same_caster_refreshes() {
    let mut auras = Auras::default();
    auras.apply(make_dot(200, 1));
    auras.active[0].remaining = 5.0;

    let result = auras.apply(make_dot(200, 1));
    assert_eq!(result, AuraApplyResult::Refreshed);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].remaining, 15.0);
}

#[test]
fn apply_different_spells_always_stack() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    let result = auras.apply(make_aura(200, 1, 1));
    assert_eq!(result, AuraApplyResult::Applied);
    assert_eq!(auras.active.len(), 2);
}

#[test]
fn tick_expires_aura_at_zero() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    let expired = auras.tick_and_expire(10.1);
    assert_eq!(expired, 1);
    assert!(auras.active.is_empty());
}

#[test]
fn tick_does_not_expire_with_time_left() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    let expired = auras.tick_and_expire(5.0);
    assert_eq!(expired, 0);
    assert_eq!(auras.active.len(), 1);
    assert!((auras.active[0].remaining - 5.0).abs() < 0.01);
}

#[test]
fn tick_expires_some_keeps_others() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    auras.apply(make_dot(200, 2));
    let expired = auras.tick_and_expire(12.0);
    assert_eq!(expired, 1);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].spell_id, 200, "DOT should survive");
}

#[test]
fn dispel_removes_all_instances_of_spell() {
    let mut auras = Auras::default();
    auras.apply(make_dot(200, 1));
    auras.apply(make_dot(200, 2));
    auras.apply(make_aura(300, 1, 1));
    let removed = auras.dispel(200);
    assert_eq!(removed, 2);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].spell_id, 300);
}

#[test]
fn dispel_nonexistent_spell_removes_nothing() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    let removed = auras.dispel(999);
    assert_eq!(removed, 0);
    assert_eq!(auras.active.len(), 1);
}

#[test]
fn cancel_removes_specific_caster_aura() {
    let mut auras = Auras::default();
    auras.apply(make_dot(200, 1));
    auras.apply(make_dot(200, 2));
    let removed = auras.cancel(200, 1);
    assert!(removed);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].caster, 2, "other caster's DOT kept");
}

#[test]
fn cancel_wrong_caster_returns_false() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    let removed = auras.cancel(100, 999);
    assert!(!removed);
    assert_eq!(auras.active.len(), 1);
}

#[test]
fn clear_all_removes_everything() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    auras.apply(make_dot(200, 2));
    auras.apply(make_aura(300, 3, 5));
    auras.clear_all();
    assert!(auras.active.is_empty());
}
