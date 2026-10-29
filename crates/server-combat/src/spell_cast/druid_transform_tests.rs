//! TC a352b1fa spell_druid.cpp271–295 native BeforeCast form transformers.
use super::test_harness::*;
use shared::components::{CombatStatus, Health, Player, PowerType};
use shared::protocol::CombatLogKind;

const CAT: u32 = 768;
const BEAR: u32 = 5487;
const PROWL: u32 = 5215;
const STAMPEDING_ROAR: u32 = 106898;

fn druid() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[
            CAT,
            BEAR,
            PROWL,
            STAMPEDING_ROAR,
            99,
            50334,
            252216,
            33917,
            77758,
            6795,
        ],
        11,
    )
    .unwrap();
    h.app.world_mut().get_mut::<Player>(h.caster).unwrap().race = 4;
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(false));
    h
}

fn form(h: &Harness) -> Option<i32> {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .and_then(crate::auras::shapeshift::current_form)
        .map(|(_, form)| form)
}

fn cast_form(h: &mut Harness, id: u32) {
    h.cast_and_deliver(id, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(35);
    h.inbox = Inbox::default();
}

#[test]
fn class_completeness_native_druid_other_transformers_cast_their_authored_form() {
    for (id, expected) in [(99, 5), (50334, 5), (252216, 1)] {
        let mut h = druid();
        h.cast_and_deliver(id, None);
        assert!(h.inbox.failed.is_empty(), "{id}: {:?}", h.inbox.failed);
        assert_eq!(form(&h), Some(expected), "native root {id}");
        assert!(h.inbox.log.iter().any(|event| event.spell_id == Some(id)
            && event.kind == CombatLogKind::CastSuccess));
    }
}

#[test]
fn class_completeness_native_druid_triggered_tiger_dash_replaces_bear_with_cat() {
    let mut h = druid();
    cast_form(&mut h, BEAR);
    assert_eq!(form(&h), Some(5));
    let caster = h.caster;
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        caster,
        252216,
        Some(caster),
        1
    ));
    h.advance(3);
    assert_eq!(form(&h), Some(1));
    assert!(!crate::auras::has_aura(h.app.world(), caster, BEAR));
    assert!(crate::auras::has_aura(h.app.world(), caster, CAT));
    assert!(crate::auras::has_aura(h.app.world(), caster, 252216));
}

#[test]
fn class_completeness_native_druid_same_form_does_not_recast_or_refresh_it() {
    for (parent, child) in [(PROWL, CAT), (STAMPEDING_ROAR, BEAR)] {
        let mut h = druid();
        cast_form(&mut h, child);
        h.cast_and_deliver(parent, Some(h.caster));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(
            !h.inbox.log.iter().any(|event| event.spell_id == Some(child)
                && matches!(
                    event.kind,
                    CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                )),
            "same-form parent {parent} recast child: {:?}",
            h.inbox.log
        );
    }
}

#[test]
fn class_completeness_native_druid_bear_transformer_replaces_cat_not_preserves_it() {
    let mut h = druid();
    cast_form(&mut h, CAT);
    h.cast_and_deliver(STAMPEDING_ROAR, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(form(&h), Some(5));
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, CAT));
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .and_then(crate::auras::shapeshift::current_form)
            .is_none(),
        "automatic form belongs only to the caster"
    );
}

#[test]
fn class_completeness_native_druid_rejected_roar_does_not_transform_the_caster() {
    let mut h = druid();
    h.cast_and_deliver(CAT, Some(h.caster));
    h.inbox = Inbox::default();
    h.cast_and_deliver(STAMPEDING_ROAR, Some(h.caster));
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::OnGlobalCooldown
    );
    assert_eq!(form(&h), Some(1));
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, BEAR));
    assert!(h.inbox.go.is_empty());
}

#[test]
fn class_completeness_native_druid_missing_child_refuses_instead_of_fake_transform_success() {
    let mut h = harness(Vec::new(), &[PROWL], 11).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(false));
    h.cast_and_deliver(PROWL, Some(h.caster));
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::NotSupported
    );
    assert_eq!(form(&h), None);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, PROWL));
    assert!(h.inbox.go.is_empty());
}

#[test]
fn class_completeness_native_berserk_resets_real_thrash_and_growl_cast_cooldowns() {
    for id in [77758, 6795] {
        let mut h = druid();
        h.app.world_mut().entity_mut(h.npc).insert(Health {
            current: 1_000_000.0,
            max: 1_000_000.0,
        });
        cast_form(&mut h, BEAR);
        h.cast_and_deliver(id, Some(h.npc));
        assert!(
            h.inbox.failed.is_empty(),
            "initial {id}: {:?}",
            h.inbox.failed
        );
        assert!(h.inbox.go.iter().any(|go| go.spell_id == id));
        h.advance(35);
        h.inbox = Inbox::default();
        h.cast_and_deliver(50334, Some(h.caster));
        assert!(
            h.inbox.failed.is_empty(),
            "Berserk {id}: {:?}",
            h.inbox.failed
        );
        h.advance(35);
        h.inbox = Inbox::default();
        h.cast_and_deliver(id, Some(h.npc));
        assert!(
            h.inbox.failed.is_empty(),
            "native Berserk must reset {id}: {:?}",
            h.inbox.failed
        );
        assert!(h.inbox.go.iter().any(|go| go.spell_id == id));
    }
}

