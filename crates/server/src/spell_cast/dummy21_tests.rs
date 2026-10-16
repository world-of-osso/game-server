//! Local Mainline106951s2: one additional point from Feral combo builders.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn fixture() -> Harness {
    let ids = [
        106951, 343216, 768, 5221, 1822, 155722, 106830, 405233, 193315,
    ];
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
    let mut h = harness_with_native_enemy_dodge_reduction(spells, &ids, 11).unwrap();
    let caster = h.caster;
    h.cast_and_deliver(768, Some(caster));
    h.advance(35);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.set_power(PowerType::Energy, 100.0);
    h.set_power(PowerType::ComboPoints, 0.0);
    h
}

fn berserk(h: &mut Harness) {
    let caster = h.caster;
    h.cast_and_deliver(106951, Some(caster));
    h.advance(35);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), caster, 106951));
    // The native1500ms timer has already paid one independent343216 point.
    h.set_power(PowerType::ComboPoints, 0.0);
    h.inbox.log.clear();
}

fn builder(h: &mut Harness, id: u32) {
    h.set_power(PowerType::Energy, 100.0);
    let npc = h.npc;
    h.cast_and_deliver(id, Some(npc));
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == id));
}

fn energize(h: &Harness, id: u32) -> Vec<(i32, i32)> {
    h.inbox
        .log
        .iter()
        .filter(|event| event.spell_id == Some(id) && event.kind == CombatLogKind::Energize)
        .map(|event| (event.amount, event.overflow))
        .collect()
}

fn extra_point(id: u32) {
    let mut h = fixture();
    builder(&mut h, id);
    assert_eq!(h.power(PowerType::ComboPoints), 1.0, "ordinary builder");
    h.advance(35);
    h.set_power(PowerType::ComboPoints, 0.0);
    h.inbox.log.clear();
    berserk(&mut h);
    builder(&mut h, id);
    assert_eq!(
        h.power(PowerType::ComboPoints),
        2.0,
        "native1 plus106951s2=1"
    );
    assert_eq!(energize(&h, id), vec![(2, 0)]);
}

#[test]
fn dummy21_berserk_completed_root_adds_one_to_shred() {
    extra_point(5221);
}

#[test]
fn dummy21_berserk_completed_root_adds_one_to_rake() {
    extra_point(1822);
}

#[test]
fn dummy21_berserk_thrash_bonus_is_once_not_per_area_target() {
    let mut h = fixture();
    spawn_npc(&mut h.app, 4.0);
    spawn_npc(&mut h.app, 5.0);
    berserk(&mut h);
    builder(&mut h, 106830);
    assert_eq!(h.power(PowerType::ComboPoints), 2.0);
    assert_eq!(energize(&h, 106830), vec![(2, 0)]);
}

#[test]
fn dummy21_berserk_bonus_clamps_at_native_combo_cap_and_logs_excess() {
    let mut h = fixture();
    berserk(&mut h);
    h.set_power(PowerType::ComboPoints, 4.0);
    builder(&mut h, 5221);
    assert_eq!(h.power(PowerType::ComboPoints), 5.0);
    assert_eq!(energize(&h, 5221), vec![(2, 1)]);
}

#[test]
fn dummy21_berserk_expiry_restores_ordinary_builder() {
    let mut h = fixture();
    berserk(&mut h);
    h.advance(450);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 106951));
    h.set_power(PowerType::ComboPoints, 0.0);
    h.inbox.log.clear();
    builder(&mut h, 5221);
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
    assert_eq!(energize(&h, 5221), vec![(1, 0)]);
}

#[test]
fn dummy21_berserk_named_periodic_child_does_not_gain_builder_bonus() {
    let mut h = fixture();
    berserk(&mut h);
    h.advance(35);
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
    assert_eq!(energize(&h, 343216), vec![(1, 0)]);
}

#[test]
fn dummy21_berserk_does_not_empower_another_family_generator() {
    let mut h = fixture();
    berserk(&mut h);
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(193315)
        .unwrap()
        .clone();
    super::execute_cast(h.app.world_mut(), h.caster, &spell, Some(h.npc)).unwrap();
    h.advance(4);
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
    assert_eq!(energize(&h, 193315), vec![(1, 0)]);
}

#[test]
fn dummy21_berserk_rejected_friendly_shred_grants_no_points() {
    let mut h = fixture();
    berserk(&mut h);
    let friend = spawn_player(&mut h.app, 11, 3.0);
    h.cast_and_deliver(5221, Some(friend));
    h.advance(4);
    assert!(!h.inbox.failed.is_empty());
    assert_eq!(h.power(PowerType::ComboPoints), 0.0);
    assert!(energize(&h, 5221).is_empty());
}
