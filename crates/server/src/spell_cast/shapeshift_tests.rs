//! Native DB2 form gates; TC a352b1fa SpellInfo::CheckShapeshift / Spell::CheckCast.
use super::test_harness::*;
use shared::protocol::CombatLogKind;
use shared::spell_data::CastFailReason;

const CAT: u32 = 768;
const BEAR: u32 = 5487;
const SHRED: u32 = 5221;
const REGROWTH: u32 = 8936;
const REJUVENATION: u32 = 774;
const PREDATORY_SWIFTNESS: u32 = 69369;
const MOONKIN: u32 = 197625;
const MOONFIRE: u32 = 8921;
const WRATH: u32 = 5176;
const SKELETON: u32 = 52509;

fn druid() -> Harness {
    // Form-admission damage must hit; avoidance has separate native tests.
    harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[
            CAT,
            BEAR,
            SHRED,
            REGROWTH,
            REJUVENATION,
            PREDATORY_SWIFTNESS,
            MOONKIN,
            MOONFIRE,
            164812, // Native Moonfire's damage child.
            WRATH,
            SKELETON,
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
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::OnlyShapeshift);
    assert!(h.inbox.go.is_empty());
    assert!(h.inbox.cooldowns.is_empty());
}

#[test]
fn class_completeness_native_cat_rejects_excluded_rejuvenation() {
    let mut h = druid();
    apply_effects(&mut h, CAT, 1);
    h.cast_and_deliver(REJUVENATION, Some(h.caster));
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::NotShapeshift);
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

#[test]
fn class_completeness_native_required_cat_accepts_shred_but_bear_does_not() {
    let mut cat = druid();
    apply_effects(&mut cat, CAT, 1);
    cat.cast_and_deliver(SHRED, Some(cat.npc));
    assert!(cat.inbox.failed.is_empty(), "{:?}", cat.inbox.failed);
    assert!(
        cat.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(SHRED)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    let mut bear = druid();
    apply_effects(&mut bear, BEAR, 1);
    bear.cast_and_deliver(SHRED, Some(bear.npc));
    assert_eq!(bear.inbox.failed[0].reason, CastFailReason::OnlyShapeshift);
}

#[test]
fn class_completeness_native_ignore_form_mask_does_not_bypass_rejuvenation() {
    let mut h = druid();
    apply_effects(&mut h, CAT, 1);
    apply_effects(&mut h, PREDATORY_SWIFTNESS, 1 << 2);
    h.cast_and_deliver(REJUVENATION, Some(h.caster));
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::NotShapeshift);
}

#[test]
fn class_completeness_native_ignore_form_requires_the_same_spell_family() {
    let data = world_db().unwrap();
    let mut spell = data.store.get(REGROWTH).unwrap().clone();
    spell.class_options.as_mut().unwrap().spell_class_set = data
        .store
        .get(133)
        .unwrap()
        .class_options
        .as_ref()
        .unwrap()
        .spell_class_set;
    let mut h = harness(vec![spell], &[CAT, PREDATORY_SWIFTNESS], 11).unwrap();
    apply_effects(&mut h, CAT, 1);
    apply_effects(&mut h, PREDATORY_SWIFTNESS, 1 << 2);
    h.cast_and_deliver(REGROWTH, Some(h.caster));
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::NotShapeshift);
}

#[test]
fn class_completeness_native_ignore_form_removal_and_expiry_restore_rejection() {
    for expire in [false, true] {
        let mut h = druid();
        apply_effects(&mut h, CAT, 1);
        apply_effects(&mut h, PREDATORY_SWIFTNESS, 1 << 2);
        if expire {
            let remaining = h
                .app
                .world()
                .get::<crate::auras::ActiveAuras>(h.caster)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == PREDATORY_SWIFTNESS)
                .unwrap()
                .duration_ms;
            assert!(remaining > 0);
            h.advance(remaining as usize / 50 + 2);
        } else {
            crate::auras::remove_auras(
                h.app.world_mut(),
                h.caster,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == PREDATORY_SWIFTNESS,
            );
        }
        assert!(!crate::auras::has_aura(
            h.app.world(),
            h.caster,
            PREDATORY_SWIFTNESS
        ));
        h.cast_and_deliver(REGROWTH, Some(h.caster));
        assert_eq!(h.inbox.failed[0].reason, CastFailReason::NotShapeshift);
    }
}

#[test]
fn class_completeness_native_ignore_form_permission_belongs_to_recipient() {
    let mut h = druid();
    apply_effects(&mut h, CAT, 1);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.npc,
            spell_id: PREDATORY_SWIFTNESS,
            effect_mask: 1 << 2,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.cast_and_deliver(REGROWTH, Some(h.caster));
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::NotShapeshift);
}

#[test]
fn class_completeness_native_stance_flags_and_upper_form_masks() {
    let mut h = druid();
    // Native form35 is a Stance; Rejuvenation explicitly excludes high-word bit2.
    apply_effects(&mut h, MOONKIN, 1 << 1);
    h.cast_and_deliver(REJUVENATION, Some(h.caster));
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::NotShapeshift);
    h.inbox.failed.clear();
    h.cast_and_deliver(MOONFIRE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|event| event.spell_id == MOONFIRE));
    h.advance(35);
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

#[test]
fn class_completeness_native_caster_form_attribute_and_restricted_form_flag() {
    let mut caster = druid();
    // Wrath requires Moonkin but AllowCasterForm allows its native unshifted cast.
    caster.cast_and_deliver(WRATH, Some(caster.npc));
    assert!(caster.inbox.failed.is_empty(), "{:?}", caster.inbox.failed);
    caster.advance(80);
    assert!(
        caster
            .inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(WRATH)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    let mut restricted = druid();
    apply_effects(&mut restricted, SKELETON, 1);
    // Shred has no NotShapeshifted attribute: this rejection comes from form10 flags.
    restricted.cast_and_deliver(SHRED, Some(restricted.npc));
    assert_eq!(
        restricted.inbox.failed[0].reason,
        CastFailReason::NotShapeshift
    );
}
