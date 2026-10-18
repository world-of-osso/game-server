use super::*;
use crate::death::PlayerDeathState;
use shared::components::Mana;
use shared::death::DeathState;

#[test]
#[ignore = "RED: ordinary battleground port omits full resurrection"]
fn bg_dead_port_concrete_resurrects_after_actual_post_invitation_death() {
    assert_dead_battleground_enters_alive(1018);
}

#[test]
#[ignore = "RED: ordinary battleground port omits full resurrection"]
fn bg_dead_port_random_resurrects_after_actual_post_invitation_death() {
    assert_dead_battleground_enters_alive(32);
}

#[test]
fn bg_dead_port_arena_retains_full_resurrection() {
    let (mut h, players, _database) = pending_arena(25);
    let held = &players[0];
    let invitation = battleground(&h, held).clone();
    assert!(invitation.arena.is_some());
    kill_invited_player(&mut h, held, &players[1]);
    accept_and_assert_alive(&mut h, held, invitation.match_id, invitation.map_id);
}

#[test]
fn bg_dead_port_living_battleground_does_not_refill_health_or_mana() {
    let (mut h, players, _database) = pending_battleground(1018);
    let held = &players[0];
    let invitation = battleground(&h, held).clone();
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
    let updates = request_port(&mut h, held, invitation.match_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "living port: {updates:?}"
    );
    assert_eq!(h.new_worlds(held).len(), 1);
    assert_eq!(
        h.app.world().get::<Health>(held.player).unwrap().current,
        40.0
    );
    assert_eq!(
        h.app.world().get::<Mana>(held.player).unwrap().current,
        30.0
    );
    h.ack(held);
    assert_eq!(h.map_of(held).unwrap().map_id, invitation.map_id);
    assert_eq!(
        h.app.world().get::<Health>(held.player).unwrap().current,
        40.0
    );
    assert_eq!(
        h.app.world().get::<Mana>(held.player).unwrap().current,
        30.0
    );
}

fn assert_dead_battleground_enters_alive(queue_id: u32) {
    let (mut h, players, _database) = pending_battleground(queue_id);
    let held = &players[0];
    let invitation = battleground(&h, held).clone();
    assert!(invitation.arena.is_none() && !invitation.accepted);
    kill_invited_player(&mut h, held, &players[1]);
    assert_eq!(battleground(&h, held).match_id, invitation.match_id);
    assert!(!battleground(&h, held).accepted);
    assert!(h.new_worlds(held).is_empty());
    assert_eq!(h.map_of(held), Some(STORMWIND));
    accept_and_assert_alive(&mut h, held, invitation.match_id, invitation.map_id);
}

fn kill_invited_player(h: &mut Harness, who: &Traveler, attacker: &Traveler) {
    assert!(!battleground(h, who).accepted);
    h.app.world_mut().entity_mut(who.player).insert((
        PlayerDeathState::default(),
        Mana {
            current: 30.0,
            max: 200.0,
        },
    ));
    // Commit real lethal damage; DeathPlugin processes its normal transition.
    let damage = h.app.world().get::<Health>(who.player).unwrap().max * 2.0;
    crate::combat::deal_direct_damage(h.app.world_mut(), attacker.player, who.player, damage);
    h.advance(2);
    assert_eq!(
        h.app.world().get::<Health>(who.player).unwrap().current,
        0.0
    );
    let death = h.app.world().get::<PlayerDeathState>(who.player).unwrap();
    assert_eq!(death.state, DeathState::Dead);
    let corpse = death
        .corpse
        .as_ref()
        .expect("normal death created a corpse");
    assert_eq!(u32::from(corpse.map_id), STORMWIND.map_id);
}

fn accept_and_assert_alive(h: &mut Harness, who: &Traveler, match_id: u64, map_id: u32) {
    let updates = request_port(h, who, match_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "dead port: {updates:?}"
    );
    let destinations = h.new_worlds(who);
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].map_id, map_id);
    assert!(battleground(h, who).accepted);
    assert_alive_and_full(h, who);
    h.ack(who);
    assert_eq!(h.map_of(who).unwrap().map_id, map_id);
    assert_alive_and_full(h, who);
}

fn assert_alive_and_full(h: &Harness, who: &Traveler) {
    let health = h.app.world().get::<Health>(who.player).unwrap();
    assert_eq!(
        health.current, health.max,
        "port fully resurrects before WorldPortAck"
    );
    let mana = h.app.world().get::<Mana>(who.player).unwrap();
    assert_eq!(mana.current, mana.max);
    let death = h.app.world().get::<PlayerDeathState>(who.player).unwrap();
    assert_eq!(death.state, DeathState::Alive);
    assert!(death.corpse.is_none(), "port clears the old corpse");
    assert!(death.graveyard.is_none());
}
