use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native Mistweaver spell data"]
fn thunder_focus_tea_gives_enveloping_its_direct_heal() {
    for empowered in [false, true] {
        let mut h = harness(Vec::new(), &[124682, 116680, 274062], 10).unwrap();
        h.app.init_resource::<ObservedCombatEvents>();
        let target = spawn_player(&mut h.app, 10, 3.0);
        *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
            current: 1.0,
            max: 100_000.0,
        };
        if empowered {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: h.caster,
                    target: h.caster,
                    spell_id: 116680,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 124682, Some(target), false)
            .unwrap();
        h.advance(55);
        let events = &h.app.world().resource::<ObservedCombatEvents>().0;
        let direct = events.iter().any(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(274062)
                && event.source == Some(h.caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.amount > 0
        });
        assert_eq!(direct, empowered, "empowered={empowered}: {events:?}");
    }
}
