//! TC a352b1fa Spell.cpp5767 / Unit.cpp766: aura263 uses parent family flags.
use super::test_harness::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

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
