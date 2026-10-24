//! TC a352b1fa Unit.cpp2647–2717/2750–2785: NPC dodge includes rear attacks.
use super::test_harness::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use bevy::prelude::*;
use shared::protocol::{CombatLogKind, MissKind};

const STRIKE: u32 = 38598;
const CLOAK_OF_DARKNESS: u32 = 70188;

#[test]
fn class_completeness_native_npc_dodge_defends_front_and_rear_weapon_spells() {
    // Target at +X faces the caster with pi, or away with zero.
    for facing in [std::f32::consts::PI, 0.0] {
        let mut h = spawn_native_dodge_target(40, facing);
        let before = h.health(h.npc);
        h.inbox.log.clear();
        let (caster, target) = (h.caster, h.npc);
        super::creature_cast(h.app.world_mut(), caster, STRIKE, Some(target), true).unwrap();
        h.advance(2);
        assert_eq!(
            h.health(target),
            before,
            "native NPC dodge admitted damage: {:?}",
            h.inbox.log
        );
        assert!(
            h.inbox.log.iter().any(|event| {
                event.spell_id == Some(STRIKE)
                    && event.target == Some(target.to_bits())
                    && event.kind == CombatLogKind::Miss(MissKind::Dodge)
            }),
            "native owner received no dodge: {:?}",
            h.inbox.log
        );
    }
}

fn assert_native_damage(h: &mut Harness, spell: u32, frames: usize) {
    let before = h.health(h.npc);
    h.inbox.log.clear();
    let (caster, target) = (h.caster, h.npc);
    super::creature_cast(h.app.world_mut(), caster, spell, Some(target), true).unwrap();
    h.advance(frames);
    assert!(
        h.health(target) < before,
        "native guard admitted no damage: {:?}",
        h.inbox.log
    );
    assert!(
        h.inbox.log.iter().any(|event| {
            event.spell_id == Some(spell)
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "actual damage receipt missing: {:?}",
        h.inbox.log
    );
    assert!(!h.inbox.log.iter().any(|event| {
        event.spell_id == Some(spell)
            && event.target == Some(target.to_bits())
            && event.kind == CombatLogKind::Miss(MissKind::Dodge)
    }));
}

#[test]
fn class_completeness_native_npc_dodge_removal_restores_possible_weapon_damage() {
    let mut h = spawn_native_dodge_target(40, 0.0);
    let target = h.npc;
    crate::auras::remove_auras(h.app.world_mut(), target, RemoveReason::Cancelled, |aura| {
        aura.spell_id == CLOAK_OF_DARKNESS
    });
    h.advance(2);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        target,
        CLOAK_OF_DARKNESS
    ));
    for _ in 0..128 {
        let before = h.health(target);
        h.inbox.log.clear();
        let caster = h.caster;
        super::creature_cast(h.app.world_mut(), caster, STRIKE, Some(target), true).unwrap();
        h.advance(2);
        if h.inbox.log.iter().any(|event| {
            event.spell_id == Some(STRIKE)
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }) {
            assert!(h.health(target) < before);
            return;
        }
        // Native baseline dodge remains legal after removing the100% aura.
        assert_eq!(h.health(target), before);
        assert!(
            h.inbox.log.iter().any(|event| {
                event.spell_id == Some(STRIKE)
                    && event.target == Some(target.to_bits())
                    && event.kind == CombatLogKind::Miss(MissKind::Dodge)
            }),
            "expected actual baseline dodge while awaiting damage: {:?}",
            h.inbox.log
        );
    }
    panic!("native Strike never dealt damage after dodge aura removal");
}

#[test]
fn class_completeness_native_npc_dodge_stun_retains_native_exclusion() {
    let mut h = spawn_native_dodge_target(40, std::f32::consts::PI);
    let (caster, target) = (h.caster, h.npc);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id: 853,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(target)
            .unwrap()
            .is_stunned()
    );
    assert_native_damage(&mut h, STRIKE, 2);
    assert!(crate::auras::has_aura(
        h.app.world(),
        target,
        CLOAK_OF_DARKNESS
    ));
}

#[test]
fn class_completeness_native_npc_dodge_attributes_and_ranged_retain_exclusions() {
    for (spell, frames) in [(7384, 3), (51723, 3), (19434, 65)] {
        let mut h = spawn_native_dodge_target(40, 0.0);
        assert_native_damage(&mut h, spell, frames);
        assert!(
            crate::auras::has_aura(h.app.world(), h.npc, CLOAK_OF_DARKNESS),
            "cloak absent after native spell{spell}, health{}, receipts{:?}",
            h.health(h.npc),
            h.inbox.log
        );
    }
}
