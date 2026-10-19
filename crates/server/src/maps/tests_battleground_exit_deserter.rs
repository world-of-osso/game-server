use super::*;

#[test]
fn bg_exit_deserter_concrete_waiting_grants_authored_aura_until_expiry() {
    assert_early_exit_deserter(1018, false);
}

#[test]
fn bg_exit_deserter_random_waiting_grants_authored_aura_until_expiry() {
    assert_early_exit_deserter(32, false);
}

#[test]
fn bg_exit_deserter_concrete_in_progress_grants_authored_aura_until_expiry() {
    assert_early_exit_deserter(1018, true);
}

#[test]
fn bg_exit_deserter_random_in_progress_grants_authored_aura_until_expiry() {
    assert_early_exit_deserter(32, true);
}

#[test]
fn bg_exit_deserter_dead_participant_receives_aura_after_full_resurrection() {
    let (mut h, players, _database) = pending_battleground(1018);
    enter_all(&mut h, &players);
    assert_dead_exit_resurrects(&mut h, &players);
    assert_authored_deserter(&h, &players[0]);
}

fn enter_waiting_player(h: &mut Harness, who: &Traveler) {
    h.app
        .world_mut()
        .entity_mut(who.connection)
        .insert(pair::<LeaveBattleground>());
    let invitation = battleground(h, who).clone();
    let updates = request_port(h, who, invitation.match_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "entry: {updates:?}"
    );
    let destinations = h.new_worlds(who);
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].map_id, invitation.map_id);
    h.ack(who);
    assert!(battleground(h, who).accepted);
    assert_eq!(battleground(h, who).phase, BgPhase::Waiting);
}

fn assert_authored_deserter(h: &Harness, who: &Traveler) -> u32 {
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(DESERTER_SPELL_ID)
        .unwrap();
    let duration_ms =
        u32::try_from(spell.misc.as_ref().unwrap().duration.unwrap().duration_ms).unwrap();
    assert!(
        duration_ms > 0,
        "source aura must have a finite positive duration"
    );
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(who.player)
        .expect("successful early ordinary BG return must create native aura26013")
        .iter()
        .find(|aura| aura.spell_id == DESERTER_SPELL_ID)
        .expect("successful early ordinary BG return must grant native aura26013");
    assert_eq!(aura.caster, who.player);
    assert_eq!(aura.duration_ms, duration_ms);
    let remaining = aura.expires_at.unwrap() - crate::auras::aura_now(h.app.world());
    assert!(remaining > 0.0 && remaining <= f64::from(duration_ms) / 1000.0);
    duration_ms
}

fn assert_no_deserter(h: &Harness, who: &Traveler) {
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(who.player)
            .is_none_or(|auras| !auras.has_spell(DESERTER_SPELL_ID))
    );
}

fn assert_early_exit_deserter(queue_id: u32, in_progress: bool) {
    let (mut h, players, _database) = pending_battleground(queue_id);
    let held = &players[0];
    if in_progress {
        enter_all(&mut h, &players);
        assert_eq!(battleground(&h, held).phase, BgPhase::InProgress);
    } else {
        enter_waiting_player(&mut h, held);
    }
    let match_id = battleground(&h, held).match_id;
    assert_no_deserter(&h, held);
    assert_return_to_origin(&mut h, held, match_id);
    let duration_ms = assert_authored_deserter(&h, held);
    for other in &players[1..] {
        assert_no_deserter(&h, other);
    }
    let updates = request_queue(&mut h, held, queue_id);
    assert!(
        updates.iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("deserter"))),
        "early exit blocks actual requeue: {updates:?}"
    );
    assert_not_queued(&h, std::slice::from_ref(held));
    // Advance normal app clocks; do not remove the aura or alter its deadline.
    use_second_frames(&mut h);
    h.advance((u64::from(duration_ms).div_ceil(1000) + 1) as usize);
    assert_no_deserter(&h, held);
    let updates = request_queue(&mut h, held, queue_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "authored expiration permits requeue: {updates:?}"
    );
    let readmitted = h
        .app
        .world()
        .get::<crate::pvp::PlayerPvpState>(held.player)
        .unwrap();
    let still_queued = matches!(
        readmitted.queue,
        Some(crate::pvp::PlayerPvpQueue::Battleground { battleground_id, .. })
            if battleground_id == queue_id
    );
    let invited_again = readmitted
        .battleground
        .as_ref()
        .is_some_and(|invitation| !invitation.accepted && invitation.map_id == 2107);
    assert!(
        still_queued || invited_again,
        "eligible requeue must remain queued or receive a real matchmaking invitation: {readmitted:?}"
    );
}

