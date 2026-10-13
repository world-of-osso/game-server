//! TC a352b1fa Spell.cpp5767 / Unit.cpp766: aura263 uses parent family flags.
use super::test_harness::*;
use shared::components::{Mana, PowerType};
use shared::protocol::CombatLogKind;
use shared::spell_data::CastFailReason;

const BLADESTORM: u32 = 227847;
const MORTAL_STRIKE: u32 = 12294;
const TAUNT: u32 = 355;
const FIREBALL: u32 = 133;
const ARCANE_EXPLOSION: u32 = 1449;

fn apply_restriction(h: &mut Harness) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: BLADESTORM,
            effect_mask: 1 << 2,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

#[test]
fn class_completeness_native_bladestorm_rejects_unmatched_taunt_before_payment() {
    let mut h = harness(Vec::new(), &[BLADESTORM, TAUNT], 1).unwrap();
    apply_restriction(&mut h);
    h.cast_and_deliver(TAUNT, Some(h.npc));
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::CantDoThatRightNow);
    assert!(h.inbox.go.is_empty());
    assert!(h.inbox.cooldowns.is_empty());
}

#[test]
fn class_completeness_native_bladestorm_blocks_foreign_class_fireball() {
    let mut h = harness(Vec::new(), &[BLADESTORM, FIREBALL], 8).unwrap();
    apply_restriction(&mut h);
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert!(h.inbox.go.is_empty());
}

#[test]
fn class_completeness_native_bladestorm_blocks_foreign_class_arcane_explosion() {
    let mut h = harness(Vec::new(), &[BLADESTORM, ARCANE_EXPLOSION], 8).unwrap();
    apply_restriction(&mut h);
    h.cast_and_deliver(ARCANE_EXPLOSION, Some(h.npc));
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert!(h.inbox.go.is_empty());
}

#[test]
fn class_completeness_native_bladestorm_parent_family_allows_real_mortal_strike() {
    let mut h = harness(Vec::new(), &[BLADESTORM, MORTAL_STRIKE], 1).unwrap();
    h.set_power(PowerType::Rage, 1000.0);
    apply_restriction(&mut h);
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(MORTAL_STRIKE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
}

fn remove_restriction(h: &mut Harness) {
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == BLADESTORM,
    );
}

#[test]
fn class_completeness_native_cast_restriction_removal_and_expiry_restore_casts() {
    for expire in [false, true] {
        let mut h = harness(Vec::new(), &[BLADESTORM, ARCANE_EXPLOSION], 8).unwrap();
        apply_restriction(&mut h);
        if expire {
            let duration = h
                .app
                .world()
                .get::<crate::auras::ActiveAuras>(h.caster)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == BLADESTORM)
                .unwrap()
                .duration_ms;
            assert!(duration > 0);
            h.advance(duration as usize / 50 + 2);
        } else {
            remove_restriction(&mut h);
        }
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, BLADESTORM));
        h.cast_and_deliver(ARCANE_EXPLOSION, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(
            h.inbox
                .log
                .iter()
                .any(|event| event.spell_id == Some(ARCANE_EXPLOSION)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0)
        );
    }
}

#[test]
fn class_completeness_native_restriction_gained_midcast_rejects_at_completion() {
    let mut h = harness(Vec::new(), &[BLADESTORM, FIREBALL], 8).unwrap();
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_some()
    );
    apply_restriction(&mut h);
    h.advance(60);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_none()
    );
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::CantDoThatRightNow);
    assert!(h.inbox.go.is_empty());
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().current,
        10_000.0
    );
    assert_eq!(h.health(h.npc), 5000.0);
}

