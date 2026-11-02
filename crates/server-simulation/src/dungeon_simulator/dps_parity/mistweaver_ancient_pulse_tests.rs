use crate::spell_cast::test_harness::{ObservedCombatEvents, harness};
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native Mistweaver spell data"]
fn hostile_party_pulse_does_not_trigger_monk_ancient_teachings() {
    let mut h = harness(Vec::new(), &[388023, 388024, 388025, 467293], 10).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
        current: 50_000.0,
        max: 100_000.0,
    };
    for spell_id in [388023, 467293] {
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
    let start = h.app.world().resource::<ObservedCombatEvents>().0.len();
    super::super::healer_simc_replay::deliver_party_hit(
        h.app.world_mut(),
        h.npc,
        h.caster,
        1_000.0,
    );
    h.advance(4);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0[start..];
    assert!(
        events
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.source == Some(h.npc.to_bits())
                && event.target == Some(h.caster.to_bits())
                && event.amount > 0)
    );
    assert!(
        !events.iter().any(|event| event.kind == CombatLogKind::Heal
            && matches!(event.spell_id, Some(388024 | 388025))),
        "hostile pulse must not trigger Ancient Teachings: {events:?}"
    );
}
