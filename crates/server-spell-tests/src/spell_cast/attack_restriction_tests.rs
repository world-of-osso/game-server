//! Native aura264: TC Spell.cpp5774–5791 and Unit.cpp2259.
use super::test_harness::*;
use shared::protocol::CombatLogKind;
use shared::spell_data::CastFailReason;

const TURTLE: u32 = 186_265;
const TURTLE_PACIFY: u32 = 205_769;
const STEADY_SHOT: u32 = 56_641;
const PHALANX: u32 = 287_353;
const CHEETAH: u32 = 186_257;
const FIREBALL: u32 = 133;

#[test]
fn native_attack_restriction264_turtle_rejects_ranged_cast_before_pacify() {
    let mut h = harness(Vec::new(), &[TURTLE, TURTLE_PACIFY, STEADY_SHOT], 3)
        .expect("native Hunter spells");
    h.cast_and_deliver(TURTLE, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        TURTLE_PACIFY
    ));
    h.advance(31);
    h.inbox.go.clear();
    h.cast_and_deliver(STEADY_SHOT, Some(h.npc));
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::CantDoThatRightNow);
    assert!(h.inbox.go.is_empty());
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_none()
    );
}

#[test]
fn native_attack_restriction264_rejects_native_attack_predicates() {
    // Native weapon17/31/58/121, ATTACK78, next-swing and autoattack-enabling rows.
    // Admission only: this does not claim execution of otherwise unbound effects.
    for spell_id in [5019, 75, 2764, 6603, 27580, 96, 1464, 585] {
        let mut h = harness(Vec::new(), &[PHALANX, spell_id], 1).expect("native spell rows");
        super::cast_triggered(h.app.world_mut(), h.caster, PHALANX, Some(h.caster), 0);
        assert!(crate::auras::has_aura(h.app.world(), h.caster, PHALANX));
        h.cast_and_deliver(spell_id, Some(h.npc));
        assert_eq!(
            h.inbox.failed.len(),
            1,
            "spell{spell_id}: {:?}",
            h.inbox.failed
        );
        assert_eq!(
            h.inbox.failed[0].reason,
            CastFailReason::CantDoThatRightNow,
            "spell{spell_id}"
        );
        assert!(h.inbox.go.is_empty(), "spell{spell_id}");
    }
}

#[test]
fn native_attack_restriction264_allows_fireball_without_attack_flags() {
    let mut h = harness(Vec::new(), &[PHALANX, FIREBALL], 8).expect("native spells");
    super::cast_triggered(h.app.world_mut(), h.caster, PHALANX, Some(h.caster), 0);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, PHALANX));
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    h.advance(80);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(FIREBALL)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
}

#[test]
fn native_attack_restriction264_turtle_allows_real_cheetah_utility() {
    let mut h =
        harness(Vec::new(), &[TURTLE, TURTLE_PACIFY, CHEETAH], 3).expect("native Hunter spells");
    h.cast_and_deliver(TURTLE, Some(h.caster));
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        TURTLE_PACIFY
    ));
    h.advance(31);
    h.cast_and_deliver(CHEETAH, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, CHEETAH));
}

#[test]
fn native_attack_restriction264_gained_mid_steady_shot_rejects_completion() {
    let mut h = harness(Vec::new(), &[PHALANX, STEADY_SHOT], 3).expect("native spells");
    h.cast_and_deliver(STEADY_SHOT, Some(h.npc));
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_some()
    );
    super::cast_triggered(h.app.world_mut(), h.caster, PHALANX, Some(h.caster), 0);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, PHALANX));
    h.advance(45);
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::CantDoThatRightNow);
    assert!(h.inbox.go.is_empty());
    assert!(!h.inbox.log.iter().any(|event| event.spell_id == Some(STEADY_SHOT)
        && event.kind == CombatLogKind::Damage));
}

#[test]
fn native_attack_restriction264_expiry_restores_real_steady_shot() {
    let mut h = harness(Vec::new(), &[PHALANX, STEADY_SHOT], 3).expect("native spells");
    super::cast_triggered(h.app.world_mut(), h.caster, PHALANX, Some(h.caster), 0);
    let duration_ms = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == PHALANX)
        .unwrap()
        .duration_ms;
    assert!(duration_ms > 0);
    h.advance(duration_ms as usize / 50 + 2);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, PHALANX));
    h.cast_and_deliver(STEADY_SHOT, Some(h.npc));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|cast| cast.spell_id == STEADY_SHOT));
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(STEADY_SHOT)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
}

#[test]
fn native_attack_restriction264_full_trigger_still_lands_real_steady_shot() {
    let mut h = harness(Vec::new(), &[PHALANX, STEADY_SHOT], 3).expect("native spells");
    super::cast_triggered(h.app.world_mut(), h.caster, PHALANX, Some(h.caster), 0);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, PHALANX));
    super::cast_triggered(h.app.world_mut(), h.caster, STEADY_SHOT, Some(h.npc), 0);
    h.advance(30);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(STEADY_SHOT)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
}

#[test]
fn native_attack_restriction264_phalanx_stops_npc_swings_until_removed() {
    let mut h = harness(Vec::new(), &[PHALANX], 1).expect("native Phalanx");
    // A real native buff with264 but no25/60 pacify or263 cast restriction.
    super::cast_triggered(h.app.world_mut(), h.npc, PHALANX, Some(h.npc), 0);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, PHALANX));
    assert!(
        !h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .unwrap()
            .is_pacified()
    );
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::combat::AutoAttack::new(h.caster));
    h.advance(5);
    let npc_swings = |h: &Harness| {
        h.inbox
            .log
            .iter()
            .filter(|event| {
                event.source == Some(h.npc.to_bits())
                    && event.spell_id.is_none()
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
            })
            .count()
    };
    assert_eq!(npc_swings(&h), 0, "{:?}", h.inbox.log);
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.npc,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == PHALANX,
    );
    h.advance(45);
    assert!(
        npc_swings(&h) > 0,
        "normal swings resume after native aura removal"
    );
}