#[test]
fn class_completeness_native_restriction_removed_midcast_allows_completion() {
    let mut h = harness(Vec::new(), &[BLADESTORM, FIREBALL], 8).unwrap();
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    apply_restriction(&mut h);
    remove_restriction(&mut h);
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
fn class_completeness_native_ranged_slot_and_explicit_attribute_exceptions() {
    for (class, spell, target_enemy) in [(3, 75, true), (8, 277757, false)] {
        let mut h = harness(Vec::new(), &[BLADESTORM, spell], class).unwrap();
        apply_restriction(&mut h);
        let target = if target_enemy { h.npc } else { h.caster };
        h.cast_and_deliver(spell, Some(target));
        h.advance(60);
        assert!(
            h.inbox.failed.is_empty(),
            "spell{spell}: {:?}",
            h.inbox.failed
        );
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
    }
}

#[test]
fn class_completeness_native_attack_effect_exception_retains_unbound_attack_action() {
    let mut h = harness(Vec::new(), &[BLADESTORM, 6603], 8).unwrap();
    apply_restriction(&mut h);
    h.cast_and_deliver(6603, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 6603));
    // Admission only: effect78 remains an explicit action-dispatch gap.
    assert_eq!(h.health(h.npc), 5000.0);
}

#[test]
fn class_completeness_native_family_exception_uses_caster_class_not_cast_family() {
    let data = world_db().unwrap();
    let mut spell = data.store.get(MORTAL_STRIKE).unwrap().clone();
    // Isolate the cast-family identity with the native Mage family, leaving flags unchanged.
    spell.class_options.as_mut().unwrap().spell_class_set = data
        .store
        .get(FIREBALL)
        .unwrap()
        .class_options
        .as_ref()
        .unwrap()
        .spell_class_set;
    let mut warrior = harness(vec![spell], &[BLADESTORM], 1).unwrap();
    warrior.set_power(PowerType::Rage, 1000.0);
    apply_restriction(&mut warrior);
    warrior.cast_and_deliver(MORTAL_STRIKE, Some(warrior.npc));
    assert!(
        warrior.inbox.failed.is_empty(),
        "{:?}",
        warrior.inbox.failed
    );
    assert!(
        warrior
            .inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(MORTAL_STRIKE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    let mut mage = harness(Vec::new(), &[BLADESTORM, MORTAL_STRIKE], 8).unwrap();
    apply_restriction(&mut mage);
    mage.cast_and_deliver(MORTAL_STRIKE, Some(mage.npc));
    assert_eq!(
        mage.inbox.failed[0].reason,
        CastFailReason::CantDoThatRightNow
    );
}

#[test]
fn class_completeness_native_cast_restriction_belongs_to_the_recipient() {
    let mut h = harness(Vec::new(), &[BLADESTORM, ARCANE_EXPLOSION], 8).unwrap();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.npc,
            spell_id: BLADESTORM,
            effect_mask: 1 << 2,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.cast_and_deliver(ARCANE_EXPLOSION, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(ARCANE_EXPLOSION)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
}

#[test]
fn class_completeness_native_triggered_damage_ignores_casting_disabled() {
    let mut h = harness(Vec::new(), &[BLADESTORM, ARCANE_EXPLOSION], 8).unwrap();
    apply_restriction(&mut h);
    super::cast_triggered(
        h.app.world_mut(),
        h.caster,
        ARCANE_EXPLOSION,
        Some(h.npc),
        1,
    );
    h.advance(2);
    assert!(h.inbox.failed.is_empty());
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(ARCANE_EXPLOSION)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().current,
        10_000.0
    );
}

#[test]
fn class_completeness_native_npc_cast_ignores_player_cast_restriction() {
    let mut h = harness(Vec::new(), &[BLADESTORM, ARCANE_EXPLOSION], 8).unwrap();
    h.app.world_mut().entity_mut(h.npc).insert((
        crate::stats::CharacterStats {
            spell_power: 100.0,
            base_mana: 10_000.0,
            ..Default::default()
        },
        Mana {
            current: 10_000.0,
            max: 10_000.0,
        },
    ));
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.npc,
            target: h.npc,
            spell_id: BLADESTORM,
            effect_mask: 1 << 2,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let spell = world_db()
        .unwrap()
        .store
        .get(ARCANE_EXPLOSION)
        .unwrap()
        .clone();
    super::execute_cast(h.app.world_mut(), h.npc, &spell, Some(h.caster)).unwrap();
    assert!(h.app.world().get::<Mana>(h.npc).unwrap().current < 10_000.0);
}
