//! Controlled real-native child casts isolate TC effect1 effectiveness, not acquisition/DPS.
use super::test_harness::{Harness, harness};
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

const MASTERY: u32 = 168534;
const ECHO_CHAMBER: u32 = 382032;

fn cast_native_child(h: &mut Harness, child: u32) -> shared::protocol::CombatLogEvent {
    let first = h.inbox.log.len();
    super::creature_cast(h.app.world_mut(), h.caster, child, Some(h.npc), false).unwrap();
    for _ in 0..200 {
        h.advance(1);
        if let Some(event) = h.inbox.log[first..].iter().find(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(h.caster.to_bits())
                && event.target == Some(h.npc.to_bits())
                && event.spell_id == Some(child)
        }) {
            return event.clone();
        }
    }
    panic!("actual native{child} normal cast must damage its target within200 frames");
}

fn qualify_native_overload_effectiveness(overload: u32) {
    let mut h = harness(Vec::new(), &[overload, MASTERY, ECHO_CHAMBER], 7).unwrap();
    let store = h.app.world().resource::<crate::spell_info::SpellStore>();
    let effect = &store.get(overload).unwrap().effects[0];
    assert_eq!(effect.base_points, 0.0);
    let coefficient = effect.bonus_coefficient;
    let variance = effect.variance / 2.0;
    let percent = store
        .get(MASTERY)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .base_points;
    assert_eq!(percent, 25.0);
    let stats = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(h.caster)
        .unwrap();
    let base = coefficient * stats.spell_power;
    // This existing harness has fixed SP and zero mastery points/crit/versatility.
    // No synthetic mastery coefficients, RNG replacement or damage tolerance broadening.
    assert_eq!(stats.mastery_points, 0.0);
    assert_eq!(stats.crit_pct, 0.0);
    assert_eq!(stats.versatility_pct, 0.0);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: MASTERY,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let hit = cast_native_child(&mut h, overload);
    assert!(!hit.crit);
    let minimum = (base * (1.0 - variance) * percent / 100.0).round() as i32;
    let maximum = (base * (1.0 + variance) * percent / 100.0).round() as i32;
    assert!(
        (minimum..=maximum).contains(&hit.amount),
        "native25%: actual={}, expected={minimum}..={maximum}",
        hit.amount
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == MASTERY,
    );
    let hit = cast_native_child(&mut h, overload);
    let minimum = (base * (1.0 - variance)).round() as i32;
    let maximum = (base * (1.0 + variance)).round() as i32;
    assert!(
        (minimum..=maximum).contains(&hit.amount),
        "TC leaves a directly cast child unscaled without its mastery aura: actual={}",
        hit.amount
    );
    let echo_percent = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(ECHO_CHAMBER)
        .unwrap()
        .effects[0]
        .base_points;
    for spell_id in [ECHO_CHAMBER, MASTERY] {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: h.caster,
                spell_id,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    let hit = cast_native_child(&mut h, overload);
    let modified_percent = percent * (1.0 + echo_percent / 100.0);
    let minimum = (base * (1.0 - variance) * modified_percent / 100.0).round() as i32;
    let maximum = (base * (1.0 + variance) * modified_percent / 100.0).round() as i32;
    assert!(
        (minimum..=maximum).contains(&hit.amount),
        "actual native108/op12 scales effect1, not an invented flat25+25 multiplier: damage={}",
        hit.amount
    );
}

fn cast_lava_damage_with_flame_shock(
    damage: u32,
    owns_flame_shock: bool,
) -> shared::protocol::CombatLogEvent {
    const FLAME_SHOCK: u32 = 188389;
    let mut h = harness(Vec::new(), &[damage, FLAME_SHOCK], 7).unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
            .crit_pct,
        0.0,
        "existing fixed-crit fixture makes owned/foreign crit flags deterministic without RNG overrides"
    );
    let flame_shock_caster = if owns_flame_shock {
        h.caster
    } else {
        super::test_harness::spawn_player(&mut h.app, 7, 0.0)
    };
    // Controlled actual native AuraApplication isolates ownership, not acquisition.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: flame_shock_caster,
            target: h.npc,
            spell_id: FLAME_SHOCK,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == FLAME_SHOCK)
        .unwrap();
    assert_eq!(aura.caster, flame_shock_caster);
    cast_native_child(&mut h, damage)
}

#[test]
#[ignore = "TC caster-owned Flame Shock root crit RED/GREEN pending"]
fn native_lava_crit_root_own_flame_shock_guarantees_crit() {
    assert!(cast_lava_damage_with_flame_shock(285452, true).crit);
}

#[test]
#[ignore = "TC foreign Flame Shock root crit RED/GREEN pending"]
fn native_lava_crit_root_foreign_flame_shock_does_not_guarantee_crit() {
    assert!(!cast_lava_damage_with_flame_shock(285452, false).crit);
}

#[test]
#[ignore = "TC caster-owned Flame Shock Overload crit RED/GREEN pending"]
fn native_lava_crit_overload_own_flame_shock_guarantees_crit() {
    assert!(cast_lava_damage_with_flame_shock(285466, true).crit);
}

#[test]
#[ignore = "TC foreign Flame Shock Overload crit RED/GREEN pending"]
fn native_lava_crit_overload_foreign_flame_shock_does_not_guarantee_crit() {
    assert!(!cast_lava_damage_with_flame_shock(285466, false).crit);
}

#[test]
fn native_lightning_overload_uses_mastery_effect1_and_removal_restores_child_base() {
    qualify_native_overload_effectiveness(45284);
}

#[test]
fn native_chain_overload_uses_mastery_effect1_and_removal_restores_child_base() {
    qualify_native_overload_effectiveness(45297);
}

#[test]
fn native_lava_overload_uses_mastery_effect1_and_removal_restores_child_base() {
    qualify_native_overload_effectiveness(285466);
}

#[test]
fn native_eb_overload_uses_mastery_effect1_and_removal_restores_child_base() {
    qualify_native_overload_effectiveness(120588);
}

#[test]
fn native_icefury_overload_uses_mastery_effect1_and_removal_restores_child_base() {
    qualify_native_overload_effectiveness(219271);
}
