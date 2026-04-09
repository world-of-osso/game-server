use std::collections::BTreeMap;

use bevy::prelude::*;
use lightyear::prelude::{Message, MessageReceiver, MessageSender};
use shared::components::Player;
use shared::protocol::{
    CalendarChannel, CalendarEventSnapshot, CalendarSignupSnapshot, CalendarSignupStatusSnapshot,
    CalendarSnapshot, CalendarStateUpdate, QueryCalendar, RespondCalendarSignup,
    ScheduleCalendarEvent,
};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;

#[derive(Resource, Debug, Default)]
struct CalendarState {
    next_event_id: u64,
    events: Vec<CalendarEventState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CalendarEventState {
    event_id: u64,
    title: String,
    organizer_name: String,
    starts_at_unix_secs: u64,
    max_signups: u8,
    is_raid: bool,
    signups: BTreeMap<String, CalendarSignupStatusSnapshot>,
}

pub struct CalendarPlugin;

impl Plugin for CalendarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CalendarState>();
        app.add_systems(
            Update,
            (
                send_initial_calendar_snapshot,
                process_query_requests,
                process_calendar_requests,
            )
                .chain(),
        );
    }
}

type NewInWorldConnections<'w, 's> =
    Query<'w, 's, Entity, (With<Authenticated>, With<InWorld>, Added<InWorld>)>;

type ActiveConnections<'w, 's> = Query<'w, 's, Entity, (With<Authenticated>, With<InWorld>)>;

type CalendarSenders<'w, 's> = Query<'w, 's, &'static mut MessageSender<CalendarStateUpdate>>;

type Players<'w, 's> = Query<'w, 's, &'static Player>;

type QueryReceivers<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static mut MessageReceiver<QueryCalendar>),
    (With<Authenticated>, With<InWorld>),
>;

type ScheduleReceivers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerLink,
        &'static mut MessageReceiver<ScheduleCalendarEvent>,
    ),
    (With<Authenticated>, With<InWorld>),
>;

type SignupReceivers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerLink,
        &'static mut MessageReceiver<RespondCalendarSignup>,
    ),
    (With<Authenticated>, With<InWorld>),
>;

type PlayerRequestReceivers<'w, 's, T> = Query<
    'w,
    's,
    (Entity, &'static PlayerLink, &'static mut MessageReceiver<T>),
    (With<Authenticated>, With<InWorld>),
>;

fn send_initial_calendar_snapshot(
    connections: NewInWorldConnections,
    state: Res<CalendarState>,
    mut senders: CalendarSenders,
) {
    let snapshot = build_calendar_snapshot(&state);
    for conn_entity in &connections {
        send_calendar_update(
            conn_entity,
            snapshot.clone(),
            Some("calendar loaded".into()),
            None,
            &mut senders,
        );
    }
}

fn process_query_requests(
    mut receivers: QueryReceivers,
    state: Res<CalendarState>,
    mut senders: CalendarSenders,
) {
    let snapshot = build_calendar_snapshot(&state);
    for (conn_entity, mut receiver) in &mut receivers {
        let mut saw_request = false;
        for _ in receiver.receive() {
            saw_request = true;
        }
        if !saw_request {
            continue;
        }
        send_calendar_update(
            conn_entity,
            snapshot.clone(),
            Some("calendar refreshed".into()),
            None,
            &mut senders,
        );
    }
}

fn process_calendar_requests(
    mut state: ResMut<CalendarState>,
    mut schedule_receivers: ScheduleReceivers,
    mut signup_receivers: SignupReceivers,
    players: Players,
    connections: ActiveConnections,
    mut senders: CalendarSenders,
) {
    process_player_requests(
        &mut state,
        &mut schedule_receivers,
        &players,
        &connections,
        &mut senders,
        schedule_event,
    );
    process_player_requests(
        &mut state,
        &mut signup_receivers,
        &players,
        &connections,
        &mut senders,
        update_signup,
    );
}

fn process_player_requests<T: Message>(
    state: &mut CalendarState,
    receivers: &mut PlayerRequestReceivers<T>,
    players: &Players,
    connections: &ActiveConnections,
    senders: &mut CalendarSenders,
    mut apply_request: impl FnMut(&mut CalendarState, &str, T) -> Result<String, String>,
) {
    for (conn_entity, link, mut receiver) in receivers.iter_mut() {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Ok(player) = players.get(link.0) else {
            send_calendar_error(
                conn_entity,
                state,
                "calendar unavailable for player",
                senders,
            );
            continue;
        };
        process_player_request_batch(
            state,
            conn_entity,
            &player.name,
            requests,
            connections,
            senders,
            &mut apply_request,
        );
    }
}

fn process_player_request_batch<T: Message>(
    state: &mut CalendarState,
    conn_entity: Entity,
    player_name: &str,
    requests: Vec<T>,
    connections: &ActiveConnections,
    senders: &mut CalendarSenders,
    apply_request: &mut impl FnMut(&mut CalendarState, &str, T) -> Result<String, String>,
) {
    for request in requests {
        match apply_request(state, player_name, request) {
            Ok(message) => {
                broadcast_calendar_update(connections, state, Some(message), None, senders);
            }
            Err(error) => {
                send_calendar_error(conn_entity, state, &error, senders);
            }
        }
    }
}

