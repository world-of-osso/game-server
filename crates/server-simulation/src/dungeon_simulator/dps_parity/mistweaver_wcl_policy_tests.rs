use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::casting::CastState;
use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires the level-90 follower snapshot"]
fn soothing_prepares_a_ready_heal_below_sampled_health_otherwise_kicks() {
    for (fraction, expected) in [(0.50, 115175), (0.80, 107428)] {
        let mut party = super::super::healer_simc_replay::snapshot_party(270, 90, 1);
        let healer = party.healer;
        let target = party.tank;
        let enemy = party.sources[0];
        let position = *party.app.world().get::<Position>(healer).unwrap();
        for unit in [target, enemy] {
            party.app.world_mut().entity_mut(unit).remove::<Position>();
            party.app.world_mut().entity_mut(unit).insert(position);
        }
        for unit in party.slots {
            let mut health = party.app.world_mut().get_mut::<Health>(unit).unwrap();
            health.current = health.max;
        }
        let mut health = party.app.world_mut().get_mut::<Health>(target).unwrap();
        health.current = health.max * fraction;
        party.app.world_mut().entity_mut(healer).insert(
            crate::class_progression::KnownSpells::fixture(vec![115175, 124682, 107428, 185099]),
        );
        let encounter = crate::simc_apl_native::Encounter::Fixture {
            started_at: crate::spell_cast::now(party.app.world()),
            duration: 120.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        };
        let mut rotation = server_follower_apl::mistweaver_rotation::Mistweaver::load(
            party.app.world_mut(),
            healer,
            encounter,
        )
        .unwrap();
        let start = party.app.world().resource::<ObservedCombatEvents>().0.len();
        rotation
            .drive(party.app.world_mut(), healer, &party.slots, Some(enemy))
            .unwrap();
        let events = &party.app.world().resource::<ObservedCombatEvents>().0[start..];
        assert!(
            events
                .iter()
                .any(|event| event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(healer.to_bits())
                    && event.spell_id == Some(expected)),
            "health={fraction} expected {expected}: {events:?}"
        );
        if expected == 115175 {
            assert_eq!(
                party.app.world().get::<CastState>(healer).unwrap().spell_id,
                115175
            );
            for _ in 0..35 {
                party.app.update();
            }
            // SpellHistory uses the real clock, not the fixture's fixed virtual time.
            party
                .app
                .world_mut()
                .resource_mut::<bevy::prelude::Time<bevy::time::Real>>()
                .advance_by(std::time::Duration::from_secs(2));
            let mut health = party.app.world_mut().get_mut::<Health>(target).unwrap();
            health.current = health.max * fraction;
            let start = party.app.world().resource::<ObservedCombatEvents>().0.len();
            rotation
                .drive(party.app.world_mut(), healer, &party.slots, Some(enemy))
                .unwrap();
            let spell = party
                .app
                .world()
                .resource::<crate::spell_info::SpellStore>()
                .get(124682)
                .unwrap()
                .clone();
            let readiness = crate::spell_cast::read_native_cast_start(
                party.app.world_mut(),
                healer,
                &spell,
                target,
            );
            let mist = crate::auras::has_aura(party.app.world(), target, 124682);
            let events = &party.app.world().resource::<ObservedCombatEvents>().0[start..];
            assert!(
                events
                    .iter()
                    .any(|event| event.kind == CombatLogKind::CastSuccess
                        && event.spell_id == Some(124682)
                        && event.target == Some(target.to_bits())),
                "ready Enveloping must follow the channel: readiness={readiness:?}, mist={mist}, health={:?}, cast={:?}",
                party.app.world().get::<Health>(target),
                party.app.world().get::<CastState>(healer)
            );
        }
    }
}
