use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, Connected, PeerId, RemoteId};
use shared::protocol::{GroupMessageCode, GroupRoleSnapshot, ReadyCheckAnswer};

use super::*;

// --- Network harness: loopback host-client connections, as in trade_tests ---

fn setup_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<GroupInviteIntent>();
    app.register_message::<RespondGroupInvite>();
    app.register_message::<LeaveGroup>();
    app.register_message::<StartReadyCheck>();
    app.register_message::<RespondReadyCheck>();
    app.register_message::<GroupRosterSnapshot>();
    app.register_message::<GroupCommandResponse>();
    app.register_message::<GroupInvitePrompt>();
    app.register_message::<ReadyCheckUpdate>();
    app.add_plugins(GroupPlugin);
    app.add_systems(Update, collect_inbox);
    app.finish();
    app.cleanup();
    app
}

struct Client {
    player: Entity,
    conn: Entity,
}

fn spawn_player(app: &mut App, character_id: u64, name: &str) -> Entity {
    app.world_mut()
        .spawn((
            Player {
                name: name.into(),
                race: 1,
                class: 2,
                appearance: shared::components::CharacterAppearance::default(),
            },
            UnitLevel(80),
            ClientId(character_id),
        ))
        .id()
}

fn connect(app: &mut App, character_id: u64, name: &str) -> Client {
    let player = spawn_player(app, character_id, name);
    let conn = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(character_id)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated {
                account_id: character_id,
            },
            PlayerLink(player),
            Inbox::default(),
            (
                MessageSender::<GroupInviteIntent>::default(),
                MessageReceiver::<GroupInviteIntent>::default(),
                MessageSender::<RespondGroupInvite>::default(),
                MessageReceiver::<RespondGroupInvite>::default(),
                MessageSender::<LeaveGroup>::default(),
                MessageReceiver::<LeaveGroup>::default(),
                MessageSender::<StartReadyCheck>::default(),
                MessageReceiver::<StartReadyCheck>::default(),
                MessageSender::<RespondReadyCheck>::default(),
                MessageReceiver::<RespondReadyCheck>::default(),
            ),
            (
                MessageSender::<GroupRosterSnapshot>::default(),
                MessageReceiver::<GroupRosterSnapshot>::default(),
                MessageSender::<GroupCommandResponse>::default(),
                MessageReceiver::<GroupCommandResponse>::default(),
                MessageSender::<GroupInvitePrompt>::default(),
                MessageReceiver::<GroupInvitePrompt>::default(),
                MessageSender::<ReadyCheckUpdate>::default(),
                MessageReceiver::<ReadyCheckUpdate>::default(),
            ),
        ))
        .id();
    Client { player, conn }
}

fn send<M: lightyear::prelude::Message>(app: &mut App, client: &Client, msg: M) {
    app.world_mut()
        .entity_mut(client.conn)
        .get_mut::<MessageSender<M>>()
        .expect("connection has sender")
        .send::<GroupChannel>(msg);
}

/// Server-to-client messages, collected every frame: a receiver drops what a frame
/// did not read.
#[derive(Component, Default)]
struct Inbox {
    rosters: Vec<GroupRosterSnapshot>,
    responses: Vec<GroupCommandResponse>,
    prompts: Vec<GroupInvitePrompt>,
    ready_checks: Vec<ReadyCheckUpdate>,
}

fn collect_inbox(
    mut connections: Query<(
        &mut Inbox,
        &mut MessageReceiver<GroupRosterSnapshot>,
        &mut MessageReceiver<GroupCommandResponse>,
        &mut MessageReceiver<GroupInvitePrompt>,
        &mut MessageReceiver<ReadyCheckUpdate>,
    )>,
) {
    for (mut inbox, mut rosters, mut responses, mut prompts, mut ready_checks) in &mut connections {
        inbox.rosters.extend(rosters.receive());
        inbox.responses.extend(responses.receive());
        inbox.prompts.extend(prompts.receive());
        inbox.ready_checks.extend(ready_checks.receive());
    }
}

