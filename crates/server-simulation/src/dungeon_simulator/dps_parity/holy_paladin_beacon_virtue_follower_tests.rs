use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires the level-90 follower snapshot"]
fn beacon_of_virtue_transfers_heals_between_marked_followers() {
    let mut party = super::super::healer_simc_replay::snapshot_party(65, 90, 1);
    let healer = party.healer;
    let tank = party.tank;
    let ally = party.slots[2];
    let position = *party.app.world().get::<Position>(healer).unwrap();
    for unit in party.slots {
        super::super::healer_simc_replay::hold(&mut party, unit);
        party.app.world_mut().entity_mut(unit).remove::<Position>();
        party.app.world_mut().entity_mut(unit).insert(position);
        let mut health = party.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.current = health.max / 2.0;
    }
    let map = *party.app.world().get::<crate::maps::InMap>(healer).unwrap();
    let faction = *party
        .app
        .world()
        .get::<shared::components::UnitFactionTemplate>(healer)
        .unwrap();
    let pet = party
        .app
        .world_mut()
        .spawn((
            map,
            position,
            faction,
            Health {
                current: 500.0,
                max: 1000.0,
            },
            crate::hunter_pet::HunterPet {
                owner: healer,
                slot: 0,
            },
        ))
        .id();
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(party.app.world(), healer, 53563)
            .unwrap(),
        200025
    );
    crate::spell_cast::creature_cast(party.app.world_mut(), healer, 200025, Some(tank), true)
        .unwrap();
    party.app.update();
    for unit in [tank, ally] {
        assert!(
            party
                .app
                .world()
                .get::<crate::auras::ActiveAuras>(unit)
                .is_some_and(|auras| auras
                    .iter()
                    .any(|aura| aura.spell_id == 200025 && aura.caster == healer)),
            "Virtue must mark both followers"
        );
    }
    assert!(
        !party
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(pet)
            .is_some_and(|auras| auras.iter().any(|aura| aura.spell_id == 200025)),
        "Virtue must not mark a secondary pet"
    );
    let cursor = party.app.world().resource::<ObservedCombatEvents>().0.len();
    crate::spell_cast::cast_triggered(party.app.world_mut(), healer, 19750, Some(ally), 1);
    for _ in 0..8 {
        party.app.update();
    }
    let events = &party.app.world().resource::<ObservedCombatEvents>().0[cursor..];
    assert!(
        events.iter().any(|event| event.kind == CombatLogKind::Heal
            && event.spell_id == Some(19750)
            && event.source == Some(healer.to_bits())
            && event.target == Some(ally.to_bits())
            && event.amount > 0),
        "the ally must receive the source heal"
    );
    assert!(
        events.iter().any(|event| event.kind == CombatLogKind::Heal
            && event.spell_id == Some(53652)
            && event.source == Some(healer.to_bits())
            && event.target == Some(tank.to_bits())
            && event.amount > 0),
        "the tank must receive Beacon transfer"
    );
    assert!(
        !events.iter().any(|event| event.kind == CombatLogKind::Heal
            && event.spell_id == Some(53652)
            && event.target == Some(ally.to_bits())),
        "a source heal must not transfer to its own target"
    );
}
