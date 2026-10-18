//! Generic native periodic-aura lifetimes, not a Fel Devastation script test.
use shared::protocol::CombatLogKind;

use super::*;
use crate::spell_cast::test_harness::{harness, world_db};

const CAUSALITY_TEAR: u32 = 436_117;
const CYCLES: usize = 4;

#[test]
#[ignore = "RED: repeated fractional period addition drops native expiry ticks"]
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
    let expected = (duration / period) as usize;
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
