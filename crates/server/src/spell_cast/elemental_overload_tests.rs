//! Controlled real-native child casts isolate TC effect1 effectiveness, not acquisition/DPS.
use super::test_harness::{Harness, harness};
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

const OVERLOAD: u32 = 45284;
const MASTERY: u32 = 168534;
const ECHO_CHAMBER: u32 = 382032;

fn cast_native_child(h: &mut Harness) -> shared::protocol::CombatLogEvent {
    let first = h.inbox.log.len();
    super::creature_cast(h.app.world_mut(), h.caster, OVERLOAD, Some(h.npc), false).unwrap();
    h.advance(20);
    h.inbox.log[first..]
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(h.caster.to_bits())
                && event.target == Some(h.npc.to_bits())
                && event.spell_id == Some(OVERLOAD)
        })
        .expect("actual native45284 normal cast must damage its target")
        .clone()
}

#[test]
fn native_lightning_overload_uses_mastery_effect1_and_removal_restores_child_base() {
    let mut h = harness(Vec::new(), &[OVERLOAD, MASTERY, ECHO_CHAMBER], 7).unwrap();
    let store = h.app.world().resource::<crate::spell_info::SpellStore>();
    let effect = &store.get(OVERLOAD).unwrap().effects[0];
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
    let hit = cast_native_child(&mut h);
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
    let hit = cast_native_child(&mut h);
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
    let hit = cast_native_child(&mut h);
    let modified_percent = percent * (1.0 + echo_percent / 100.0);
    let minimum = (base * (1.0 - variance) * modified_percent / 100.0).round() as i32;
    let maximum = (base * (1.0 + variance) * modified_percent / 100.0).round() as i32;
    assert!(
        (minimum..=maximum).contains(&hit.amount),
        "actual native108/op12 scales effect1, not an invented flat25+25 multiplier: damage={}",
        hit.amount
    );
}
