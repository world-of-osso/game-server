use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::{CombatStatus, Health};
use shared::protocol::CombatLogKind;

fn cloud_stacks(world: &bevy::prelude::World, healer: bevy::prelude::Entity) -> u8 {
    world
        .get::<crate::auras::ActiveAuras>(healer)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 399510 && aura.caster == healer)
        })
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "requires the level-90 follower snapshot"]
fn sheilun_clouds_accumulate_in_combat_and_are_consumed_by_the_heal() {
    let mut party = super::super::healer_simc_replay::snapshot_party(270, 90, 1);
    let healer = party.healer;
    party
        .app
        .world_mut()
        .entity_mut(healer)
        .insert(CombatStatus(true));
    for _ in 0..170 {
        party.app.update();
    }
    assert_eq!(cloud_stacks(party.app.world(), healer), 1);
    for _ in 0..160 {
        party.app.update();
    }
    assert_eq!(cloud_stacks(party.app.world(), healer), 2);
    let mut health = party.app.world_mut().get_mut::<Health>(healer).unwrap();
    health.current = health.max / 2.0;
    let before = health.current;
    crate::spell_cast::creature_cast(party.app.world_mut(), healer, 399491, Some(healer), false)
        .unwrap();
    for _ in 0..80 {
        party.app.update();
    }
    assert!(party.app.world().get::<Health>(healer).unwrap().current > before);
    assert_eq!(cloud_stacks(party.app.world(), healer), 0);
    assert!(!crate::auras::has_aura(party.app.world(), healer, 399497));
}

#[test]
#[ignore = "requires the level-90 follower snapshot"]
fn two_sheilun_clouds_increase_the_heal_by_ten_percent() {
    let mut values = Vec::new();
    for keep_clouds in [false, true] {
        let mut party = super::super::healer_simc_replay::snapshot_party(270, 90, 1);
        let healer = party.healer;
        party
            .app
            .world_mut()
            .entity_mut(healer)
            .insert(CombatStatus(true));
        for _ in 0..330 {
            party.app.update();
        }
        assert_eq!(cloud_stacks(party.app.world(), healer), 2);
        if !keep_clouds {
            crate::auras::remove_auras(
                party.app.world_mut(),
                healer,
                crate::auras::RemoveReason::Consumed,
                |aura| aura.caster == healer && [399497, 399510].contains(&aura.spell_id),
            );
        }
        let mut health = party.app.world_mut().get_mut::<Health>(healer).unwrap();
        health.current = health.max / 2.0;
        let start = party.app.world().resource::<ObservedCombatEvents>().0.len();
        crate::spell_cast::creature_cast(
            party.app.world_mut(),
            healer,
            399491,
            Some(healer),
            false,
        )
        .unwrap();
        for _ in 0..80 {
            party.app.update();
        }
        let events = &party.app.world().resource::<ObservedCombatEvents>().0[start..];
        let heal = events
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(399491)
                    && event.target == Some(healer.to_bits())
            })
            .expect("Sheilun direct heal");
        values.push((heal.amount + heal.overflow) as f32 / if heal.crit { 2.0 } else { 1.0 });
    }
    assert!(
        (values[1] / values[0] - 1.10).abs() < 0.002,
        "heal amounts: {values:?}"
    );
}
