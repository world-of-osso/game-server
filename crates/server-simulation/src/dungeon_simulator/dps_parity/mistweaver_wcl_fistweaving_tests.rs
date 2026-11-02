use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires the level-90 follower snapshot"]
fn mistweaver_leaves_soothing_for_blackout_when_no_heal_or_rising_kick_is_ready() {
    let mut party = super::super::healer_simc_replay::snapshot_party(270, 90, 1);
    let healer = party.healer;
    let target = party.tank;
    let enemy = party.sources[0];
    let position = *party.app.world().get::<Position>(healer).unwrap();
    for unit in [target, enemy] {
        party.app.world_mut().entity_mut(unit).remove::<Position>();
        party.app.world_mut().entity_mut(unit).insert(position);
    }
    let mut health = party.app.world_mut().get_mut::<Health>(target).unwrap();
    health.current = health.max * 0.5;
    party.app.world_mut().entity_mut(healer).insert(
        crate::class_progression::KnownSpells::fixture(vec![115175, 100784]),
    );
    crate::spell_cast::creature_cast(party.app.world_mut(), healer, 115175, Some(target), false)
        .unwrap();
    for _ in 0..35 {
        party.app.update();
    }
    party
        .app
        .world_mut()
        .resource_mut::<bevy::prelude::Time<bevy::time::Real>>()
        .advance_by(std::time::Duration::from_secs(2));
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
                && event.spell_id == Some(100784)
                && event.source == Some(healer.to_bits())),
        "Blackout Kick must replace an unneeded channel: {events:?}"
    );
}
