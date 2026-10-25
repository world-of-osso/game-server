//! Native Attr3 ALWAYS_HIT0x00040000: TC a352b1fa Object.cpp1947–1995.
//! Real self-cast5277 supplies200 DodgePercent; no rows, chances or RNG are edited.
use super::{
    SHIELD_CHARGE, SHIELD_CHARGE_DAMAGE, SHIELD_SLAM, apply_passive, charge_fixture, fixture,
    spawn_npc,
};
use shared::protocol::{CombatLogKind, MissKind};

const EVASION: u32 = 5277;
const DIVINE_SHIELD: u32 = 642;
const SPELL_REFLECTION: u32 = 23920;
const UNHOLY_CURSE: u32 = 20006;

#[test]
fn census_always_hit_native_charge_chain_bypasses_evasion_but_plain_slam_does_not() {
    let (mut h, target, near, _) = charge_fixture();
    let control = spawn_npc(&mut h.app, 3.0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            SHIELD_SLAM,
            SHIELD_CHARGE,
        ]));
    for unit in [control, target, near] {
        crate::spell_cast::creature_cast(h.app.world_mut(), unit, EVASION, Some(unit), false)
            .unwrap();
        assert_eq!(
            h.app
                .world()
                .get::<crate::auras::ActiveAuras>(unit)
                .unwrap()
                .modifier(49),
            200.0
        );
    }
    h.cast_and_deliver(SHIELD_SLAM, Some(control));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(SHIELD_SLAM)
                && matches!(
                    event.kind,
                    CombatLogKind::Miss(MissKind::Dodge | MissKind::Miss)
                ))
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(SHIELD_SLAM)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    h.advance(40);
    h.inbox = Default::default();
    h.cast_and_deliver(SHIELD_CHARGE, Some(target));
    h.advance(20);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<shared::components::Position>(h.caster)
            .unwrap()
            .x
            > 5.0,
        "native385952/385953 ALWAYS_HIT must reach movement despite real200% Evasion; log={:?}",
        h.inbox.log
    );
    for recipient in [target, near] {
        assert!(
            h.inbox
                .log
                .iter()
                .any(|event| event.spell_id == Some(SHIELD_CHARGE_DAMAGE)
                    && event.target == Some(recipient.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "native385954 ALWAYS_HIT cannot be avoided; log={:?}",
            h.inbox.log
        );
    }
    assert!(crate::auras::has_aura(
        h.app.world(),
        target,
        SHIELD_CHARGE_DAMAGE
    ));
}

#[test]
fn census_always_hit_charge_landing_keeps_native_school_immunity() {
    let (mut h, target, near, _) = charge_fixture();
    // Actual642 effect0 school-immunity127; qualify the defense input, not spell eligibility.
    apply_passive(&mut h, target, DIVINE_SHIELD);
    apply_passive(&mut h, near, DIVINE_SHIELD);
    // Landing's area is caster-centered; both immune recipients must be in range.
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.caster)
        .unwrap()
        .x = 8.0;
    h.inbox = Default::default();
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        SHIELD_CHARGE_DAMAGE,
        Some(target),
        1
    ));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    for recipient in [target, near] {
        assert!(
            h.inbox
                .log
                .iter()
                .any(|event| event.spell_id == Some(SHIELD_CHARGE_DAMAGE)
                    && event.target == Some(recipient.to_bits())
                    && event.kind == CombatLogKind::Miss(MissKind::Immune)),
            "log={:?}",
            h.inbox.log
        );
        assert!(
            !h.inbox
                .log
                .iter()
                .any(|event| event.spell_id == Some(SHIELD_CHARGE_DAMAGE)
                    && event.target == Some(recipient.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0)
        );
    }
}

#[test]
fn census_always_hit_magic_spell_still_reflects() {
    let mut h = fixture(false);
    let target = h.npc;
    // Actual23920 effect0 supplies100% reflect; native20006 is reflectable Magic+ALWAYS_HIT.
    apply_passive(&mut h, target, SPELL_REFLECTION);
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        UNHOLY_CURSE,
        Some(target),
        1
    ));
    h.advance(5);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(UNHOLY_CURSE)
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Miss(MissKind::Reflect)),
        "ALWAYS_HIT is checked after reflection; log={:?}",
        h.inbox.log
    );
    assert!(!crate::auras::has_aura(h.app.world(), target, UNHOLY_CURSE));
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        UNHOLY_CURSE
    ));
}
