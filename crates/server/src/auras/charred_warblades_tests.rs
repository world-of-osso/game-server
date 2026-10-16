//! TC a352b1fa spell_dh.cpp480-519: actual Fire damage accumulates a
//! native effect0 percentage for the next PERIODIC_DUMMY heal, then resets.
use super::*;
use crate::spell_cast::test_harness::{
    Harness, apply_native_enemy_dodge_reduction_to, drain,
    harness_with_native_enemy_dodge_reduction, spawn_peer_connection, spawn_player,
};
use shared::components::Health;
use shared::protocol::CombatLogKind;

const CHARRED: u32 = 213_010;
const HEAL: u32 = 213_011;
/// Native Chaos school includes Fire; native damage, coefficients and proc flags.
const CHAOS_HIT: u32 = 201_428;
const FIRE_HIT: u32 = 258_921;
const PHYSICAL_HIT: u32 = 12_294;
const SHIELD: u32 = 17;

fn fixture() -> Harness {
    // Damage/heal lifecycle, not avoidance: use the existing source-aura fixture.
    let mut h = harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[CHARRED, HEAL, CHAOS_HIT, FIRE_HIT, PHYSICAL_HIT, SHIELD],
        12,
    )
    .unwrap();
    *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
        current: 500.0,
        max: 5000.0,
    };
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: CHARRED,
            effect_mask: 1,
            source: AuraSource::KnownSpell,
        },
    );
    h
}

fn hit(h: &mut Harness, spell: u32) -> u32 {
    let before = h.health(h.npc);
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, spell, Some(h.npc), 1);
    h.advance(1);
    (before - h.health(h.npc)) as u32
}

fn expected_heal(h: &Harness, damage: u32) -> u32 {
    let percent = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == CHARRED)
        .unwrap()
        .effects[0]
        .amount;
    (damage as f32 * percent / 100.0) as u32
}

#[test]
fn class_completeness_native_charred_warblades_fixture_establishes_damage_before_heal_assertions() {
    let mut h = fixture();
    for attempt in 0..128 {
        h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 5000.0;
        h.inbox.log.clear();
        let damage = hit(&mut h, CHAOS_HIT);
        assert!(
            expected_heal(&h, damage) > 0,
            "native damage prerequisite failed at hit {attempt}: {:?}",
            h.inbox.log
        );
    }
}

#[test]
fn class_completeness_native_charred_warblades_accumulates_fire_damage_then_heals_once() {
    let mut h = fixture();
    let first = hit(&mut h, CHAOS_HIT);
    let second = hit(&mut h, FIRE_HIT);
    assert!(first > 0 && second > 0, "real damage precedes heal proof");
    let expected = expected_heal(&h, first) + expected_heal(&h, second);
    assert!(expected > 0);
    assert_eq!(
        h.health(h.caster),
        500.0,
        "heal waits for native 1000ms period"
    );
    h.advance(22);
    assert_eq!(
        h.health(h.caster),
        500.0 + expected as f32,
        "actual Fire damage must supply accumulated self-heal"
    );
    let count = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(HEAL))
        .count();
    assert_eq!(count, 1);
    h.advance(22);
    assert_eq!(
        h.health(h.caster),
        500.0 + expected as f32,
        "no damage means no repeated heal"
    );
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(HEAL))
            .count(),
        count
    );
}

#[test]
fn class_completeness_native_charred_warblades_isolates_two_recipients() {
    let mut h = fixture();
    let owner_damage = hit(&mut h, CHAOS_HIT);
    let expected_owner = expected_heal(&h, owner_damage);
    assert!(expected_owner > 0);
    let other = spawn_player(&mut h.app, 12, 2.0);
    apply_native_enemy_dodge_reduction_to(&mut h, other);
    let other_connection = spawn_peer_connection(&mut h.app, other, 2);
    h.app.world_mut().get_mut::<Health>(other).unwrap().current = 400.0;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: other,
            target: other,
            spell_id: CHARRED,
            effect_mask: 1,
            source: AuraSource::KnownSpell,
        },
    );
    let before = h.health(h.npc);
    crate::spell_cast::cast_triggered(h.app.world_mut(), other, CHAOS_HIT, Some(h.npc), 1);
    h.advance(1);
    let other_damage = (before - h.health(h.npc)) as u32;
    assert!(other_damage > 0);
    let expected_other = expected_heal(&h, other_damage);
    assert!(expected_other > 0);
    let mut other_logs = Vec::new();
    for _ in 0..22 {
        h.advance(1);
        other_logs.extend(drain::<shared::protocol::CombatLogEvent>(
            &mut h.app,
            other_connection,
        ));
    }
    assert_eq!(h.health(h.caster), 500.0 + expected_owner as f32);
    assert_eq!(h.health(other), 400.0 + expected_other as f32);
    let heals: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(HEAL))
        .collect();
    assert_eq!(heals.len(), 1);
    assert_eq!(heals[0].source, Some(h.caster.to_bits()));
    assert_eq!(heals[0].target, Some(h.caster.to_bits()));
    let other_heals: Vec<_> = other_logs
        .iter()
        .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(HEAL))
        .collect();
    assert_eq!(other_heals.len(), 1);
    assert_eq!(other_heals[0].source, Some(other.to_bits()));
    assert_eq!(other_heals[0].target, Some(other.to_bits()));
}