#[test]
fn class_completeness_native_druid_prowl_transforms_through_its_cat_child() {
    let mut h = druid();
    assert_eq!(form(&h), None);
    h.cast_and_deliver(PROWL, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        form(&h),
        Some(1),
        "native BeforeCast must supply Cat, not just Stealth"
    );
    assert!(crate::auras::has_aura(h.app.world(), h.caster, CAT));
    assert!(crate::auras::has_aura(h.app.world(), h.caster, PROWL));
    assert_eq!(
        h.app
            .world()
            .get::<super::powers::PowerPools>(h.caster)
            .unwrap()
            .order
            .first(),
        Some(&PowerType::Energy)
    );
}

#[test]
fn class_completeness_native_druid_stampeding_roar_transforms_through_its_bear_child() {
    let mut h = druid();
    assert_eq!(form(&h), None);
    h.cast_and_deliver(STAMPEDING_ROAR, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        form(&h),
        Some(5),
        "native BeforeCast must supply Bear, not just speed"
    );
    assert!(crate::auras::has_aura(h.app.world(), h.caster, BEAR));
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        STAMPEDING_ROAR
    ));
    assert_eq!(
        h.app
            .world()
            .get::<super::powers::PowerPools>(h.caster)
            .unwrap()
            .order
            .first(),
        Some(&PowerType::Rage)
    );
}

#[test]
fn class_completeness_native_berserk_triggered_reset_preserves_current_gcd() {
    let mut h = druid();
    cast_form(&mut h, BEAR);
    h.cast_and_deliver(77758, Some(h.npc));
    assert!(h.inbox.failed.is_empty());
    let caster = h.caster;
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        caster,
        50334,
        Some(caster),
        1
    ));
    h.inbox = Inbox::default();
    h.cast_and_deliver(77758, Some(h.npc));
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::OnGlobalCooldown
    );
    h.advance(35);
    h.inbox = Inbox::default();
    h.cast_and_deliver(77758, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 77758));
}

#[test]
fn class_completeness_native_berserk_does_not_restore_mangle_charges() {
    let mut h = druid();
    cast_form(&mut h, BEAR);
    h.cast_and_deliver(33917, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(35);
    h.inbox = Inbox::default();
    h.cast_and_deliver(50334, Some(h.caster));
    assert!(h.inbox.failed.is_empty());
    h.inbox = Inbox::default();
    h.cast_and_deliver(33917, Some(h.npc));
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::NoChargesRemain
    );
    h.advance(70);
    h.inbox = Inbox::default();
    h.cast_and_deliver(33917, Some(h.npc));
    assert!(
        h.inbox.failed.is_empty(),
        "native charge recovery: {:?}",
        h.inbox.failed
    );
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 33917));
}

#[test]
fn class_completeness_native_berserk_preserves_unrelated_roar_cooldown() {
    let mut h = druid();
    h.cast_and_deliver(STAMPEDING_ROAR, Some(h.caster));
    assert!(h.inbox.failed.is_empty());
    h.advance(35);
    h.inbox = Inbox::default();
    h.cast_and_deliver(50334, Some(h.caster));
    assert!(h.inbox.failed.is_empty());
    h.advance(35);
    h.inbox = Inbox::default();
    h.cast_and_deliver(STAMPEDING_ROAR, Some(h.caster));
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::OnCooldown
    );
}

#[test]
fn class_completeness_native_failed_berserk_does_not_reset_thrash() {
    let mut h = druid();
    cast_form(&mut h, BEAR);
    h.cast_and_deliver(50334, Some(h.caster));
    assert!(h.inbox.failed.is_empty());
    h.cast_and_deliver(77758, Some(h.npc));
    assert!(h.inbox.failed.is_empty());
    h.advance(35);
    h.inbox = Inbox::default();
    h.cast_and_deliver(50334, Some(h.caster));
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::OnCooldown
    );
    h.inbox = Inbox::default();
    h.cast_and_deliver(77758, Some(h.npc));
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::OnCooldown
    );
}

#[test]
fn class_completeness_native_berserk_does_not_reset_another_druids_growl() {
    let mut h = druid();
    let other = spawn_player(&mut h.app, 11, 0.0);
    h.advance(3);
    assert!(super::cast_granted(h.app.world_mut(), other, BEAR, Some(other)).is_ok());
    h.advance(35);
    let npc = h.npc;
    assert!(super::cast_granted(h.app.world_mut(), other, 6795, Some(npc)).is_ok());
    h.inbox = Inbox::default();
    h.cast_and_deliver(50334, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(35);
    assert!(matches!(
        super::cast_granted(h.app.world_mut(), other, 6795, Some(npc)),
        Err(super::CastFailure::OnCooldown)
    ));
}
