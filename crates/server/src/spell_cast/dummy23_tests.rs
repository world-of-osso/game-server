//! Native Shadow Blades combo modifiers; its damage Dummy remains separate.
use super::test_harness::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn fixture() -> Harness {
    let ids = [121471, 53, 185438, 197835];
    let spells = ids
        .iter()
        .map(|id| {
            let mut spell = world_db().unwrap().store.get(*id).unwrap().clone();
            for effect in &mut spell.effects {
                effect.variance = 0.0;
            }
            spell
        })
        .collect();
    let mut h = harness(spells, &ids, 4).unwrap();
    h.set_power(PowerType::Energy, 100.0);
    h.set_power(PowerType::ComboPoints, 0.0);
    h
}

fn blades(h: &mut Harness) {
    let caster = h.caster;
    h.cast_and_deliver(121471, Some(caster));
    h.advance(35);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), caster, 121471));
    h.inbox.log.clear();
}

fn builder(h: &mut Harness, id: u32) {
    let npc = h.npc;
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(id, Some(npc));
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|event| event.spell_id == id));
}

#[test]
fn dummy23_shadow_blades_native_backstab_grants_double_combo_points() {
    let mut h = fixture();
    builder(&mut h, 53);
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
    h.advance(35);
    h.set_power(PowerType::ComboPoints, 0.0);
    blades(&mut h);
    builder(&mut h, 53);
    assert_eq!(h.power(PowerType::ComboPoints), 2.0);
    let grants: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(53) && e.kind == CombatLogKind::Energize)
        .map(|e| (e.amount, e.overflow))
        .collect();
    assert_eq!(grants, vec![(2, 0)]);
}

#[test]
fn dummy23_shadow_blades_native_backstab_cap_keeps_overflow() {
    let mut h = fixture();
    blades(&mut h);
    h.set_power(PowerType::ComboPoints, 4.0);
    builder(&mut h, 53);
    assert_eq!(h.power(PowerType::ComboPoints), 5.0);
    let grants: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(53) && e.kind == CombatLogKind::Energize)
        .map(|e| (e.amount, e.overflow))
        .collect();
    assert_eq!(grants, vec![(2, 1)]);
}

#[test]
fn dummy23_shadow_blades_native_expiry_restores_backstab() {
    let mut h = fixture();
    blades(&mut h);
    h.advance(500);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 121471));
    builder(&mut h, 53);
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
}

#[test]
fn dummy23_shadow_blades_native_storm_doubles_each_enemy_not_once() {
    let mut h = fixture();
    spawn_npc(&mut h.app, 3.0);
    blades(&mut h);
    builder(&mut h, 197835);
    assert_eq!(h.power(PowerType::ComboPoints), 4.0);
}

#[test]
fn dummy23_shadow_blades_storm_caps_the_native_pool() {
    let mut h = fixture();
    spawn_npc(&mut h.app, 3.0);
    blades(&mut h);
    h.set_power(PowerType::ComboPoints, 4.0);
    builder(&mut h, 197835);
    assert_eq!(h.power(PowerType::ComboPoints), 5.0);
}

#[test]
fn dummy23_storm_without_blades_retains_one_point_per_enemy() {
    let mut h = fixture();
    spawn_npc(&mut h.app, 3.0);
    builder(&mut h, 197835);
    assert_eq!(h.power(PowerType::ComboPoints), 2.0);
}

#[test]
fn dummy23_shadow_blades_expiry_restores_scripted_storm() {
    let mut h = fixture();
    spawn_npc(&mut h.app, 3.0);
    blades(&mut h);
    h.advance(500);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 121471));
    builder(&mut h, 197835);
    assert_eq!(h.power(PowerType::ComboPoints), 2.0);
}

#[test]
fn dummy23_shadow_blades_rejected_friendly_backstab_grants_nothing() {
    let mut h = fixture();
    blades(&mut h);
    let ally = spawn_player(&mut h.app, 4, 2.0);
    h.cast_and_deliver(53, Some(ally));
    h.advance(4);
    assert!(!h.inbox.failed.is_empty());
    assert_eq!(h.power(PowerType::ComboPoints), 0.0);
}
