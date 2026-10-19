use super::*;
use crate::spell_cast::powers::{PowerPool, PowerPools, power_value};
use shared::components::PowerType;

#[test]
#[ignore = "RED: ordinary BG completion leaves living health and primary resources depleted"]
fn bg_end_resources_mana_fills_health_and_primary_preserves_secondary() {
    assert_completion_resets_primary(PowerType::Mana, 100.0);
}

#[test]
#[ignore = "RED: ordinary BG completion leaves living health and primary resources depleted"]
fn bg_end_resources_energy_fills_health_and_primary_preserves_secondary() {
    assert_completion_resets_primary(PowerType::Energy, 100.0);
}

#[test]
#[ignore = "RED: ordinary BG completion leaves living health and primary resources depleted"]
fn bg_end_resources_rage_fills_health_and_zeros_primary_preserves_secondary() {
    assert_completion_resets_primary(PowerType::Rage, 0.0);
}

#[test]
#[ignore = "RED: ordinary BG completion leaves living health and primary resources depleted"]
fn bg_end_resources_runic_fills_health_and_zeros_primary_preserves_secondary() {
    assert_completion_resets_primary(PowerType::RunicPower, 0.0);
}

#[test]
#[ignore = "RED: ordinary BG completion leaves living health and primary resources depleted"]
fn bg_end_resources_lunar_fills_health_and_zeros_primary_preserves_secondary() {
    assert_completion_resets_primary(PowerType::LunarPower, 0.0);
}

#[test]
#[ignore = "RED: ordinary BG completion leaves living health and primary resources depleted"]
fn bg_end_resources_focus_fills_health_but_preserves_unlisted_primary() {
    assert_completion_resets_primary(PowerType::Focus, 40.0);
}

fn seed_primary(h: &mut Harness, player: Entity, primary: PowerType) {
    let pools = if primary == PowerType::Mana {
        vec![PowerPool {
            power: PowerType::HolyPower,
            current: 2.0,
            max: 5.0,
            regen_per_sec: 0.0,
        }]
    } else {
        vec![
            PowerPool {
                power: primary,
                current: 40.0,
                max: 100.0,
                regen_per_sec: 0.0,
            },
            PowerPool {
                power: PowerType::HolyPower,
                current: 2.0,
                max: 5.0,
                regen_per_sec: 0.0,
            },
        ]
    };
    h.app.world_mut().entity_mut(player).insert((
        Health {
            current: 40.0,
            max: 100.0,
        },
        Mana {
            current: 40.0,
            max: 100.0,
        },
        PowerPools {
            order: vec![primary, PowerType::HolyPower],
            pools,
            ..Default::default()
        },
    ));
}

fn assert_resources(h: &Harness, player: Entity, primary: PowerType, health: f32, power: f32) {
    let world = h.app.world();
    assert_eq!(
        world.get::<Health>(player).unwrap().current,
        health,
        "completed accepted/current-copy participant health"
    );
    assert_eq!(
        power_value(world, player, primary).unwrap().0,
        power,
        "only source primary pool reset"
    );
    assert_eq!(
        power_value(world, player, PowerType::HolyPower).unwrap().0,
        2.0,
        "secondary pool unchanged"
    );
    if primary != PowerType::Mana {
        assert_eq!(
            world.get::<Mana>(player).unwrap().current,
            40.0,
            "non-primary mana unchanged"
        );
    }
}

fn assert_completion_resets_primary(primary: PowerType, expected_power: f32) {
    let mut fixture = node_fixture();
    for who in &fixture.entered.players[1..] {
        seed_primary(&mut fixture.entered.h, who.player, primary);
    }
    let outside = fixture.other_players[0].player;
    seed_primary(&mut fixture.entered.h, outside, primary);
    let unentered = fixture.unentered.player;
    fixture
        .entered
        .h
        .app
        .world_mut()
        .entity_mut(unentered)
        .insert(PowerPools {
            order: vec![primary, PowerType::HolyPower],
            pools: vec![PowerPool {
                power: primary,
                current: 40.0,
                max: 100.0,
                regen_per_sec: 0.0,
            }],
            ..Default::default()
        });
    let (_, outside_health) = drive_to_end(&mut fixture, 200);
    let outcome = battleground(&fixture.entered.h, &fixture.entered.players[1]).outcome;
    assert!(matches!(outcome, Some(BgOutcome::Winner(_))));
    for who in &fixture.entered.players[1..] {
        assert_resources(
            &fixture.entered.h,
            who.player,
            primary,
            100.0,
            expected_power,
        );
        assert_eq!(
            fixture
                .entered
                .h
                .app
                .world()
                .get::<CombatStatus>(who.player),
            Some(&CombatStatus(false))
        );
        assert_eq!(fixture.entered.h.map_of(who), Some(fixture.entered.copy));
        assert!(fixture.entered.h.new_worlds(who).is_empty());
    }
    assert_resources(&fixture.entered.h, outside, primary, outside_health, 40.0);
    assert_eq!(
        fixture.entered.h.app.world().get::<CombatStatus>(outside),
        Some(&CombatStatus(true))
    );
    assert_eq!(
        fixture
            .entered
            .h
            .app
            .world()
            .get::<Health>(unentered)
            .unwrap()
            .current,
        0.0
    );
    assert_eq!(
        power_value(fixture.entered.h.app.world(), unentered, primary)
            .unwrap()
            .0,
        if primary == PowerType::Mana {
            30.0
        } else {
            40.0
        }
    );
    assert_dead_participant_restored(&fixture);
    // Ended replays must not refill a participant's newly spent health/power again.
    let replay = fixture.entered.players[1].player;
    seed_primary(&mut fixture.entered.h, replay, primary);
    fixture.entered.h.advance(5);
    assert_resources(&fixture.entered.h, replay, primary, 40.0, 40.0);
}

#[test]
fn bg_end_resources_departed_participant_is_not_reset() {
    let mut fixture = node_fixture();
    let departed_index = fixture.entered.players.len() - 1;
    let departed = &fixture.entered.players[departed_index];
    let h = &mut fixture.entered.h;
    let data = crate::spell_cast::test_harness::world_db().unwrap();
    h.app
        .insert_resource(crate::spell_info::SpellStore::fixture(vec![
            data.store.get(26013).unwrap().clone(),
        ]));
    h.app
        .world_mut()
        .entity_mut(departed.connection)
        .insert(pair::<LeaveBattleground>());
    h.app
        .world_mut()
        .get_mut::<MessageSender<LeaveBattleground>>(departed.connection)
        .unwrap()
        .send::<PvpChannel>(LeaveBattleground {
            match_id: fixture.entered.match_id,
        });
    let updates = h.receive_pvp_updates(5);
    assert!(
        updates[&departed.connection]
            .iter()
            .all(|update| update.error.is_none()),
        "native departure: {updates:?}"
    );
    assert_eq!(h.new_worlds(departed).len(), 1);
    h.ack(departed);
    assert_eq!(h.map_of(departed), Some(STORMWIND));
    seed_primary(h, departed.player, PowerType::Mana);
    let departed_player = departed.player;
    drive_to_end(&mut fixture, 200);
    assert_resources(
        &fixture.entered.h,
        departed_player,
        PowerType::Mana,
        40.0,
        40.0,
    );
}
