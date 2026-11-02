use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
use shared::casting::{CastState, CastType};
use shared::components::{ActiveSpec, Health};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native Mistweaver spell data"]
fn empowered_enveloping_starts_a_real_soothing_channel() {
    for (talent, tea, expected) in [
        (true, true, true),
        (true, false, false),
        (false, true, false),
    ] {
        let mut h = harness(Vec::new(), &[124682, 115175, 1268807, 116680], 10).unwrap();
        h.app.init_resource::<ObservedCombatEvents>();
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(ActiveSpec(270));
        let target = spawn_player(&mut h.app, 10, 3.0);
        *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
            current: 1.0,
            max: 100_000.0,
        };
        for id in [talent.then_some(1268807), tea.then_some(116680)]
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
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 124682, Some(target), false)
            .unwrap();
        h.advance(55);
        let channel = h.app.world().get::<CastState>(h.caster);
        let soothing = channel.is_some_and(|cast| {
            cast.spell_id == 115175
                && cast.cast_type == CastType::Channel
                && cast.target == target.to_bits()
        });
        assert_eq!(soothing, expected, "talent={talent} tea={tea}: {channel:?}");
        let events = &h.app.world().resource::<ObservedCombatEvents>().0;
        let ticked = events.iter().any(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(115175)
                && event.source == Some(h.caster.to_bits())
                && event.target == Some(target.to_bits())
        });
        assert_eq!(ticked, expected, "talent={talent} tea={tea}: {events:?}");
    }
}
