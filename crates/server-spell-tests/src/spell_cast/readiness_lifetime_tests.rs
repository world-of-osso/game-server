//! Reproduce native-equal-budget2's 400ms server frames with real elapsed time.
//! TC a352b1fa SpellHistory.cpp:834–855 uses GameTime for charge deadlines.
//! Keep Bevy's normal virtual-time cap: simulation catch-up is not this contract.
use std::time::{Duration, Instant};

use bevy::time::TimeUpdateStrategy;
use shared::protocol::CombatLogKind;
use shared::spell_data::CastFailReason;

use super::test_harness::*;

struct ReadinessProbe {
    harness: Harness,
    spell: u32,
    recovery: Duration,
    requested_again: bool,
}

impl ReadinessProbe {
    fn start(mut harness: Harness, spell: u32, recovery_ms: u32) -> Self {
        // Finish setup and catch up the real clock before consuming its charge.
        harness.app.insert_resource(TimeUpdateStrategy::Automatic);
        harness.advance(1);
        harness.cast_and_deliver(spell, Some(harness.npc));
        assert!(
            harness.inbox.failed.is_empty(),
            "{:?}",
            harness.inbox.failed
        );
        let charge = harness.inbox.charges.last().expect("consumed owner charge");
        assert_eq!((charge.current, charge.max), (0, 1));
        assert_eq!(charge.recharge_ms, recovery_ms);
        harness.inbox.log.clear();
        harness.inbox.failed.clear();
        Self {
            harness,
            spell,
            recovery: Duration::from_millis(u64::from(recovery_ms)),
            requested_again: false,
        }
    }

    fn update(&mut self, elapsed: Duration) {
        self.harness.advance(1);
        // Let owner/intent delivery complete without relying on an exact frame rate.
        if !self.requested_again && elapsed >= self.recovery + Duration::from_secs(2) {
            self.harness.cast(self.spell, Some(self.harness.npc));
            self.requested_again = true;
        }
    }

    fn assert_recovered(&self) {
        assert!(self.requested_again);
        let successes: Vec<_> = self
            .harness
            .inbox
            .log
            .iter()
            .filter(|event| {
                event.spell_id == Some(self.spell) && event.kind == CombatLogKind::CastSuccess
            })
            .collect();
        assert_eq!(
            successes.len(),
            1,
            "spell {} rejected after real recovery: {:?}",
            self.spell,
            self.harness.inbox.failed
        );
        assert!(
            self.harness
                .inbox
                .charges
                .iter()
                .any(|charge| charge.current == 1)
        );
    }
}

#[test]
fn native_readiness_real_sixty_second_owner_lifetime_survives_slow_frames() {
    // Active70391 SpellCategories1072/47749 → SpellCategory1680/1612.
    // Neither charge may age while the other fixture loads canonical data.
    let overpower = harness(Vec::new(), &[7384], 1).unwrap();
    let glaive = harness(Vec::new(), &[185123], 12).unwrap();
    let mut overpower = ReadinessProbe::start(overpower, 7384, 12_000);
    let mut glaive = ReadinessProbe::start(glaive, 185123, 9_000);
    let started = Instant::now();
    let mut checked_gcd = false;
    while started.elapsed() < Duration::from_secs(60) {
        std::thread::sleep(Duration::from_millis(400));
        let elapsed = started.elapsed();
        overpower.update(elapsed);
        glaive.update(elapsed);
        if !checked_gcd && elapsed >= Duration::from_secs(2) {
            // The authored 1500ms GCD must have expired even though the charge has not.
            overpower.harness.cast(7384, Some(overpower.harness.npc));
            checked_gcd = true;
        }
    }
    assert!(checked_gcd);
    assert_eq!(
        overpower.harness.inbox.failed.len(),
        1,
        "only the intentional pre-recharge request may fail: {:?}",
        overpower.harness.inbox.failed
    );
    assert_eq!(
        overpower.harness.inbox.failed[0].reason,
        CastFailReason::NoChargesRemain
    );
    overpower.assert_recovered();
    glaive.assert_recovered();
}

#[test]
fn native_fury_slam_gcd_recovers_during_slow_real_frames() {
    use crate::auras::{AuraApplication, AuraSource, apply_aura};

    let mut h = harness(Vec::new(), &[1464, 137050], 1).unwrap();
    let caster = h.caster;
    // Native Fury137050e8 removes Slam's Rage cost; no resource/stat injection.
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target: caster,
            spell_id: 137050,
            effect_mask: 1 << 8,
            source: AuraSource::Cast,
        },
    );
    h.cast_and_deliver(1464, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let gcd = h.inbox.cooldowns.iter().find(|timer| timer.is_gcd).unwrap();
    assert_eq!(gcd.duration_ms, 1500);
    h.inbox.log.clear();
    h.app.insert_resource(TimeUpdateStrategy::Automatic);
    let started = Instant::now();
    let mut requested = false;
    while started.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(800));
        h.advance(1);
        if !requested && started.elapsed() >= Duration::from_secs(2) {
            h.cast(1464, Some(h.npc));
            requested = true;
        }
    }
    assert!(requested);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|event| {
                event.spell_id == Some(1464) && event.kind == CombatLogKind::CastSuccess
            })
            .count(),
        1
    );
}
