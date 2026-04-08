use super::*;

#[test]
fn absorb_fully_absorbed() {
    let mut auras = Auras::default();
    auras
        .active
        .push(make_shield(100, 1, SpellSchool::Fire, 500.0));
    let leftover = auras.absorb_damage(300.0, SpellSchool::Fire);
    assert_eq!(leftover, 0.0);
    if let Some(AuraEffect::SchoolAbsorb { remaining, .. }) = auras.active[0].effects[0] {
        assert!((remaining - 200.0).abs() < 0.01);
    } else {
        panic!("shield effect missing");
    }
}

#[test]
fn absorb_partially_absorbed() {
    let mut auras = Auras::default();
    auras
        .active
        .push(make_shield(100, 1, SpellSchool::Fire, 200.0));
    let leftover = auras.absorb_damage(500.0, SpellSchool::Fire);
    assert!((leftover - 300.0).abs() < 0.01);
    assert!(auras.active.is_empty());
}

#[test]
fn absorb_wrong_school_ignored() {
    let mut auras = Auras::default();
    auras
        .active
        .push(make_shield(100, 1, SpellSchool::Fire, 500.0));
    let leftover = auras.absorb_damage(300.0, SpellSchool::Frost);
    assert_eq!(leftover, 300.0);
    assert_eq!(auras.active.len(), 1);
}

#[test]
fn absorb_weakest_first() {
    let mut auras = Auras::default();
    auras
        .active
        .push(make_shield(100, 1, SpellSchool::Shadow, 1000.0));
    auras
        .active
        .push(make_shield(200, 2, SpellSchool::Shadow, 200.0));

    let leftover = auras.absorb_damage(300.0, SpellSchool::Shadow);
    assert_eq!(leftover, 0.0);
    assert_eq!(auras.active.len(), 1, "weak shield should be depleted");
    assert_eq!(auras.active[0].spell_id, 100);
    if let Some(AuraEffect::SchoolAbsorb { remaining, .. }) = auras.active[0].effects[0] {
        assert!((remaining - 900.0).abs() < 0.01);
    } else {
        panic!("strong shield effect missing");
    }
}

#[test]
fn absorb_multiple_shields_chained() {
    let mut auras = Auras::default();
    auras
        .active
        .push(make_shield(100, 1, SpellSchool::Holy, 100.0));
    auras
        .active
        .push(make_shield(200, 2, SpellSchool::Holy, 150.0));

    let leftover = auras.absorb_damage(400.0, SpellSchool::Holy);
    assert!((leftover - 150.0).abs() < 0.01);
    assert!(auras.active.is_empty());
}

#[test]
fn absorb_zero_damage_returns_zero() {
    let mut auras = Auras::default();
    auras
        .active
        .push(make_shield(100, 1, SpellSchool::Fire, 500.0));
    let leftover = auras.absorb_damage(0.0, SpellSchool::Fire);
    assert_eq!(leftover, 0.0);
}

#[test]
fn absorb_preserves_non_absorb_auras() {
    let mut auras = Auras::default();
    let mut buff = make_aura(300, 1, 1);
    buff.effects[0] = Some(AuraEffect::ModDamageDone { percent: 0.10 });
    auras.apply(buff);
    auras
        .active
        .push(make_shield(100, 2, SpellSchool::Fire, 50.0));
    let leftover = auras.absorb_damage(50.0, SpellSchool::Fire);
    assert_eq!(leftover, 0.0);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].spell_id, 300);
}