#[test]
fn bg_exit_deserter_completed_match_does_not_grant_penalty() {
    let template = shared::battleground::arathi_basin();
    let mut entered = enter_source_match(&template, node_rules(template.map_id).unwrap().starts);
    let held = &entered.players[0];
    let h = &mut entered.h;
    for point in source_battleground_objects(entered.copy.map_id).0 {
        let (object, at) = objects_in(h, entered.copy, point.info.entry)[0];
        h.walk_to(held, at);
        use_object(h, held, object);
    }
    use_second_frames(h);
    h.advance(180);
    assert_eq!(battleground(h, held).phase, BgPhase::Ended);
    assert_no_deserter(h, held);
    assert_return_to_origin(h, held, entered.match_id);
    assert_no_deserter(h, held);
}

#[test]
fn bg_exit_deserter_arena_leave_does_not_grant_penalty() {
    let (mut h, players, _database) = pending_arena(25);
    enter_all(&mut h, &players);
    use_second_frames(&mut h);
    h.advance(65);
    let held = &players[0];
    let invitation = battleground(&h, held).clone();
    assert!(invitation.arena.is_some());
    assert_eq!(invitation.phase, BgPhase::InProgress);
    assert_no_deserter(&h, held);
    assert_return_to_origin(&mut h, held, invitation.match_id);
    assert_no_deserter(&h, held);
}

#[test]
fn bg_exit_deserter_declined_invitation_does_not_grant_penalty() {
    let (mut h, players, _database) = pending_battleground(1018);
    let held = &players[0];
    let invitation = battleground(&h, held).clone();
    assert!(!invitation.accepted);
    h.app
        .world_mut()
        .get_mut::<MessageSender<RespondToBattlegroundInvite>>(held.connection)
        .unwrap()
        .send::<PvpChannel>(RespondToBattlegroundInvite {
            match_id: invitation.match_id,
            accepted: false,
        });
    let updates = h.receive_pvp_updates(5);
    assert!(
        updates
            .values()
            .flatten()
            .all(|update| update.error.is_none()),
        "decline: {updates:?}"
    );
    assert!(h.new_worlds(held).is_empty());
    assert_eq!(h.map_of(held), Some(STORMWIND));
    assert_not_queued(&h, std::slice::from_ref(held));
    assert_no_deserter(&h, held);
}

#[test]
fn bg_exit_deserter_failed_combat_leave_does_not_grant_penalty() {
    let (mut h, players, _database) = pending_battleground(1018);
    enter_all(&mut h, &players);
    let held = &players[0];
    let opponent = find_opponent(&h, &players);
    let invitation = battleground(&h, held).clone();
    let copy = h.map_of(held).unwrap();
    land_nonlethal_hit(&mut h, held, opponent);
    let updates = request_leave(&mut h, held, invitation.match_id);
    assert!(
        updates.iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("combat"))),
        "combat refusal: {updates:?}"
    );
    assert!(h.new_worlds(held).is_empty());
    assert_eq!(h.map_of(held), Some(copy));
    assert!(battleground(&h, held).accepted);
    assert_no_deserter(&h, held);
}
