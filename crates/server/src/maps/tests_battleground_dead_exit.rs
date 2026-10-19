use super::*;
use crate::death::PlayerDeathState;
use shared::components::Mana;
use shared::death::DeathState;

#[test]
#[ignore = "RED: ordinary battlefield exit does not fully resurrect"]
fn pvp_dead_exit_concrete_resurrects_and_clears_corpse_before_return() {
    let (mut h, players, _database) = pending_battleground(1018);
    enter_all(&mut h, &players);
    assert_dead_exit_resurrects(&mut h, &players);
}

#[test]
#[ignore = "RED: ordinary battlefield exit does not fully resurrect"]
fn pvp_dead_exit_random_resurrects_and_clears_corpse_before_return() {
    let (mut h, players, _database) = pending_battleground(32);
    enter_all(&mut h, &players);
    assert_dead_exit_resurrects(&mut h, &players);
}

#[test]
#[ignore = "RED: ordinary battlefield exit does not fully resurrect"]
fn pvp_dead_exit_surviving_arena_resurrects_and_clears_corpse_before_return() {
    let (mut h, players, _database) = pending_arena(25);
    enter_all(&mut h, &players);
    use_second_frames(&mut h);
    h.advance(65);
    assert_dead_exit_resurrects(&mut h, &players);
}

#[test]
fn pvp_dead_exit_living_injured_participant_is_not_healed() {
    let (mut h, players, _database) = pending_battleground(1018);
    enter_all(&mut h, &players);
    let held = &players[0];
    let match_id = battleground(&h, held).match_id;
    h.app.world_mut().entity_mut(held.player).insert((
        PlayerDeathState::default(),
        Health {
            current: 40.0,
            max: 100.0,
        },
        Mana {
            current: 30.0,
            max: 200.0,
        },
    ));
    assert_leave_starts_return(&mut h, held, match_id);
    assert_injured(&h, held);
    h.ack(held);
    assert_eq!(h.map_of(held), Some(STORMWIND));
    assert_injured(&h, held);
}

fn assert_dead_exit_resurrects(h: &mut Harness, players: &[Traveler]) {
    let held = &players[0];
    let opponent = find_opponent(h, players);
    let match_id = battleground(h, held).match_id;
    let copy = h.map_of(held).unwrap();
    assert_eq!(battleground(h, held).phase, BgPhase::InProgress);
    h.app.world_mut().entity_mut(held.player).insert((
        PlayerDeathState::default(),
        Mana {
            current: 30.0,
            max: 200.0,
        },
    ));
    let damage = h.app.world().get::<Health>(held.player).unwrap().max * 2.0;
    crate::combat::deal_direct_damage(h.app.world_mut(), opponent.player, held.player, damage);
    h.advance(2);
    assert_eq!(
        h.app.world().get::<Health>(held.player).unwrap().current,
        0.0
    );
    let death = h.app.world().get::<PlayerDeathState>(held.player).unwrap();
    assert_eq!(death.state, DeathState::Dead);
    assert_eq!(
        u32::from(death.corpse.as_ref().expect("real death corpse").map_id),
        copy.map_id
    );
    assert_eq!(h.map_of(held), Some(copy));
    assert_eq!(
        battleground(h, held).phase,
        BgPhase::InProgress,
        "death did not end this match"
    );
    assert_eq!(
        h.app.world().get::<CombatStatus>(held.player),
        Some(&CombatStatus(false)),
        "normal death stopped combat"
    );
    assert!(h.new_worlds(held).is_empty());
    assert_leave_starts_return(h, held, match_id);
    assert_alive_and_full(h, held);
    h.ack(held);
    assert_eq!(h.map_of(held), Some(STORMWIND));
    assert_alive_and_full(h, held);
    assert!(players[1..].iter().all(|who| h.map_of(who) == Some(copy)));
}

fn assert_leave_starts_return(h: &mut Harness, held: &Traveler, match_id: u64) {
    let updates = request_leave(h, held, match_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "ordinary exit: {updates:?}"
    );
    let destinations = h.new_worlds(held);
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].map_id, STORMWIND.map_id);
    assert!(
        h.app
            .world()
            .get::<crate::pvp::PlayerPvpState>(held.player)
            .unwrap()
            .battleground
            .is_none()
    );
}

fn assert_alive_and_full(h: &Harness, held: &Traveler) {
    let health = h.app.world().get::<Health>(held.player).unwrap();
    assert_eq!(
        health.current, health.max,
        "exit fully resurrects before return WorldPortAck"
    );
    let mana = h.app.world().get::<Mana>(held.player).unwrap();
    assert_eq!(mana.current, mana.max);
    let death = h.app.world().get::<PlayerDeathState>(held.player).unwrap();
    assert_eq!(death.state, DeathState::Alive);
    assert!(death.corpse.is_none());
    assert!(death.graveyard.is_none());
}

fn assert_injured(h: &Harness, held: &Traveler) {
    assert_eq!(
        h.app.world().get::<Health>(held.player).unwrap().current,
        40.0
    );
    assert_eq!(
        h.app.world().get::<Mana>(held.player).unwrap().current,
        30.0
    );
    assert_eq!(
        h.app
            .world()
            .get::<PlayerDeathState>(held.player)
            .unwrap()
            .state,
        DeathState::Alive
    );
}
