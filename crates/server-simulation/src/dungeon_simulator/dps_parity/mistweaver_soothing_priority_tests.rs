use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::casting::CastState;
use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires the level-90 follower snapshot"]
fn mistweaver_interrupts_soothing_for_ready_kick_only_without_emergency() {
    for (health_fraction, should_kick) in [(0.8, true), (0.3, false)] {
        let mut party = super::super::healer_simc_replay::snapshot_party(270, 90, 1);
        let healer = party.healer;
        let target = party.tank;
        let enemy = party.sources[0];
        let position = *party.app.world().get::<Position>(healer).unwrap();
        for unit in [target, enemy] {
            party.app.world_mut().entity_mut(unit).remove::<Position>();
            party.app.world_mut().entity_mut(unit).insert(position);
        }
        crate::spell_cast::creature_cast(
            party.app.world_mut(),
            healer,
            115175,
            Some(target),
            false,
        )
        .unwrap();
        for _ in 0..35 {
            party.app.update();
        }
        assert_eq!(
            party.app.world().get::<CastState>(healer).unwrap().spell_id,
            115175
        );
        for unit in party.slots {
            let mut health = party.app.world_mut().get_mut::<Health>(unit).unwrap();
            health.current = health.max * health_fraction;
        }
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
        let before = party.app.world().resource::<ObservedCombatEvents>().0.len();
        rotation
            .drive(party.app.world_mut(), healer, &party.slots, Some(enemy))
            .unwrap();
        let events = &party.app.world().resource::<ObservedCombatEvents>().0[before..];
        let kicked = events.iter().any(|event| {
            event.kind == CombatLogKind::CastSuccess
                && matches!(event.spell_id, Some(107428 | 467307))
                && event.source == Some(healer.to_bits())
        });
        assert_eq!(kicked, should_kick, "unexpected kick decision: {events:?}");
        let still_channeling = party
            .app
            .world()
            .get::<CastState>(healer)
            .is_some_and(|cast| cast.spell_id == 115175);
        assert_eq!(still_channeling, !should_kick);
    }
}
