use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires the level-90 follower snapshot"]
fn surging_totem_heals_followers_and_not_its_summoned_totem() {
    let mut party = super::super::healer_simc_replay::snapshot_party(264, 90, 1);
    let healer = party.healer;
    let follower = party.slots[0];
    let position = *party.app.world().get::<Position>(healer).unwrap();
    *party.app.world_mut().get_mut::<Position>(follower).unwrap() = position;
    let health = party.app.world_mut().get_mut::<Health>(follower).unwrap();
    let max = health.max;
    drop(health);
    party
        .app
        .world_mut()
        .get_mut::<Health>(follower)
        .unwrap()
        .current = max / 2.0;
    crate::spell_cast::creature_cast_with_targets(
        party.app.world_mut(),
        healer,
        444995,
        crate::spell_cast::CastTargets {
            unit: healer,
            primary: Some(healer),
            destination: Some([position.x, position.y, position.z]),
        },
    )
    .unwrap();
    party.app.update();
    let totem = party
        .app
        .world_mut()
        .query::<(
            bevy::prelude::Entity,
            &crate::spell_cast::summon::SummonedCreature,
        )>()
        .iter(party.app.world())
        .find(|(_, summon)| summon.owner == healer && summon.spell_id == 444995)
        .map(|(entity, _)| entity)
        .expect("Surging Totem must be summoned");
    let health = party.app.world_mut().get_mut::<Health>(totem).unwrap();
    let max = health.max;
    drop(health);
    party
        .app
        .world_mut()
        .get_mut::<Health>(totem)
        .unwrap()
        .current = max / 2.0;
    for _ in 0..80 {
        party.app.update();
    }
    let events = &party.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events.iter().any(|e| e.kind == CombatLogKind::Heal
            && e.spell_id == Some(73921)
            && e.source == Some(healer.to_bits())
            && e.target == Some(follower.to_bits())
            && e.amount > 0),
        "rain must heal the follower: {events:?}"
    );
    assert!(
        !events.iter().any(|e| e.kind == CombatLogKind::Heal
            && e.spell_id == Some(73921)
            && e.target == Some(totem.to_bits())),
        "rain must exclude summoned totems: {events:?}"
    );
}