fn inbox<'a>(app: &'a mut App, client: &Client) -> Mut<'a, Inbox> {
    app.world_mut()
        .get_mut::<Inbox>(client.conn)
        .expect("connection has an inbox")
}

fn rosters(app: &mut App, client: &Client) -> Vec<GroupRosterSnapshot> {
    std::mem::take(&mut inbox(app, client).rosters)
}

fn responses(app: &mut App, client: &Client) -> Vec<GroupCommandResponse> {
    std::mem::take(&mut inbox(app, client).responses)
}

fn prompts(app: &mut App, client: &Client) -> Vec<GroupInvitePrompt> {
    std::mem::take(&mut inbox(app, client).prompts)
}

fn ready_checks(app: &mut App, client: &Client) -> Vec<ReadyCheckUpdate> {
    std::mem::take(&mut inbox(app, client).ready_checks)
}

fn pump(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

fn last_roster(app: &mut App, client: &Client) -> GroupRosterSnapshot {
    rosters(app, client)
        .pop()
        .expect("client should receive a roster")
}

fn codes(app: &mut App, client: &Client) -> Vec<GroupMessageCode> {
    responses(app, client).into_iter().map(|r| r.code).collect()
}

fn invite_and_accept(app: &mut App, leader: &Client, invitee: &Client, name: &str) {
    send(app, leader, GroupInviteIntent { name: name.into() });
    pump(app);
    send(app, invitee, RespondGroupInvite { accept: true });
    pump(app);
}

#[test]
fn invite_then_accept_sends_both_members_a_two_member_roster() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");

    send(&mut app, &alice, GroupInviteIntent { name: "bob".into() });
    pump(&mut app);
    let prompts = prompts(&mut app, &bob);
    assert_eq!(
        prompts,
        vec![GroupInvitePrompt {
            inviter_name: "Alice".into(),
            timeout_secs: 60.0,
        }]
    );
    let responses = responses(&mut app, &alice);
    assert_eq!(responses[0].code, GroupMessageCode::InvitePlayer);
    assert_eq!(
        responses[0].message,
        "You have invited Bob to join your group."
    );

    send(&mut app, &bob, RespondGroupInvite { accept: true });
    pump(&mut app);

    for client in [&alice, &bob] {
        let roster = last_roster(&mut app, client);
        assert!(!roster.is_raid);
        let names: Vec<_> = roster.members.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["Alice", "Bob"]);
        assert!(roster.members[0].is_leader);
        assert!(!roster.members[1].is_leader);
        assert!(roster.members.iter().all(|m| m.online && m.subgroup == 1));
        assert_eq!(roster.members[1].class, 2);
        assert_eq!(roster.members[1].level, 80);
        assert_eq!(roster.members[1].entity, Some(bob.player.to_bits()));
    }
    assert_eq!(codes(&mut app, &alice), [GroupMessageCode::JoinedGroup]);
}

#[test]
fn declined_invite_notifies_inviter_and_forms_no_group() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    send(&mut app, &alice, GroupInviteIntent { name: "Bob".into() });
    pump(&mut app);
    responses(&mut app, &alice);

    send(&mut app, &bob, RespondGroupInvite { accept: false });
    pump(&mut app);

    let responses = responses(&mut app, &alice);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].code, GroupMessageCode::DeclineGroup);
    assert_eq!(responses[0].message, "Bob declines your group invitation.");
    assert!(rosters(&mut app, &alice).is_empty());
    assert!(
        !app.world()
            .resource::<GroupRegistry>()
            .is_in_same_group(1, 2)
    );
}

