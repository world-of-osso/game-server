//! TC a352b1fa spell_druid.cpp271–295 native BeforeCast form transformers.
use super::test_harness::*;
use shared::components::{CombatStatus, Player, PowerType};

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
