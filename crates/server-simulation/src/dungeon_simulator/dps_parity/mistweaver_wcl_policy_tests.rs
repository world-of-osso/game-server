use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::casting::CastState;
use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires the level-90 follower snapshot"]
fn mistweaver_does_not_manually_prepare_heals_with_soothing_mist() {
    for fraction in [0.50, 0.80] {
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
            !events
                .iter()
                .any(|event| event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(healer.to_bits())
                    && event.spell_id == Some(115175))
        );
        if fraction == 0.50 {
            assert_eq!(
                party.app.world().get::<CastState>(healer).unwrap().spell_id,
                124682
            );
        } else {
            assert!(
                events
                    .iter()
                    .any(|event| event.kind == CombatLogKind::CastSuccess
                        && event.source == Some(healer.to_bits())
                        && matches!(event.spell_id, Some(107428 | 467307))),
                "ready kick missing: {events:?}"
            );
        }
    }
}
