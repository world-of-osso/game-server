//! TC a352b1fa HandleAuraLinked: zero preserves child BP0; nonzero overrides it.
use super::test_harness::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use bevy::prelude::*;

const IRONFUR: u32 = 192_081;
const IRONFUR_MASTERY: u32 = 1_251_877;
const SPELLSHIELD: u32 = 56_071;
const SPELLSHIELD_CHILD: u32 = 66_668;
const URSOLS_WARDING: u32 = 471_492;
const URSOLS_CHILD: u32 = 471_491;
const WAKING_DREAM: u32 = 392_221;
const WAKING_CHILD: u32 = 392_223;
const STACKING_SHIELD: u32 = 96_493;
const STACKING_CHILD: u32 = 96_494;
const CHANNEL_LINK: u32 = 98_979;
const CHANNEL_CHILD: u32 = 98_973;

fn linked_harness() -> Harness {
    harness_with(
        Vec::new(),
        &[
            IRONFUR,
            IRONFUR_MASTERY,
            SPELLSHIELD,
            SPELLSHIELD_CHILD,
            URSOLS_WARDING,
            URSOLS_CHILD,
            WAKING_DREAM,
            WAKING_CHILD,
            STACKING_SHIELD,
            STACKING_CHILD,
            CHANNEL_LINK,
            CHANNEL_CHILD,
        ],
        11,
        true,
    )
    .unwrap()
}

fn apply_link(h: &mut Harness, spell_id: u32, caster: Entity, target: Entity) {
    let effect = world_db()
        .unwrap()
        .store
        .get(spell_id)
        .unwrap()
        .effects
        .iter()
        .find(|effect| matches!(effect.aura, 284 | 285))
        .unwrap();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask: 1 << effect.index,
            source: AuraSource::Cast,
        },
    );
}

fn child_aura(h: &Harness, target: Entity, spell: u32) -> crate::auras::AuraInstance {
    h.app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell)
        .cloned()
        .unwrap()
}

fn mastery(h: &Harness) -> f32 {
    h.app
        .world()
        .get::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .mastery_points
}

#[test]
fn class_completeness_native_linked_ironfur_applies_real_mastery_from_the_recipient() {
    let mut h = linked_harness();
    h.advance(3);
    let baseline = mastery(&h);
    let (caster, npc) = (h.caster, h.npc);
    apply_link(&mut h, IRONFUR, npc, caster);
    h.advance(3);
    let native = world_db()
        .unwrap()
        .store
        .get(IRONFUR_MASTERY)
        .unwrap()
        .effects[0]
        .base_points;
    assert_eq!(mastery(&h), baseline + native);
    assert_eq!(child_aura(&h, caster, IRONFUR_MASTERY).caster, caster);
}

#[test]
fn class_completeness_native_linked_ironfur_reapply_and_expiry_do_not_duplicate_mastery() {
    let mut h = linked_harness();
    h.advance(3);
    let baseline = mastery(&h);
    let caster = h.caster;
    apply_link(&mut h, IRONFUR, caster, caster);
    h.advance(3);
    let first = child_aura(&h, caster, IRONFUR_MASTERY);
    let boosted = mastery(&h);
    apply_link(&mut h, IRONFUR, caster, caster);
    h.advance(3);
    let refreshed = child_aura(&h, caster, IRONFUR_MASTERY);
    assert_eq!(refreshed.instance_id, first.instance_id);
    assert_eq!(refreshed.expires_at, first.expires_at);
    assert_eq!(mastery(&h), boosted);
    let duration = world_db()
        .unwrap()
        .store
        .get(IRONFUR)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .duration
        .unwrap()
        .duration_ms;
    h.advance(duration as usize / 50 + 4);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        caster,
        IRONFUR_MASTERY
    ));
    assert_eq!(mastery(&h), baseline);
}

#[test]
fn class_completeness_native_linked_explicit_child_uses_original_caster_and_preserves_foreign_replacement()
 {
    let mut h = linked_harness();
    let (caster, npc) = (h.caster, h.npc);
    apply_link(&mut h, SPELLSHIELD, caster, npc);
    assert_eq!(child_aura(&h, npc, SPELLSHIELD_CHILD).caster, caster);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: npc,
            target: npc,
            spell_id: SPELLSHIELD_CHILD,
            effect_mask: crate::auras::aura_effect_mask(
                world_db().unwrap().store.get(SPELLSHIELD_CHILD).unwrap(),
            ),
            source: AuraSource::Cast,
        },
    );
    crate::auras::remove_auras(h.app.world_mut(), npc, RemoveReason::Cancelled, |aura| {
        aura.spell_id == SPELLSHIELD && aura.caster == caster
    });
    assert_eq!(child_aura(&h, npc, SPELLSHIELD_CHILD).caster, npc);
}

#[test]
fn class_completeness_native_linked_reapply_synchronizes_native_stacks_without_restarting_child() {
    let mut h = linked_harness();
    let (caster, npc) = (h.caster, h.npc);
    apply_link(&mut h, STACKING_SHIELD, caster, npc);
    let first = child_aura(&h, npc, STACKING_CHILD);
    apply_link(&mut h, STACKING_SHIELD, caster, npc);
    let child = child_aura(&h, npc, STACKING_CHILD);
    assert_eq!(child.stacks, 2);
    assert_eq!(child.expires_at, first.expires_at);
    let native = world_db()
        .unwrap()
        .store
        .get(STACKING_CHILD)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.aura == 87)
        .unwrap()
        .base_points;
    let multiplier = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(npc)
        .unwrap()
        .damage_taken(1);
    assert!((multiplier - (1.0 + native * 2.0 / 100.0)).abs() < 0.0001);
}

#[test]
fn class_completeness_native_linked_channel_with_area_unit_child_uses_original_caster() {
    let mut h = linked_harness();
    let (caster, npc) = (h.caster, h.npc);
    apply_link(&mut h, CHANNEL_LINK, caster, npc);
    assert_eq!(child_aura(&h, caster, CHANNEL_CHILD).caster, caster);
}

#[test]
fn class_completeness_native_linked_child_respects_active_trigger_depth_limit() {
    let mut h = linked_harness();
    let caster = h.caster;
    super::cast_triggered(
        h.app.world_mut(),
        caster,
        IRONFUR,
        Some(caster),
        super::effects::MAX_TRIGGER_DEPTH,
    );
    assert!(crate::auras::has_aura(h.app.world(), caster, IRONFUR));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        caster,
        IRONFUR_MASTERY
    ));
}

#[test]
fn class_completeness_native_linked_nonzero_overrides_child_bp0_and_removes_owned_child() {
    let mut h = linked_harness();
    let caster = h.caster;
    // Native70448 parent BP10/8 overrides child BP-1/-1000, per TC5316–5320.
    for (parent, child, amount) in [
        (URSOLS_WARDING, URSOLS_CHILD, 10.0),
        (WAKING_DREAM, WAKING_CHILD, 8.0),
    ] {
        apply_link(&mut h, parent, caster, caster);
        let applied = child_aura(&h, caster, child);
        assert_eq!(applied.caster, caster);
        assert_eq!(applied.effects[0].amount, amount);
        crate::auras::remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
            aura.spell_id == parent
        });
        assert!(!crate::auras::has_aura(h.app.world(), caster, child));
    }
}
