//! Negative paths and lifetime boundaries using native Mainline children.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;

fn fixture(ids: &[u32], class: u8) -> Harness {
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
    harness(spells, &[], class).unwrap()
}
fn apply(h: &mut Harness, caster: Entity, target: Entity, id: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: id,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}
fn damage(h: &mut Harness, id: u32, target: Entity, amount: f32, periodic: bool) {
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .unwrap()
        .clone();
    let ctx = super::effects::EffectCtx {
        caster: h.caster,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::effects::deal_damage(h.app.world_mut(), &ctx, target, amount, false, periodic);
}
#[test]
fn dummy19_havoc_boundaries_no_double_source_or_recursive_copy() {
    for mode in 0..7 {
        let mut h = fixture(&[80240, 686, 42223, 45438, 17], 9);
        let caster = h.caster;
        let primary = h.npc;
        let marked = spawn_npc(&mut h.app, 4.0);
        let other = spawn_player(&mut h.app, 9, 2.0);
        h.advance(1);
        apply(
            &mut h,
            if mode == 1 { other } else { caster },
            marked,
            80240,
        );
        h.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .versatility_pct = 25.0;
        match mode {
            2 => apply(&mut h, marked, marked, 45438),
            3 => apply(&mut h, caster, primary, 17),
            6 => {
                h.app
                    .world_mut()
                    .get_mut::<Health>(primary)
                    .unwrap()
                    .current = 20.0;
            }
            _ => {}
        }
        damage(
            &mut h,
            if mode == 5 { 42223 } else { 686 },
            primary,
            100.0,
            mode == 4,
        );
        h.advance(1);
        let copies: Vec<_> = h
            .inbox
            .log
            .iter()
            .filter(|e| e.target == Some(marked.to_bits()) && e.kind == CombatLogKind::Damage)
            .collect();
        match mode {
            0 => {
                assert_eq!(copies.len(), 1);
                assert_eq!(copies[0].amount, 63);
            }
            6 => {
                assert_eq!(copies.len(), 1);
                assert_eq!(copies[0].amount, 10);
            }
            _ => assert!(copies.is_empty(), "mode {mode}"),
        }
    }
}
#[test]
fn dummy19_havoc_native_single_mark_preserves_foreign_owner_and_expires() {
    let mut h = fixture(&[80240, 686], 9);
    let caster = h.caster;
    let first = h.npc;
    let second = spawn_npc(&mut h.app, 4.0);
    let other = spawn_player(&mut h.app, 9, 2.0);
    h.advance(1);
    apply(&mut h, other, first, 80240);
    apply(&mut h, caster, first, 80240);
    apply(&mut h, caster, second, 80240);
    let marks = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(first)
        .unwrap();
    assert!(
        marks
            .iter()
            .any(|a| a.spell_id == 80240 && a.caster == other)
    );
    assert!(
        !marks
            .iter()
            .any(|a| a.spell_id == 80240 && a.caster == caster)
    );
    h.advance(320);
    assert!(!crate::auras::has_aura(h.app.world(), second, 80240));
    let before = h.health(second);
    damage(&mut h, 686, first, 100.0, false);
    assert_eq!(h.health(second), before);
}
#[test]
fn dummy19_dispersion_two_native_ranks_snapshot_and_cancel() {
    let mut h = fixture(&[47585, 288733, 453729], 5);
    let caster = h.caster;
    h.app.world_mut().get_mut::<Health>(caster).unwrap().current = 100.0;
    apply(&mut h, caster, caster, 288733);
    apply(&mut h, caster, caster, 453729);
    apply(&mut h, caster, caster, 47585);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 288733,
    );
    h.advance(5);
    assert_eq!(h.health(caster), 174.0);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 47585,
    );
    h.advance(180);
    assert_eq!(h.health(caster), 174.0);
}
#[test]
fn dummy19_vampiric_touch_expiry_and_cancel_never_horrify() {
    for reason in [
        crate::auras::RemoveReason::Expired,
        crate::auras::RemoveReason::Cancelled,
    ] {
        let mut h = fixture(&[34914, 87204], 5);
        let caster = h.caster;
        let target = h.npc;
        apply(&mut h, caster, target, 34914);
        crate::auras::remove_auras(h.app.world_mut(), target, reason, |a| a.spell_id == 34914);
        assert!(!crate::auras::has_aura(h.app.world(), caster, 87204));
        assert!(!crate::auras::has_aura(h.app.world(), target, 87204));
    }
}
#[test]
fn dummy19_ardent_defender_direct_damage_nonfatal_then_rescue_then_death() {
    let mut h = fixture(&[31850], 2);
    let caster = h.caster;
    let enemy = h.npc;
    apply(&mut h, caster, caster, 31850);
    crate::combat::deal_direct_damage(h.app.world_mut(), enemy, caster, 100.0);
    assert_eq!(h.health(caster), 900.0);
    assert!(crate::auras::has_aura(h.app.world(), caster, 31850));
    crate::combat::deal_direct_damage(h.app.world_mut(), enemy, caster, 1000.0);
    assert_eq!(h.health(caster), 200.0);
    assert!(!crate::auras::has_aura(h.app.world(), caster, 31850));
    crate::combat::deal_direct_damage(h.app.world_mut(), enemy, caster, 1000.0);
    assert_eq!(h.health(caster), 0.0);
}
#[test]
fn dummy19_ardent_defender_exclusive_commit_is_consumed_before_next_hit() {
    let mut h = fixture(&[31850], 2);
    let caster = h.caster;
    apply(&mut h, caster, caster, 31850);
    let before = h.health(caster);
    let first = crate::auras::apply_attack_damage(h.app.world_mut(), caster, before, 1);
    assert!(first.prevented);
    assert!(!first.lethal);
    assert_eq!(h.health(caster), 200.0);
    assert!(!crate::auras::has_aura(h.app.world(), caster, 31850));
    let second = crate::auras::apply_attack_damage(h.app.world_mut(), caster, 1000.0, 1);
    assert!(!second.prevented);
    assert!(second.lethal);
    assert_eq!(h.health(caster), 0.0);
}