fn schedule_event(
    state: &mut CalendarState,
    organizer_name: &str,
    request: ScheduleCalendarEvent,
) -> Result<String, String> {
    let title = request.title.trim();
    if title.is_empty() {
        return Err("calendar event title is required".into());
    }
    if request.max_signups == 0 {
        return Err("calendar event max signups must be greater than zero".into());
    }
    let event_id = next_event_id(state);
    let mut signups = BTreeMap::new();
    signups.insert(
        organizer_name.to_string(),
        CalendarSignupStatusSnapshot::Confirmed,
    );
    state.events.push(CalendarEventState {
        event_id,
        title: title.to_string(),
        organizer_name: organizer_name.to_string(),
        starts_at_unix_secs: request.starts_at_unix_secs,
        max_signups: request.max_signups,
        is_raid: request.is_raid,
        signups,
    });
    Ok(format!("calendar event scheduled: {title}"))
}

fn update_signup(
    state: &mut CalendarState,
    character_name: &str,
    request: RespondCalendarSignup,
) -> Result<String, String> {
    let Some(event) = state
        .events
        .iter_mut()
        .find(|event| event.event_id == request.event_id)
    else {
        return Err(format!("calendar event not found: {}", request.event_id));
    };
    let already_confirmed = event
        .signups
        .get(character_name)
        .is_some_and(|status| *status == CalendarSignupStatusSnapshot::Confirmed);
    if request.status == CalendarSignupStatusSnapshot::Confirmed
        && !already_confirmed
        && confirmed_signup_count(event) >= event.max_signups as usize
    {
        return Err(format!("calendar event is full: {}", event.title));
    }
    event
        .signups
        .insert(character_name.to_string(), request.status);
    Ok(format!("calendar signup updated: {}", event.title))
}

fn next_event_id(state: &mut CalendarState) -> u64 {
    let next = state.next_event_id.max(1);
    state.next_event_id = next + 1;
    next
}

fn confirmed_signup_count(event: &CalendarEventState) -> usize {
    event
        .signups
        .values()
        .filter(|status| **status == CalendarSignupStatusSnapshot::Confirmed)
        .count()
}

fn build_calendar_snapshot(state: &CalendarState) -> CalendarSnapshot {
    let mut events: Vec<_> = state
        .events
        .iter()
        .map(|event| CalendarEventSnapshot {
            event_id: event.event_id,
            title: event.title.clone(),
            organizer_name: event.organizer_name.clone(),
            starts_at_unix_secs: event.starts_at_unix_secs,
            max_signups: event.max_signups,
            is_raid: event.is_raid,
            signups: event
                .signups
                .iter()
                .map(|(character_name, status)| CalendarSignupSnapshot {
                    character_name: character_name.clone(),
                    status: *status,
                })
                .collect(),
        })
        .collect();
    events.sort_by_key(|event| (event.starts_at_unix_secs, event.event_id));
    CalendarSnapshot { events }
}

fn broadcast_calendar_update(
    connections: &ActiveConnections,
    state: &CalendarState,
    message: Option<String>,
    error: Option<String>,
    senders: &mut CalendarSenders,
) {
    let snapshot = build_calendar_snapshot(state);
    for conn_entity in connections.iter() {
        send_calendar_update(
            conn_entity,
            snapshot.clone(),
            message.clone(),
            error.clone(),
            senders,
        );
    }
}

fn send_calendar_error(
    conn_entity: Entity,
    state: &CalendarState,
    error: &str,
    senders: &mut CalendarSenders,
) {
    send_calendar_update(
        conn_entity,
        build_calendar_snapshot(state),
        None,
        Some(error.to_string()),
        senders,
    );
}

fn send_calendar_update(
    conn_entity: Entity,
    snapshot: CalendarSnapshot,
    message: Option<String>,
    error: Option<String>,
    senders: &mut CalendarSenders,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<CalendarChannel>(CalendarStateUpdate {
        snapshot: Some(snapshot),
        message,
        error,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event_request(title: &str, starts_at_unix_secs: u64) -> ScheduleCalendarEvent {
        ScheduleCalendarEvent {
            title: title.into(),
            starts_at_unix_secs,
            max_signups: 2,
            is_raid: true,
        }
    }

    #[test]
    fn schedule_event_adds_organizer_as_confirmed() {
        let mut state = CalendarState::default();

        let message = schedule_event(&mut state, "Theron", event_request("Karazhan", 200))
            .expect("event scheduled");

        assert_eq!(message, "calendar event scheduled: Karazhan");
        assert_eq!(state.events.len(), 1);
        assert_eq!(
            state.events[0].signups.get("Theron"),
            Some(&CalendarSignupStatusSnapshot::Confirmed)
        );
    }

    #[test]
    fn update_signup_rejects_confirmed_when_event_is_full() {
        let mut state = CalendarState {
            next_event_id: 2,
            events: vec![CalendarEventState {
                event_id: 1,
                title: "Karazhan".into(),
                organizer_name: "Theron".into(),
                starts_at_unix_secs: 200,
                max_signups: 2,
                is_raid: true,
                signups: BTreeMap::from([
                    ("Theron".into(), CalendarSignupStatusSnapshot::Confirmed),
                    ("Alice".into(), CalendarSignupStatusSnapshot::Confirmed),
                ]),
            }],
        };

        let error = update_signup(
            &mut state,
            "Bob",
            RespondCalendarSignup {
                event_id: 1,
                status: CalendarSignupStatusSnapshot::Confirmed,
            },
        )
        .expect_err("event should be full");

        assert_eq!(error, "calendar event is full: Karazhan");
    }

    #[test]
    fn build_calendar_snapshot_sorts_by_start_time() {
        let mut state = CalendarState::default();
        schedule_event(&mut state, "Theron", event_request("Late Raid", 300)).unwrap();
        schedule_event(&mut state, "Alice", event_request("Early Raid", 100)).unwrap();

        let snapshot = build_calendar_snapshot(&state);

        assert_eq!(snapshot.events.len(), 2);
        assert_eq!(snapshot.events[0].title, "Early Raid");
        assert_eq!(snapshot.events[1].title, "Late Raid");
    }
}
