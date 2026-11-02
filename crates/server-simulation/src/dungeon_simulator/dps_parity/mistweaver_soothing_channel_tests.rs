use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
use shared::casting::{CastState, CastType};
use shared::components::{ActiveSpec, Health};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native Mistweaver spell data"]
fn soothing_mist_allows_instant_heals_on_its_target_without_losing_channel() {
    for spell in [116670, 124682, 399491] {
        let mut h = harness(Vec::new(), &[115175, 116670, 124682, 399491], 10).unwrap();
        h.app.init_resource::<ObservedCombatEvents>();
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(ActiveSpec(270));
        let target = spawn_player(&mut h.app, 10, 3.0);
        let other = spawn_player(&mut h.app, 10, 4.0);
        for unit in [target, other] {
            let mut health = h.app.world_mut().get_mut::<Health>(unit).unwrap();
            health.current = health.max / 2.0;
        }
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 115175, Some(target), false)
            .unwrap();
        h.advance(35);
        let channel = h
            .app
            .world()
            .get::<CastState>(h.caster)
            .expect("Soothing Mist channel");
        assert_eq!(
            (channel.spell_id, channel.cast_type, channel.target),
            (115175, CastType::Channel, target.to_bits())
        );
        assert!(
            crate::spell_cast::creature_cast(
                h.app.world_mut(),
                h.caster,
                spell,
                Some(other),
                false
            )
            .is_err(),
            "off-target cast must not use this channel"
        );
        let start = h.app.world().resource::<ObservedCombatEvents>().0.len();
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, spell, Some(target), false)
            .unwrap();
        let channel = h
            .app
            .world()
            .get::<CastState>(h.caster)
            .expect("cast must preserve the channel");
        assert_eq!(
            (channel.spell_id, channel.cast_type, channel.target),
            (115175, CastType::Channel, target.to_bits())
        );
        let events = &h.app.world().resource::<ObservedCombatEvents>().0[start..];
        assert!(
            events
                .iter()
                .any(|event| event.kind == CombatLogKind::CastSuccess
                    && event.spell_id == Some(spell)),
            "{spell} must resolve immediately: {events:?}"
        );
        h.advance(25);
        let events = &h.app.world().resource::<ObservedCombatEvents>().0[start..];
        assert!(
            events.iter().any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(115175)
                && event.target == Some(target.to_bits())),
            "Soothing Mist must tick after {spell}"
        );
    }
}
