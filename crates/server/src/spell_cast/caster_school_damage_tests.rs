//! TC a352b1fa Unit.cpp7056–7059/8178–8181; published70391 Haunt effect1.
use super::test_harness::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason, apply_aura};
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

const HAUNT: u32 = 48181;
const FIREBALL: u32 = 133;
const OPEN_WOUNDS: u32 = 188400;
// Native Fireball:100 SP ×3.8812499, with the authored ±2.5% variance.
const FIREBALL_BASE: f32 = 388.125;

fn fixture() -> Harness {
    harness(Vec::new(), &[HAUNT, FIREBALL, OPEN_WOUNDS, 172, 146739], 9).unwrap()
}

fn cast_haunt(h: &mut Harness) {
    h.cast_and_deliver(HAUNT, Some(h.npc));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, HAUNT));
}

fn fireball_damage(h: &mut Harness) -> f32 {
    let before = h.health(h.npc);
    let first_event = h.inbox.log.len();
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    h.advance(90);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let hits: Vec<_> = h.inbox.log[first_event..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(FIREBALL)
                && event.source == Some(h.caster.to_bits())
                && event.target == Some(h.npc.to_bits())
        })
        .collect();
    assert_eq!(hits.len(), 1, "one completed ordinary Fireball");
    let loss = before - h.health(h.npc);
    assert_eq!(loss, hits[0].amount as f32);
    loss
}

#[test]
fn caster_school_damage_native_haunt_amplifies_owner_fireball_by_sixteen_percent() {
    let mut h = fixture();
    cast_haunt(&mut h);
    let damage = fireball_damage(&mut h);
    assert!(
        varied(FIREBALL_BASE * 1.16).contains(&damage),
        "Haunt damage {damage}"
    );
}

#[test]
fn caster_school_damage_native_haunt_amplifies_actual_corruption_ticks() {
    let mut h = fixture();
    cast_haunt(&mut h);
    let first_event = h.inbox.log.len();
    h.cast_and_deliver(172, Some(h.npc));
    h.advance(90);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let ticks: Vec<_> = h.inbox.log[first_event..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(146739)
                && event.source == Some(h.caster.to_bits())
                && event.target == Some(h.npc.to_bits())
                && event.periodic
        })
        .collect();
    assert_eq!(ticks.len(), 2, "two native2000ms Corruption ticks");
    for tick in ticks {
        // Native146739:100 SP ×0.380104989, then Haunt16%.
        assert!(
            varied(38.0105 * 1.16).contains(&(tick.amount as f32)),
            "Corruption tick {}",
            tick.amount
        );
    }
}

#[test]
fn caster_school_damage_native_baseline_foreign_owner_and_school_exclusion() {
    for condition in ["baseline", "foreign Haunt", "physical-only Open Wounds"] {
        let mut h = fixture();
        let (caster, spell, effect_mask) = match condition {
            "foreign Haunt" => (spawn_player(&mut h.app, 9, 0.0), HAUNT, 2),
            "physical-only Open Wounds" => (h.caster, OPEN_WOUNDS, 1),
            _ => (h.caster, 0, 0),
        };
        if spell != 0 {
            apply_aura(
                h.app.world_mut(),
                &AuraApplication {
                    caster,
                    target: h.npc,
                    spell_id: spell,
                    effect_mask,
                    source: AuraSource::Cast,
                },
            );
            assert!(crate::auras::has_aura(h.app.world(), h.npc, spell));
        }
        let damage = fireball_damage(&mut h);
        assert!(
            varied(FIREBALL_BASE).contains(&damage),
            "{condition}: {damage}"
        );
    }
}

#[test]
fn caster_school_damage_native_haunt_cancel_and_expiry_restore_baseline() {
    for expires in [false, true] {
        let mut h = fixture();
        cast_haunt(&mut h);
        if expires {
            h.advance(500);
        } else {
            crate::auras::remove_auras(h.app.world_mut(), h.npc, RemoveReason::Cancelled, |aura| {
                aura.spell_id == HAUNT
            });
        }
        assert!(!crate::auras::has_aura(h.app.world(), h.npc, HAUNT));
        let damage = fireball_damage(&mut h);
        assert!(
            varied(FIREBALL_BASE).contains(&damage),
            "expired={expires}: {damage}"
        );
    }
}
