//! Generic native periodic-aura lifetimes, not a Fel Devastation script test.
use shared::protocol::CombatLogKind;

use super::*;
use crate::spell_cast::test_harness::{harness, world_db};

const CAUSALITY_TEAR: u32 = 436_117;
const CYCLES: usize = 4;

#[test]
fn periodic_native_fractional_period_keeps_every_expiry_tick_across_several_lifetimes() {
    let mut h = harness(Vec::new(), &[CAUSALITY_TEAR], 8).unwrap();
    let spell = world_db().unwrap().store.get(CAUSALITY_TEAR).unwrap();
    let duration = spell
        .misc
        .as_ref()
        .unwrap()
        .duration
        .as_ref()
        .unwrap()
        .duration_ms;
    let period = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .aura_period_ms;
    assert!(duration > 0 && period > 0, "finite native periodic fixture");
    let expected = duration as usize / period as usize;
    // Advance ordinary application frames, not a rewritten clock: repeated0.2s
    // addition drifts past expiry at later virtual uptime.
    h.advance(1280);
    let mut counts = Vec::new();
    for _ in 0..CYCLES {
        let before_log = h.inbox.log.len();
        let before_health = h.health(h.npc);
        apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster: h.caster,
                target: h.npc,
                spell_id: CAUSALITY_TEAR,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
        assert!(has_aura(h.app.world(), h.npc, CAUSALITY_TEAR));
        h.advance(duration as usize / 50 + 6);
        let ticks: Vec<_> = h.inbox.log[before_log..]
            .iter()
            .filter(|event| {
                event.spell_id == Some(CAUSALITY_TEAR)
                    && event.source == Some(h.caster.to_bits())
                    && event.target == Some(h.npc.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.periodic
            })
            .collect();
        assert!(ticks.iter().all(|tick| tick.amount == 2 && !tick.crit));
        assert_eq!(
            before_health - h.health(h.npc),
            ticks.iter().map(|tick| tick.amount as f32).sum::<f32>()
        );
        assert!(!has_aura(h.app.world(), h.npc, CAUSALITY_TEAR));
        counts.push(ticks.len());
        h.advance(14);
    }
    assert_eq!(
        counts,
        vec![expected; CYCLES],
        "every native3s/200ms lifetime must deliver15 ticks"
    );
}

#[test]
fn periodic_power_changes_are_live_unless_native_effect_snapshots() {
    for snapshot in [false, true] {
        let data = world_db().unwrap();
        let mut spell = data.spell_store().get(146739).unwrap().clone();
        let effect = spell
            .effects
            .iter_mut()
            .find(|effect| effect.index == 0)
            .unwrap();
        effect.variance = 0.0;
        if snapshot {
            effect.attributes |= 1 << 16;
        }
        let mut h = harness(vec![spell], &[], 9).unwrap();
        h.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
            .crit_pct = 0.0;
        apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster: h.caster,
                target: h.npc,
                spell_id: 146739,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
        h.advance(45);
        let ticks = |h: &crate::spell_cast::test_harness::Harness| -> Vec<i32> {
            h.inbox
                .log
                .iter()
                .filter(|event| {
                    event.spell_id == Some(146739)
                        && event.kind == CombatLogKind::Damage
                        && event.periodic
                })
                .map(|event| event.amount)
                .collect()
        };
        let first = *ticks(&h).first().expect("first actual periodic tick");
        h.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
            .spell_power += 300.0;
        h.advance(40);
        let second = *ticks(&h).get(1).expect("second actual periodic tick");
        if snapshot {
            assert_eq!(second, first, "EX_COMPUTE_ON_CAST keeps application power");
        } else {
            assert!(
                second > first * 2,
                "ordinary periodic damage uses current spell power: {first} -> {second}"
            );
        }
    }
}

#[test]
fn periodic_healing_uses_live_healing_power() {
    let mut h = harness(Vec::new(), &[139], 5).unwrap();
    let stats = h
        .app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap();
    stats.into_inner().crit_pct = 0.0;
    let health = h
        .app
        .world_mut()
        .get_mut::<shared::components::Health>(h.caster)
        .unwrap()
        .into_inner();
    health.current = 1_000.0;
    health.max = 100_000.0;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 139,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    let heals = |h: &crate::spell_cast::test_harness::Harness| -> Vec<i32> {
        h.inbox
            .log
            .iter()
            .filter(|event| {
                event.spell_id == Some(139) && event.kind == CombatLogKind::Heal && event.periodic
            })
            .map(|event| event.amount)
            .collect()
    };
    h.advance(65);
    let initial = heals(&h);
    let first = *initial.last().expect("first actual Renew tick");
    let before = initial.len();
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .healing_power += 300.0;
    h.advance(60);
    let second = *heals(&h).get(before).expect("next actual Renew tick");
    assert!(
        second > first * 2,
        "periodic healing must use current healing power: {first} -> {second}"
    );
}
