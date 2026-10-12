//! TC a352b1fa Unit.cpp2898: victim aura187 affects melee, not ranged, crit.
use super::test_harness::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason};
use crate::combat::AutoAttack;
use crate::stats::CharacterStats;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

const HARDENED_BATTLELYNX: u32 = 460575;
const SLAM: u32 = 1464;
const NATIVE_CRIT_REDUCTION: f32 = 60.0;

fn protected_target() -> Harness {
    let mut h = harness(Vec::new(), &[HARDENED_BATTLELYNX, SLAM], 1).unwrap();
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = NATIVE_CRIT_REDUCTION;
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 1_000_000.0;
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().max = 1_000_000.0;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.npc,
            target: h.npc,
            spell_id: HARDENED_BATTLELYNX,
            effect_mask: 1 << 2,
            source: AuraSource::Cast,
        },
    );
    h
}

fn slam_repeatedly(h: &mut Harness) {
    for _ in 0..24 {
        h.set_power(PowerType::Rage, 1000.0);
        h.cast_and_deliver(SLAM, Some(h.npc));
        h.advance(40);
    }
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

fn damage_events(h: &Harness) -> Vec<&shared::protocol::CombatLogEvent> {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.target == Some(h.npc.to_bits()) && event.kind == CombatLogKind::Damage
        })
        .collect()
}

#[test]
fn class_completeness_native_hardened_battlelynx_prevents_slam_crit_until_removed() {
    let mut h = protected_target();
    slam_repeatedly(&mut h);
    let protected = damage_events(&h);
    assert!(protected.len() >= 24, "{:?}", h.inbox.log);
    assert!(protected.iter().all(|event| !event.crit), "{protected:?}");
    crate::auras::remove_auras(h.app.world_mut(), h.npc, RemoveReason::Cancelled, |aura| {
        aura.spell_id == HARDENED_BATTLELYNX
    });
    h.inbox.log.clear();
    slam_repeatedly(&mut h);
    assert!(
        damage_events(&h).iter().any(|event| event.crit),
        "{:?}",
        h.inbox.log
    );
}

#[test]
fn class_completeness_native_hardened_battlelynx_prevents_auto_attack_crit() {
    let mut h = protected_target();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(AutoAttack::new(h.npc));
    h.advance(1000);
    let protected = damage_events(&h);
    assert!(protected.len() >= 12, "{:?}", h.inbox.log);
    assert!(protected.iter().all(|event| !event.crit), "{protected:?}");
}