#[test]
fn ready_check_round_trip_broadcasts_answers_and_result() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    invite_and_accept(&mut app, &alice, &bob, "Bob");
    responses(&mut app, &bob);

    send(&mut app, &alice, StartReadyCheck);
    pump(&mut app);
    let started = ready_checks(&mut app, &bob);
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].initiator_name, "Alice");
    assert!(!started[0].finished);
    assert_eq!(started[0].time_remaining_secs, 30.0);
    let answers: Vec<_> = started[0].members.iter().map(|m| m.answer).collect();
    assert_eq!(
        answers,
        [ReadyCheckAnswer::Ready, ReadyCheckAnswer::Pending]
    );
    assert_eq!(
        responses(&mut app, &bob)[0].message,
        "Alice has initiated a ready check."
    );

    send(&mut app, &bob, RespondReadyCheck { ready: true });
    pump(&mut app);
    let updates = ready_checks(&mut app, &alice);
    let last = updates.last().expect("ready check update");
    assert!(last.finished);
    assert!(
        last.members
            .iter()
            .all(|m| m.answer == ReadyCheckAnswer::Ready)
    );
    assert!(codes(&mut app, &alice).contains(&GroupMessageCode::ReadyCheckAllReady));
    let roster = last_roster(&mut app, &alice);
    assert_eq!((roster.ready_count, roster.total_count), (2, 2));
}

#[test]
fn leaving_leader_passes_leadership_and_leaver_gets_empty_roster() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    let cara = connect(&mut app, 3, "Cara");
    invite_and_accept(&mut app, &alice, &bob, "Bob");
    invite_and_accept(&mut app, &alice, &cara, "Cara");
    responses(&mut app, &bob);

    send(&mut app, &alice, LeaveGroup);
    pump(&mut app);

    assert!(last_roster(&mut app, &alice).members.is_empty());
    let roster = last_roster(&mut app, &bob);
    let leaders: Vec<_> = roster
        .members
        .iter()
        .map(|m| (m.name.as_str(), m.is_leader))
        .collect();
    assert_eq!(leaders, [("Bob", true), ("Cara", false)]);
    let bob_codes = codes(&mut app, &bob);
    assert!(bob_codes.contains(&GroupMessageCode::LeftGroup));
    assert!(bob_codes.contains(&GroupMessageCode::NewLeaderYou));
    assert!(codes(&mut app, &cara).contains(&GroupMessageCode::NewLeader));
}

#[test]
fn relogged_member_keeps_group_and_roster_reports_new_entity() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    invite_and_accept(&mut app, &alice, &bob, "Bob");
    rosters(&mut app, &alice);

    app.world_mut().despawn(bob.conn);
    app.world_mut().despawn(bob.player);
    pump(&mut app);
    let offline = last_roster(&mut app, &alice);
    assert!(!offline.members[1].online);
    assert_eq!(offline.members[1].entity, None);

    let bob_again = connect(&mut app, 2, "Bob");
    pump(&mut app);
    let roster = last_roster(&mut app, &bob_again);
    assert_eq!(roster.members.len(), 2);
    assert_eq!(roster.members[1].entity, Some(bob_again.player.to_bits()));
    assert!(
        app.world()
            .resource::<GroupRegistry>()
            .is_in_same_group(1, 2)
    );
}

// --- Registry rules ---

fn online(names: &[&str]) -> OnlineView {
    names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            (
                i as u64 + 1,
                OnlineMember {
                    entity: 100 + i as u64,
                    name: (*name).into(),
                    class: 1,
                    level: 10,
                },
            )
        })
        .collect()
}

fn notice_codes(effects: &[GroupEffect], to: CharacterId) -> Vec<GroupMessageCode> {
    effects
        .iter()
        .filter_map(|e| match e {
            GroupEffect::Notice { to: t, code, .. } if *t == to => Some(*code),
            _ => None,
        })
        .collect()
}

fn registry_with_party(view: &OnlineView, size: usize) -> GroupRegistry {
    let mut registry = GroupRegistry::default();
    registry.sync_presence(view);
    for id in 2..=size as u64 {
        let name = view[&id].name.clone();
        registry.invite(1, &name, view);
        registry.respond_invite(id, true);
    }
    registry
}

const SEVEN: [&str; 7] = ["P1", "P2", "P3", "P4", "P5", "P6", "P7"];

#[test]
fn sixth_member_is_rejected_from_a_party() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 5);
    assert_eq!(registry.group_members(1).len(), 5);

    let effects = registry.invite(1, "P6", &view);

    assert_eq!(notice_codes(&effects, 1), [GroupMessageCode::GroupFull]);
    assert!(!registry.is_in_same_group(1, 6));
}

