//! Native DB2 form gates; TC a352b1fa SpellInfo::CheckShapeshift / Spell::CheckCast.
use super::test_harness::*;
use shared::protocol::CombatLogKind;

const CAT: u32 = 768;
const BEAR: u32 = 5487;
const SHRED: u32 = 5221;
const REGROWTH: u32 = 8936;
const REJUVENATION: u32 = 774;
const PREDATORY_SWIFTNESS: u32 = 69369;

fn druid() -> Harness {
    harness(
        Vec::new(),
        &[
            CAT,
            BEAR,
            SHRED,
            REGROWTH,
            REJUVENATION,
            PREDATORY_SWIFTNESS,
        ],
        11,
    )
    .unwrap()
}

fn apply_effects(h: &mut Harness, spell_id: u32, effect_mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

#[test]
fn class_completeness_native_shred_requires_cat_before_payment() {
    let mut h = druid();
    h.cast_and_deliver(SHRED, Some(h.npc));
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert!(h.inbox.go.is_empty());
    assert!(h.inbox.cooldowns.is_empty());
}

#[test]
fn class_completeness_native_cat_rejects_excluded_rejuvenation() {
    let mut h = druid();
    apply_effects(&mut h, CAT, 1);
    h.cast_and_deliver(REJUVENATION, Some(h.caster));
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        REJUVENATION
    ));
}

#[test]
fn class_completeness_native_predatory_swiftness_bypasses_regrowth_form_gate() {
    let mut h = druid();
    apply_effects(&mut h, CAT, 1);
    h.cast_and_deliver(REGROWTH, Some(h.caster));
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    // Aura275 uses the family mask, not MiscValue10 as a spell ID.
    apply_effects(&mut h, PREDATORY_SWIFTNESS, 1 << 2);
    h.inbox.failed.clear();
    h.cast_and_deliver(REGROWTH, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(60);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(REGROWTH) && event.kind == CombatLogKind::Heal)
    );
}