#[test]
fn class_completeness_native_charred_warblades_unlearning_discards_pending_heals() {
    let mut h = fixture();
    let damage = hit(&mut h, CHAOS_HIT);
    assert!(expected_heal(&h, damage) > 0);
    let owner = h.caster;
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            HEAL,
            FIRE_HIT,
            CHAOS_HIT,
            PHYSICAL_HIT,
            SHIELD,
        ]));
    h.advance(22);
    assert!(!has_aura(h.app.world(), owner, CHARRED));
    assert_eq!(h.health(owner), 500.0);
}

#[test]
fn class_completeness_native_charred_warblades_rejects_non_fire_and_foreign_actor_damage() {
    let mut h = fixture();
    assert!(hit(&mut h, PHYSICAL_HIT) > 0);
    let other = spawn_player(&mut h.app, 12, 2.0);
    let target_before = h.health(h.npc);
    crate::spell_cast::cast_triggered(h.app.world_mut(), other, FIRE_HIT, Some(h.npc), 1);
    h.advance(22);
    assert!(
        h.health(h.npc) < target_before,
        "foreign actor dealt real Fire damage"
    );
    assert_eq!(h.health(h.caster), 500.0);
    assert!(!h.inbox.log.iter().any(|event| event.spell_id == Some(HEAL)));
}

#[test]
fn class_completeness_native_charred_warblades_truncates_each_hit_before_accumulating() {
    let mut h = fixture();
    let mut expected = 0;
    for _ in 0..3 {
        let damage = hit(&mut h, FIRE_HIT);
        assert!(damage > 0);
        expected += expected_heal(&h, damage);
    }
    h.advance(22);
    assert_eq!(h.health(h.caster), 500.0 + expected as f32);
    let next = hit(&mut h, FIRE_HIT);
    let later = expected_heal(&h, next);
    h.advance(22);
    assert_eq!(h.health(h.caster), 500.0 + (expected + later) as f32);
}

#[test]
fn class_completeness_native_charred_warblades_removal_discards_pending_heals() {
    let mut h = fixture();
    let damage = hit(&mut h, CHAOS_HIT);
    assert!(expected_heal(&h, damage) > 0);
    let unit = h.caster;
    remove_auras(h.app.world_mut(), unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == CHARRED
    });
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: CHARRED,
            effect_mask: 1,
            source: AuraSource::KnownSpell,
        },
    );
    h.advance(22);
    assert_eq!(
        h.health(unit),
        500.0,
        "replacement instance cannot inherit stale accumulated damage"
    );
    let new_damage = hit(&mut h, CHAOS_HIT);
    let expected = expected_heal(&h, new_damage);
    assert!(expected > 0);
    h.advance(22);
    assert_eq!(h.health(unit), 500.0 + expected as f32);
}

#[test]
fn class_completeness_native_charred_warblades_refresh_keeps_pending_heals() {
    let mut h = fixture();
    let damage = hit(&mut h, CHAOS_HIT);
    let expected = expected_heal(&h, damage);
    assert!(expected > 0);
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: CHARRED,
            effect_mask: 1,
            source: AuraSource::KnownSpell,
        },
    );
    h.advance(22);
    assert_eq!(h.health(unit), 500.0 + expected as f32);
}

#[test]
fn class_completeness_native_charred_warblades_fully_absorbed_fire_does_not_heal() {
    let mut h = fixture();
    let source = h.caster;
    let target = h.npc;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: source,
            target,
            spell_id: SHIELD,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    let before = h.health(target);
    let damage = hit(&mut h, FIRE_HIT);
    assert_eq!(damage, 0, "real native barrier fully absorbs the hit");
    assert_eq!(h.health(target), before);
    h.advance(22);
    assert_eq!(h.health(source), 500.0);
    assert!(!h.inbox.log.iter().any(|event| event.spell_id == Some(HEAL)));
}