#[test]
fn raid_accepts_sixth_member_after_conversion() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 5);

    let converted = registry.convert_to_raid(1);
    assert_eq!(
        notice_codes(&converted, 3),
        [GroupMessageCode::PartyConvertedToRaid]
    );
    registry.invite(1, "P6", &view);
    let joined = registry.respond_invite(6, true);

    assert!(registry.is_in_same_group(1, 6));
    assert_eq!(notice_codes(&joined, 6), [GroupMessageCode::RaidYouJoined]);
    let roster = registry.build_roster(1, &view).expect("raid roster");
    assert!(roster.is_raid);
    let subgroups: Vec<_> = roster.members.iter().map(|m| m.subgroup).collect();
    assert_eq!(subgroups, [1, 1, 1, 1, 1, 2]);

    registry.set_subgroup(1, "P6", 8);
    let roster = registry.build_roster(1, &view).expect("raid roster");
    assert_eq!(roster.members[5].subgroup, 8);
}

#[test]
fn only_the_leader_can_invite_kick_or_promote() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 3);

    assert_eq!(
        notice_codes(&registry.invite(2, "P4", &view), 2),
        [GroupMessageCode::NotLeader]
    );
    assert_eq!(
        notice_codes(&registry.uninvite(2, "P3"), 2),
        [GroupMessageCode::NotLeader]
    );
    let kicked = registry.uninvite(1, "P3");
    assert_eq!(notice_codes(&kicked, 3), [GroupMessageCode::UninviteYou]);
    assert!(!registry.is_in_same_group(1, 3));

    registry.promote(1, "P2");
    let roster = registry.build_roster(1, &view).expect("roster");
    assert!(roster.members.iter().any(|m| m.name == "P2" && m.is_leader));
}

#[test]
fn invite_expires_after_sixty_seconds_as_a_decline() {
    let view = online(&SEVEN);
    let mut registry = GroupRegistry::default();
    registry.sync_presence(&view);
    registry.invite(1, "P2", &view);

    assert!(registry.tick(59.0).is_empty());
    let effects = registry.tick(1.5);

    assert_eq!(notice_codes(&effects, 1), [GroupMessageCode::DeclineGroup]);
    assert!(registry.respond_invite(2, true).is_empty());
    assert!(!registry.is_in_same_group(1, 2));
}

#[test]
fn is_in_same_group_and_group_members_follow_membership() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 2);
    registry.sync_presence(&view);
    registry.invite(3, "P4", &view);
    registry.respond_invite(4, true);

    assert!(registry.is_in_same_group(1, 2));
    assert!(registry.is_in_same_group(3, 4));
    assert!(!registry.is_in_same_group(1, 3));
    assert!(!registry.is_in_same_group(1, 5));
    assert_eq!(registry.group_members(2), [1, 2]);
    assert!(registry.group_members(5).is_empty());
}

#[test]
fn group_disbands_when_no_member_is_online() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 2);

    registry.sync_presence(&online(&[]));

    assert!(!registry.is_in_same_group(1, 2));
    assert!(registry.group_members(1).is_empty());
}

#[test]
fn role_and_loot_method_show_in_roster() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 2);

    registry.set_role(2, "P2", GroupRoleSnapshot::Healer);
    assert_eq!(
        notice_codes(&registry.set_role(2, "P1", GroupRoleSnapshot::Tank), 2),
        [GroupMessageCode::NotLeader]
    );
    let loot = registry.set_loot_method(1, shared::loot::LootMode::RoundRobin);

    assert_eq!(
        notice_codes(&loot, 2),
        [GroupMessageCode::SetLootRoundRobin]
    );
    let roster = registry.build_roster(1, &view).expect("roster");
    assert_eq!(roster.members[1].role, GroupRoleSnapshot::Healer);
    assert_eq!(roster.loot_method, shared::loot::LootMode::RoundRobin);
}
