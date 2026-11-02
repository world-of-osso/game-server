use crate::spell_cast::test_harness::{ObservedCombatEvents, harness};
use shared::components::{ActiveSpec, Health};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native Mistweaver spell data"]
fn emperors_elixir_tea_kick_deals_jadefire_and_transfers_a_fifth() {
    for (talent, tea, expected) in [
        (true, true, true),
        (true, false, false),
        (false, true, false),
    ] {
        let spells = [
            107428, 185099, 1248815, 1268807, 116680, 388023, 388024, 388025, 467293,
        ];
        let mut h = harness(Vec::new(), &spells, 10).unwrap();
        h.app.init_resource::<ObservedCombatEvents>();
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(ActiveSpec(270));
        *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
            current: 1.0,
            max: 100_000_000.0,
        };
        for id in [
            Some(388023),
            Some(467293),
            talent.then_some(1268807),
            tea.then_some(116680),
        ]
        .into_iter()
        .flatten()
        {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: h.caster,
                    target: h.caster,
                    spell_id: id,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        h.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
            .crit_pct = 0.0;
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 107428, Some(h.npc), false)
            .unwrap();
        h.advance(20);
        let events = &h.app.world().resource::<ObservedCombatEvents>().0;
        let hit = events.iter().position(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1248815)
                && event.source == Some(h.caster.to_bits())
        });
        assert_eq!(
            hit.is_some(),
            expected,
            "talent={talent} tea={tea}: {events:?}"
        );
        if let Some(index) = hit {
            let heal = events[index + 1..]
                .iter()
                .find(|event| {
                    event.kind == CombatLogKind::Heal
                        && event.spell_id == Some(388024)
                        && event.target == Some(h.caster.to_bits())
                })
                .expect("Jadefire damage must transfer through Ancient Teachings");
            let expected_heal = events[index].amount as f32 * 3.45 * 0.20;
            assert!(
                (heal.amount as f32 - expected_heal).abs() <= 3.0,
                "{heal:?}, expected {expected_heal}"
            );
        }
    }
}
