use super::*;
use shared::battleground::BgPhase;
use shared::protocol::LeaveBattleground;

#[test]
#[ignore = "RED: ordinary BG exit retains mount acquired inside the match"]
fn bg_exit_mount_concrete_waiting_removes_mount() {
    assert_exit_dismounts(1018, BgPhase::Waiting);
}

#[test]
#[ignore = "RED: ordinary BG exit retains mount acquired inside the match"]
fn bg_exit_mount_random_waiting_removes_mount() {
    assert_exit_dismounts(32, BgPhase::Waiting);
}

#[test]
#[ignore = "RED: ordinary BG exit retains mount acquired inside the match"]
fn bg_exit_mount_concrete_in_progress_removes_mount() {
    assert_exit_dismounts(1018, BgPhase::InProgress);
}

#[test]
#[ignore = "RED: ordinary BG exit retains mount acquired inside the match"]
fn bg_exit_mount_random_in_progress_removes_mount() {
    assert_exit_dismounts(32, BgPhase::InProgress);
}

#[test]
#[ignore = "RED: ordinary BG exit retains mount acquired inside the match"]
fn bg_exit_mount_concrete_completed_removes_mount() {
    assert_exit_dismounts(1018, BgPhase::Ended);
}

#[test]
#[ignore = "RED: ordinary BG exit retains mount acquired inside the match"]
fn bg_exit_mount_random_completed_removes_mount() {
    assert_exit_dismounts(32, BgPhase::Ended);
}

fn enter_players(h: &mut Harness, players: &[Traveler]) {
    for who in players {
        let invitation = battleground(h, who).clone();
        let updates = request_port(h, who, invitation.match_id);
        assert!(
            updates.iter().all(|update| update.error.is_none()),
            "native acceptance: {updates:?}"
        );
        let destinations = h.new_worlds(who);
        assert_eq!(destinations.len(), 1);
        assert_eq!(destinations[0].map_id, invitation.map_id);
        h.ack(who);
        assert_eq!(h.map_of(who).unwrap().map_id, invitation.map_id);
    }
}

fn request_native_leave(h: &mut Harness, who: &Traveler, match_id: u64) -> Vec<PvpStateUpdate> {
    h.app
        .world_mut()
        .entity_mut(who.connection)
        .insert(pair::<LeaveBattleground>());
    h.app
        .world_mut()
        .get_mut::<MessageSender<LeaveBattleground>>(who.connection)
        .unwrap()
        .send::<PvpChannel>(LeaveBattleground { match_id });
    h.receive_pvp_updates(5).remove(&who.connection).unwrap()
}

fn finish_source_nodes(h: &mut Harness, who: &Traveler) {
    let copy = h.map_of(who).unwrap();
    for point in source_battleground_objects(copy.map_id).0 {
        let (object, at) = objects_in(h, copy, point.info.entry)[0];
        h.walk_to(who, at);
        use_object(h, who, object);
    }
    use_second_frames(h);
    h.advance(180);
    assert_eq!(battleground(h, who).phase, BgPhase::Ended);
}

fn assert_exit_dismounts(queue_id: u32, phase: BgPhase) {
    let (mut h, players, _database) = pending_battleground(queue_id);
    let who = &players[0];
    let entrants = if phase == BgPhase::Waiting {
        &players[..1]
    } else {
        players.as_slice()
    };
    enter_players(&mut h, entrants);
    mount_source_rider(&mut h, who);
    if phase == BgPhase::Ended {
        finish_source_nodes(&mut h, who);
        assert_source_mounted(&h, who);
    }
    assert_eq!(battleground(&h, who).phase, phase);
    let match_id = battleground(&h, who).match_id;
    let unrelated = h
        .app
        .world()
        .get::<ActiveAuras>(who.player)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == UNRELATED_AURA)
        .unwrap()
        .instance_id;
    let updates = request_native_leave(&mut h, who, match_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "successful native leave: {updates:?}"
    );
    let destinations = h.new_worlds(who);
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].map_id, STORMWIND.map_id);
    h.ack(who);
    assert_eq!(h.map_of(who), Some(STORMWIND));
    let auras = h.app.world().get::<ActiveAuras>(who.player).unwrap();
    assert!(
        !auras.has_type(crate::auras::handlers::MOUNTED),
        "ordinary BG exit must remove mount acquired in the match"
    );
    assert!(!auras.has_spell(GOLDEN_GRYPHON));
    assert!(!auras.has_spell(STANDARD_GROUND_SPEED));
    assert!(h.app.world().get::<Mounted>(who.player).is_none());
    assert_eq!(
        auras
            .iter()
            .find(|aura| aura.spell_id == UNRELATED_AURA)
            .unwrap()
            .instance_id,
        unrelated
    );
    assert_not_queued(&h, std::slice::from_ref(who));
}

#[test]
fn bg_exit_mount_failed_combat_leave_retains_mount() {
    let (mut h, players, _database) = pending_battleground(1018);
    enter_players(&mut h, &players);
    let who = &players[0];
    mount_source_rider(&mut h, who);
    let team = battleground(&h, who).team;
    let opponent = players
        .iter()
        .find(|other| battleground(&h, other).team != team)
        .unwrap();
    let health = h.app.world().get::<Health>(who.player).unwrap().current;
    crate::combat::deal_direct_damage(h.app.world_mut(), opponent.player, who.player, 1.0);
    assert!(h.app.world().get::<Health>(who.player).unwrap().current < health);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::CombatStatus>(who.player),
        Some(&shared::components::CombatStatus(true))
    );
    let invitation = battleground(&h, who).clone();
    let copy = h.map_of(who);
    let updates = request_native_leave(&mut h, who, invitation.match_id);
    assert!(
        updates.iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("combat"))),
        "native combat leave refusal: {updates:?}"
    );
    assert!(h.new_worlds(who).is_empty());
    assert_eq!(h.map_of(who), copy);
    assert_source_mounted(&h, who);
}

#[test]
fn bg_exit_mount_pending_invitation_retains_mount() {
    let (mut h, players, _database) = pending_battleground(1018);
    let who = &players[0];
    mount_source_rider(&mut h, who);
    h.receive_pvp_updates(5);
    assert!(!battleground(&h, who).accepted);
    assert!(h.new_worlds(who).is_empty());
    assert_source_mounted(&h, who);
}
