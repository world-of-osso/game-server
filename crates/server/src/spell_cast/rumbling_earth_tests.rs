//! Rumbling Earth uses the successful original Shockwave target set, not survivors.
use super::test_harness::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason};
use bevy::prelude::*;
use shared::components::{Health, Rotation};
use shared::protocol::CombatLogKind;

const SHOCKWAVE: u32 = 46968;
const STUN: u32 = 132168;
const RUMBLING_EARTH: u32 = 275339;
const PROTECTION: u32 = 1022;

fn spawn_shockwave_caster(talent: bool, count: usize) -> (Harness, Vec<Entity>) {
    let mut h = harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[SHOCKWAVE, STUN, RUMBLING_EARTH, PROTECTION],
        1,
    )
    .unwrap();
    h.app.world_mut().entity_mut(h.caster).insert(Rotation {
        x: 0.0,
        y: std::f32::consts::FRAC_PI_2,
        z: 0.0,
    });
    let mut known = vec![SHOCKWAVE];
    if talent {
        known.push(RUMBLING_EARTH);
    } else {
        crate::auras::remove_auras(
            h.app.world_mut(),
            h.caster,
            RemoveReason::Cancelled,
            |aura| aura.spell_id == RUMBLING_EARTH,
        );
    }
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(known));
    h.advance(3);
    h.inbox = Inbox::default();
    let mut targets = vec![h.npc];
    for index in 1..count {
        targets.push(spawn_npc(&mut h.app, 3.0 + index as f32 * 0.5));
    }
    (h, targets)
}

fn cast_shockwave(h: &mut Harness) -> u32 {
    h.cast_and_deliver(SHOCKWAVE, None);
    h.advance(12);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == SHOCKWAVE));
    let update = h
        .inbox
        .cooldowns
        .iter()
        .rev()
        .find(|update| update.spell_id == SHOCKWAVE && !update.is_gcd)
        .expect("native Shockwave publishes its cooldown");
    assert_eq!(update.duration_ms, 40000);
    update.remaining_ms
}

#[test]
fn rumbling_earth_native_three_target_boundary_reduces_once_by_fifteen_seconds() {
    for (count, remaining) in [(2, 40000), (3, 25000), (4, 25000)] {
        let (mut h, targets) = spawn_shockwave_caster(true, count);
        let observed = cast_shockwave(&mut h);
        for target in targets {
            assert!(h.health(target) < 5000.0, "actual source cone damage");
            assert!(crate::auras::has_aura(h.app.world(), target, STUN));
        }
        assert_eq!(observed, remaining, "native{count}-target threshold");
    }
}

#[test]
fn rumbling_earth_native_absent_talent_does_not_reduce_three_target_cast() {
    let (mut h, _) = spawn_shockwave_caster(false, 3);
    assert_eq!(cast_shockwave(&mut h), 40000);
}

#[test]
fn rumbling_earth_native_killing_hits_still_count_for_the_same_cast() {
    let (mut h, targets) = spawn_shockwave_caster(true, 3);
    for target in &targets {
        h.app
            .world_mut()
            .get_mut::<Health>(*target)
            .unwrap()
            .current = 1.0;
    }
    let remaining = cast_shockwave(&mut h);
    for target in targets {
        assert_eq!(
            h.health(target),
            0.0,
            "native Shockwave killed its launch target"
        );
    }
    assert_eq!(
        remaining, 25000,
        "count the launch hits, not surviving units"
    );
}

#[test]
fn rumbling_earth_native_physical_immune_target_does_not_meet_threshold() {
    let (mut h, targets) = spawn_shockwave_caster(true, 3);
    let immune = targets[2];
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: immune,
            target: immune,
            spell_id: PROTECTION,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    assert_eq!(cast_shockwave(&mut h), 40000);
    assert_eq!(h.health(immune), 5000.0);
    assert!(!h.inbox.log.iter().any(|event| {
        event.spell_id == Some(SHOCKWAVE)
            && event.kind == CombatLogKind::Damage
            && event.target == Some(immune.to_bits())
    }));
    assert!(targets[..2].iter().all(|target| h.health(*target) < 5000.0));
}
