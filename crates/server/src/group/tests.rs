#[path = "follower_tests.rs"]
mod follower_tests;

use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, Connected, PeerId, RemoteId};
use shared::components::{
    AuraView, Health, Position, PowerEntry, PowerType, UnitAuras, UnitPowers,
};
use shared::death::DeathState;
use shared::protocol::{
    GroupAssistants, GroupCountdownRequest, GroupCountdownUpdate, GroupMemberState,
    GroupMemberStates, GroupMessageCode, GroupRoleSnapshot, MINIMAP_PING_MAP_HALFSIZE, MinimapPing,
    MinimapPingRequest, RAID_TARGET_SKULL, RaidTargetIcons, ReadyCheckAnswer, SetGroupAssistant,
    SetRaidTarget,
};

use crate::death::PlayerDeathState;

use super::*;

// --- Network harness: loopback host-client connections, as in trade_tests ---

fn setup_app() -> App {
    setup_app_with(|_| {})
}

fn setup_app_with(extra: impl FnOnce(&mut App)) -> App {
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
    app.register_message::<GroupInviteCancelled>();
    app.register_message::<GroupMemberStates>();
    app.register_message::<SetRaidTarget>();
    app.register_message::<RaidTargetIcons>();
    app.register_message::<ConvertGroupToRaid>();
    app.register_message::<MinimapPingRequest>();
    app.register_message::<MinimapPing>();
    app.register_message::<GroupCountdownRequest>();
    app.register_message::<GroupCountdownUpdate>();
    app.register_message::<SetGroupAssistant>();
    app.register_message::<GroupAssistants>();
    app.add_plugins(GroupPlugin);
    extra(&mut app);
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
            Health {
                current: 1000.0,
                max: 1000.0,
            },
            Position {
                x: character_id as f32,
                y: 0.0,
                z: 0.0,
            },
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
            (
                MessageSender::<GroupInviteCancelled>::default(),
                MessageReceiver::<GroupInviteCancelled>::default(),
                MessageSender::<GroupMemberStates>::default(),
                MessageReceiver::<GroupMemberStates>::default(),
            ),
            (
                MessageSender::<SetRaidTarget>::default(),
                MessageReceiver::<SetRaidTarget>::default(),
                MessageSender::<RaidTargetIcons>::default(),
                MessageReceiver::<RaidTargetIcons>::default(),
                MessageSender::<ConvertGroupToRaid>::default(),
                MessageReceiver::<ConvertGroupToRaid>::default(),
            ),
            (
                MessageSender::<MinimapPingRequest>::default(),
                MessageReceiver::<MinimapPingRequest>::default(),
                MessageSender::<MinimapPing>::default(),
                MessageReceiver::<MinimapPing>::default(),
                MessageSender::<GroupCountdownRequest>::default(),
                MessageReceiver::<GroupCountdownRequest>::default(),
                MessageSender::<GroupCountdownUpdate>::default(),
                MessageReceiver::<GroupCountdownUpdate>::default(),
                MessageSender::<SetGroupAssistant>::default(),
                MessageReceiver::<SetGroupAssistant>::default(),
                MessageSender::<GroupAssistants>::default(),
                MessageReceiver::<GroupAssistants>::default(),
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
    cancelled_invites: Vec<GroupInviteCancelled>,
    member_states: Vec<GroupMemberStates>,
    raid_targets: Vec<RaidTargetIcons>,
    minimap_pings: Vec<MinimapPing>,
    countdowns: Vec<GroupCountdownUpdate>,
    assistants: Vec<GroupAssistants>,
    inventory_errors: Vec<shared::protocol::InventoryError>,
}

fn collect_inbox(
    mut connections: Query<(
        &mut Inbox,
        &mut MessageReceiver<GroupRosterSnapshot>,
        &mut MessageReceiver<GroupCommandResponse>,
        &mut MessageReceiver<GroupInvitePrompt>,
        &mut MessageReceiver<ReadyCheckUpdate>,
        &mut MessageReceiver<GroupInviteCancelled>,
        &mut MessageReceiver<GroupMemberStates>,
        &mut MessageReceiver<RaidTargetIcons>,
        &mut MessageReceiver<MinimapPing>,
        &mut MessageReceiver<GroupCountdownUpdate>,
        &mut MessageReceiver<GroupAssistants>,
    )>,
) {
    for (
        mut inbox,
        mut rosters,
        mut responses,
        mut prompts,
        mut ready_checks,
        mut cancels,
        mut states,
        mut raid_targets,
        mut minimap_pings,
        mut countdowns,
        mut assistants,
    ) in &mut connections
    {
        inbox.rosters.extend(rosters.receive());
        inbox.responses.extend(responses.receive());
        inbox.prompts.extend(prompts.receive());
        inbox.ready_checks.extend(ready_checks.receive());
        inbox.cancelled_invites.extend(cancels.receive());
        inbox.member_states.extend(states.receive());
        inbox.raid_targets.extend(raid_targets.receive());
        inbox.minimap_pings.extend(minimap_pings.receive());
        inbox.countdowns.extend(countdowns.receive());
        inbox.assistants.extend(assistants.receive());
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

fn member_states(app: &mut App, client: &Client) -> Vec<GroupMemberStates> {
    std::mem::take(&mut inbox(app, client).member_states)
}

fn raid_targets(app: &mut App, client: &Client) -> Vec<RaidTargetIcons> {
    std::mem::take(&mut inbox(app, client).raid_targets)
}

fn minimap_pings(app: &mut App, client: &Client) -> Vec<MinimapPing> {
    std::mem::take(&mut inbox(app, client).minimap_pings)
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

fn countdowns(app: &mut App, client: &Client) -> Vec<GroupCountdownUpdate> {
    std::mem::take(&mut inbox(app, client).countdowns)
}

#[test]
fn countdown_broadcasts_start_replacement_cancel_to_three_group_members_only() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    let carol = connect(&mut app, 3, "Carol");
    let outsider = connect(&mut app, 4, "Outsider");
    invite_and_accept(&mut app, &alice, &bob, "Bob");
    invite_and_accept(&mut app, &alice, &carol, "Carol");
    for seconds in [10, 20, 0] {
        send(&mut app, &alice, GroupCountdownRequest { seconds });
        pump(&mut app);
        for client in [&alice, &bob, &carol] {
            let updates = countdowns(&mut app, client);
            assert_eq!(updates.len(), 1);
            assert_eq!(updates[0].initiator_name, "Alice");
            assert_eq!(updates[0].duration_secs, seconds);
            assert_eq!(updates[0].time_remaining_secs, seconds as f32);
        }
        assert!(countdowns(&mut app, &outsider).is_empty());
    }
}

#[test]
fn countdown_rejects_ordinary_member_and_solo_player_requests() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    let outsider = connect(&mut app, 3, "Outsider");
    invite_and_accept(&mut app, &alice, &bob, "Bob");
    for client in [&bob, &outsider] {
        send(&mut app, client, GroupCountdownRequest { seconds: 10 });
        pump(&mut app);
        assert!(countdowns(&mut app, &alice).is_empty());
        assert!(countdowns(&mut app, &bob).is_empty());
    }
    assert!(codes(&mut app, &bob).contains(&GroupMessageCode::NotLeader));
    assert!(codes(&mut app, &outsider).contains(&GroupMessageCode::NotInGroup));
}

#[test]
fn countdown_raid_assistant_can_start_readycheck_and_countdown_until_revoked() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    invite_and_accept(&mut app, &alice, &bob, "Bob");
    send(&mut app, &alice, ConvertGroupToRaid);
    pump(&mut app);
    send(
        &mut app,
        &alice,
        SetGroupAssistant {
            name: "Bob".into(),
            assistant: true,
        },
    );
    pump(&mut app);
    for client in [&alice, &bob] {
        assert_eq!(
            inbox(&mut app, client).assistants.last().unwrap().members,
            ["Bob"]
        );
    }
    send(&mut app, &bob, StartReadyCheck);
    send(&mut app, &bob, GroupCountdownRequest { seconds: 10 });
    pump(&mut app);
    assert_eq!(
        ready_checks(&mut app, &alice)
            .last()
            .unwrap()
            .initiator_name,
        "Bob"
    );
    assert_eq!(
        countdowns(&mut app, &alice).last().unwrap().initiator_name,
        "Bob"
    );
    send(
        &mut app,
        &bob,
        SetGroupAssistant {
            name: "Alice".into(),
            assistant: true,
        },
    );
    pump(&mut app);
    assert!(codes(&mut app, &bob).contains(&GroupMessageCode::NotLeader));
    send(
        &mut app,
        &alice,
        SetGroupAssistant {
            name: "Bob".into(),
            assistant: false,
        },
    );
    pump(&mut app);
    assert!(
        inbox(&mut app, &bob)
            .assistants
            .last()
            .unwrap()
            .members
            .is_empty()
    );
    countdowns(&mut app, &bob);
    send(&mut app, &bob, GroupCountdownRequest { seconds: 0 });
    pump(&mut app);
    assert!(countdowns(&mut app, &bob).is_empty());
    assert!(codes(&mut app, &bob).contains(&GroupMessageCode::NotLeader));
}

#[test]
fn countdown_server_clock_expires_once_for_every_member() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    invite_and_accept(&mut app, &alice, &bob, "Bob");
    send(&mut app, &alice, GroupCountdownRequest { seconds: 10 });
    pump(&mut app);
    let deadline = countdowns(&mut app, &alice).pop().unwrap().deadline_unix_ms;
    countdowns(&mut app, &bob);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_secs(4),
    ));
    app.world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(Duration::from_secs(4));
    pump(&mut app);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::ZERO,
    ));
    pump(&mut app);
    for client in [&alice, &bob] {
        assert_eq!(
            countdowns(&mut app, client),
            vec![GroupCountdownUpdate {
                deadline_unix_ms: deadline,
                initiator_name: "Alice".into(),
                duration_secs: 10,
                time_remaining_secs: 0.0,
            }]
        );
    }
    pump(&mut app);
    assert!(countdowns(&mut app, &alice).is_empty());
    assert!(countdowns(&mut app, &bob).is_empty());
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
                    portrait: Default::default(),
                    dungeon_difficulty: crate::maps::store::DIFFICULTY_NORMAL,
                    raid_difficulty: crate::maps::RaidDifficulty::default(),
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
    let loot = registry.set_loot_method(1, shared::loot::LootMode::PersonalLoot);

    assert_eq!(notice_codes(&loot, 2), [GroupMessageCode::SetLootPersonal]);
    let roster = registry.build_roster(1, &view).expect("roster");
    assert_eq!(roster.members[1].role, GroupRoleSnapshot::Healer);
    assert_eq!(roster.loot_method, shared::loot::LootMode::PersonalLoot);
}

#[test]
fn mainline_loot_method_and_quality_threshold_are_leader_owned() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 2);
    assert_eq!(
        notice_codes(
            &registry.set_loot_method(2, shared::loot::LootMode::PersonalLoot),
            2
        ),
        [GroupMessageCode::NotLeader]
    );
    assert_eq!(
        notice_codes(&registry.set_loot_threshold(2, 4), 2),
        [GroupMessageCode::NotLeader]
    );
    for mode in [
        shared::loot::LootMode::FreeForAll,
        shared::loot::LootMode::RoundRobin,
        shared::loot::LootMode::NeedBeforeGreed,
    ] {
        assert_eq!(
            notice_codes(&registry.set_loot_method(1, mode), 1),
            [GroupMessageCode::LootMethodUnavailable]
        );
    }
    registry.set_loot_method(1, shared::loot::LootMode::GroupLoot);
    registry.set_loot_threshold(1, 4);
    assert_eq!(
        registry.loot_policy_of(2),
        Some((shared::loot::LootMode::GroupLoot, 4))
    );
    registry.set_loot_threshold(1, 1);
    assert_eq!(
        registry.loot_policy_of(2),
        Some((shared::loot::LootMode::GroupLoot, 4))
    );
}

fn cancelled_invitees(effects: &[GroupEffect]) -> Vec<(CharacterId, String)> {
    effects
        .iter()
        .filter_map(|e| match e {
            GroupEffect::InviteCancelled { to, inviter_name } => Some((*to, inviter_name.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn invite_from_a_player_who_joins_another_group_is_cancelled() {
    let view = online(&SEVEN);
    let mut registry = GroupRegistry::default();
    registry.sync_presence(&view);
    registry.invite(1, "P2", &view);
    registry.invite(3, "P1", &view);

    let joined = registry.respond_invite(1, true);

    assert_eq!(cancelled_invitees(&joined), [(2, "P1".to_string())]);
    assert!(registry.respond_invite(2, true).is_empty());
    assert!(!registry.is_in_same_group(2, 3));
    assert_eq!(registry.group_members(3), [3, 1]);
}

#[test]
fn invite_from_a_leader_who_leaves_is_cancelled() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 3);
    registry.invite(1, "P4", &view);

    let left = registry.leave(1);

    assert_eq!(cancelled_invitees(&left), [(4, "P1".to_string())]);
    assert!(registry.respond_invite(4, true).is_empty());
    assert!(!registry.is_in_same_group(2, 4));
    assert!(!registry.is_in_same_group(1, 4));
}

#[test]
fn invite_from_a_leader_who_promotes_someone_else_is_cancelled() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 2);
    registry.invite(1, "P3", &view);

    let promoted = registry.promote(1, "P2");

    assert_eq!(cancelled_invitees(&promoted), [(3, "P1".to_string())]);
    assert!(registry.respond_invite(3, true).is_empty());
    assert!(!registry.is_in_same_group(1, 3));
}

#[test]
fn expired_invite_closes_the_invitees_popup() {
    let view = online(&SEVEN);
    let mut registry = GroupRegistry::default();
    registry.sync_presence(&view);
    registry.invite(1, "P2", &view);

    let effects = registry.tick(61.0);

    assert_eq!(cancelled_invitees(&effects), [(2, "P1".to_string())]);
}

#[test]
fn invite_from_a_player_who_logs_out_is_cancelled() {
    let view = online(&SEVEN);
    let mut registry = GroupRegistry::default();
    registry.sync_presence(&view);
    registry.invite(1, "P2", &view);

    let mut inviter_gone = view.clone();
    inviter_gone.remove(&1);
    let effects = registry.sync_presence(&inviter_gone);

    assert_eq!(cancelled_invitees(&effects), [(2, "P1".to_string())]);
    assert!(registry.respond_invite(2, true).is_empty());
}

#[test]
fn raid_of_five_converts_back_to_a_party_in_one_subgroup() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 3);
    registry.convert_to_raid(1);
    registry.set_subgroup(1, "P2", 4);
    registry.set_subgroup(1, "P3", 8);

    let effects = registry.convert_to_party(1);

    assert_eq!(
        notice_codes(&effects, 2),
        [GroupMessageCode::RaidConvertedToParty]
    );
    let roster = registry.build_roster(1, &view).expect("party roster");
    assert!(!roster.is_raid);
    assert!(roster.members.iter().all(|m| m.subgroup == 1));
    assert_eq!(
        notice_codes(&registry.invite(1, "P4", &view), 1),
        [GroupMessageCode::InvitePlayer]
    );
}

#[test]
fn raid_of_six_cannot_convert_to_a_party() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 5);
    registry.convert_to_raid(1);
    registry.invite(1, "P6", &view);
    registry.respond_invite(6, true);

    let effects = registry.convert_to_party(1);

    assert_eq!(notice_codes(&effects, 1), [GroupMessageCode::GroupFull]);
    assert!(registry.build_roster(1, &view).expect("roster").is_raid);
}

#[test]
fn convert_to_party_needs_a_raid_and_the_leader() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 2);
    assert_eq!(
        notice_codes(&registry.convert_to_party(1), 1),
        [GroupMessageCode::NotInRaid]
    );
    registry.convert_to_raid(1);
    assert_eq!(
        notice_codes(&registry.convert_to_party(2), 2),
        [GroupMessageCode::NotLeader]
    );
}

#[test]
fn invite_rejections_for_self_and_busy_ungrouped_inviter() {
    let view = online(&SEVEN);
    let mut registry = GroupRegistry::default();
    registry.sync_presence(&view);

    assert_eq!(
        notice_codes(&registry.invite(1, "p1", &view), 1),
        [GroupMessageCode::InviteSelf]
    );
    registry.invite(1, "P2", &view);
    assert_eq!(
        notice_codes(&registry.invite(1, "P3", &view), 1),
        [GroupMessageCode::InvitePartyBusy]
    );
    assert_eq!(
        notice_codes(&registry.invite(3, "P2", &view), 3),
        [GroupMessageCode::AlreadyInGroup]
    );
    assert_eq!(
        notice_codes(&registry.invite(1, "Nobody", &view), 1),
        [GroupMessageCode::BadPlayerName]
    );
}

#[test]
fn moving_a_party_member_to_a_subgroup_needs_a_raid() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 2);

    assert_eq!(
        notice_codes(&registry.set_subgroup(1, "P2", 3), 1),
        [GroupMessageCode::NotInRaid]
    );
}

#[test]
fn second_ready_check_is_rejected_and_timeout_finishes_with_away_members() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 3);
    registry.start_ready_check(1, &view);
    registry.respond_ready_check(2, false);

    assert_eq!(
        notice_codes(&registry.start_ready_check(1, &view), 1),
        [GroupMessageCode::ReadyCheckInProgress]
    );
    let effects = registry.tick(31.0);

    assert_eq!(
        notice_codes(&effects, 3),
        [GroupMessageCode::ReadyCheckFinished]
    );
    let group_id = 1;
    let (_, update) = registry.build_ready_check(group_id).expect("ready check");
    assert!(update.finished);
    let answers: Vec<_> = update
        .members
        .iter()
        .map(|m| (m.name.as_str(), m.answer))
        .collect();
    assert_eq!(
        answers,
        [
            ("P1", ReadyCheckAnswer::Ready),
            ("P2", ReadyCheckAnswer::NotReady),
            ("P3", ReadyCheckAnswer::Pending),
        ]
    );
}

#[test]
fn level_up_resends_the_roster() {
    let view = online(&SEVEN);
    let mut registry = registry_with_party(&view, 2);
    registry.build_roster(1, &view);
    assert!(registry.sync_presence(&view).is_empty());

    let mut leveled = view.clone();
    leveled.get_mut(&2).expect("P2 online").level = 11;
    let effects = registry.sync_presence(&leveled);

    assert_eq!(effects, [GroupEffect::Roster(1)]);
    let roster = registry.build_roster(1, &leveled).expect("roster");
    assert_eq!(roster.members[1].level, 11);
}

// --- Live member state ---

fn member_names(update: &GroupMemberStates) -> Vec<&str> {
    update.members.iter().map(|m| m.name.as_str()).collect()
}

#[test]
fn member_states_carry_live_health_power_death_and_debuffs_for_every_member() {
    let mut app = setup_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(100),
    ));
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    let cara = connect(&mut app, 3, "Cara");
    invite_and_accept(&mut app, &alice, &bob, "Bob");
    pump(&mut app);
    assert!(
        member_states(&mut app, &cara).is_empty(),
        "ungrouped get none"
    );
    let first = member_states(&mut app, &alice);
    let everyone: Vec<_> = first.iter().flat_map(member_names).collect();
    assert_eq!(everyone, ["Alice", "Bob"]);
    let bob_state = &first
        .iter()
        .flat_map(|u| &u.members)
        .find(|m| m.name == "Bob");
    assert_eq!(
        bob_state.map(|m| (m.health, m.max_health, m.death, m.position.x)),
        Some((1000, 1000, DeathState::Alive, 2.0))
    );
    member_states(&mut app, &bob);

    pump(&mut app);
    assert!(
        member_states(&mut app, &alice).is_empty(),
        "unchanged state is not resent"
    );

    let debuff = AuraView {
        overrides: Vec::new(),
        instance_id: 1,
        spell_id: 589,
        caster: None,
        stacks: 1,
        charges: 0,
        duration_ms: 18_000,
        remaining_ms: 18_000,
        harmful: true,
        dispel_type: 1,
        flags: 0,
    };
    let buff = AuraView {
        spell_id: 21562,
        harmful: false,
        instance_id: 2,
        ..debuff.clone()
    };
    app.world_mut().entity_mut(bob.player).insert((
        Health {
            current: 412.4,
            max: 1000.0,
        },
        UnitPowers {
            entries: vec![PowerEntry {
                power: PowerType::Mana,
                current: 300,
                max: 500,
                partial: 625,
                regen_per_sec: 100.0,
            }],
            charged_points: Vec::new(),
        },
        UnitAuras {
            auras: vec![debuff.clone(), buff],
        },
        PlayerDeathState {
            state: DeathState::Ghost,
            ..Default::default()
        },
    ));
    pump(&mut app);

    let updates = member_states(&mut app, &alice);
    let changed: Vec<&GroupMemberState> = updates.iter().flat_map(|u| &u.members).collect();
    assert_eq!(changed.len(), 1, "only Bob changed");
    assert_eq!(changed[0].name, "Bob");
    assert_eq!((changed[0].health, changed[0].max_health), (412, 1000));
    assert_eq!(
        changed[0].power,
        Some(PowerEntry {
            power: PowerType::Mana,
            current: 300,
            max: 500,
            partial: 625,
            regen_per_sec: 100.0,
        })
    );
    assert_eq!(changed[0].death, DeathState::Ghost);
    assert_eq!(changed[0].debuffs, vec![debuff]);
}

#[test]
fn member_states_follow_membership_and_resend_to_a_relogged_member() {
    let mut app = setup_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(100),
    ));
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    invite_and_accept(&mut app, &alice, &bob, "Bob");
    pump(&mut app);
    member_states(&mut app, &alice);

    app.world_mut().despawn(bob.conn);
    app.world_mut().despawn(bob.player);
    pump(&mut app);
    let bob_again = connect(&mut app, 2, "Bob");
    pump(&mut app);

    let resent: Vec<_> = member_states(&mut app, &bob_again)
        .iter()
        .flat_map(|u| u.members.iter().map(|m| m.name.clone()))
        .collect();
    assert_eq!(resent, ["Alice", "Bob"]);
    let to_alice: Vec<_> = member_states(&mut app, &alice)
        .iter()
        .flat_map(|u| u.members.iter().map(|m| m.name.clone()))
        .collect();
    assert_eq!(to_alice, ["Bob"], "Alice already has her own state");
}

#[test]
fn invitee_receives_invite_cancelled_when_the_invite_expires() {
    let mut app = setup_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_secs(31),
    ));
    app.world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(Duration::from_secs(31));
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    send(&mut app, &alice, GroupInviteIntent { name: "Bob".into() });
    pump(&mut app);
    assert_eq!(prompts(&mut app, &bob).len(), 1);
    pump(&mut app);

    assert_eq!(
        std::mem::take(&mut inbox(&mut app, &bob).cancelled_invites),
        vec![GroupInviteCancelled {
            inviter_name: "Alice".into(),
        }]
    );
    assert!(codes(&mut app, &alice).contains(&GroupMessageCode::DeclineGroup));
}

// --- Raid target icons (TrinityCore Group::SetTargetIcon) ---

/// Icons with `marks` as (icon 1–8, unit) pairs.
fn icons(marks: &[(u8, u64)]) -> RaidTargetIcons {
    let mut icons = RaidTargetIcons::default();
    for &(icon, unit) in marks {
        icons.targets[icon as usize - 1] = Some(unit);
    }
    icons
}

const HOGGER: u64 = 4_294_967_401;
const MURLOC: u64 = 4_294_967_402;

#[test]
fn solo_player_marks_an_npc_for_themselves() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");

    send(
        &mut app,
        &alice,
        SetRaidTarget {
            target: HOGGER,
            icon: RAID_TARGET_SKULL,
        },
    );
    pump(&mut app);

    assert_eq!(
        raid_targets(&mut app, &alice),
        [icons(&[(RAID_TARGET_SKULL, HOGGER)])]
    );
    assert_eq!(raid_targets(&mut app, &bob), []);
}

#[test]
fn group_mark_reaches_every_member_moves_and_clears() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    invite_and_accept(&mut app, &alice, &bob, "bob");
    raid_targets(&mut app, &alice);
    raid_targets(&mut app, &bob);

    let mark = |target, icon| SetRaidTarget { target, icon };
    send(&mut app, &bob, mark(HOGGER, RAID_TARGET_SKULL));
    pump(&mut app);
    for client in [&alice, &bob] {
        assert_eq!(
            raid_targets(&mut app, client),
            [icons(&[(RAID_TARGET_SKULL, HOGGER)])]
        );
    }

    // One unit per icon: Skull moves to the murloc.
    send(&mut app, &alice, mark(MURLOC, RAID_TARGET_SKULL));
    pump(&mut app);
    for client in [&alice, &bob] {
        assert_eq!(
            raid_targets(&mut app, client),
            [icons(&[(RAID_TARGET_SKULL, MURLOC)])]
        );
    }

    // One icon per unit: Cross on the murloc takes Skull off it.
    send(&mut app, &alice, mark(MURLOC, 7));
    pump(&mut app);
    for client in [&alice, &bob] {
        assert_eq!(raid_targets(&mut app, client), [icons(&[(7, MURLOC)])]);
    }

    // SetRaidTarget(unit, 0) clears the unit's icon.
    send(&mut app, &bob, mark(MURLOC, 0));
    pump(&mut app);
    for client in [&alice, &bob] {
        assert_eq!(raid_targets(&mut app, client), [RaidTargetIcons::default()]);
    }
}

#[test]
fn raid_members_other_than_the_leader_cannot_mark() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    invite_and_accept(&mut app, &alice, &bob, "bob");
    send(&mut app, &alice, ConvertGroupToRaid);
    pump(&mut app);
    raid_targets(&mut app, &alice);

    send(
        &mut app,
        &bob,
        SetRaidTarget {
            target: HOGGER,
            icon: RAID_TARGET_SKULL,
        },
    );
    pump(&mut app);

    assert_eq!(raid_targets(&mut app, &alice), []);
}

#[test]
fn joining_a_party_clears_its_icons_and_leaving_clears_the_leavers() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    let carol = connect(&mut app, 3, "Carol");
    invite_and_accept(&mut app, &alice, &bob, "bob");
    send(
        &mut app,
        &alice,
        SetRaidTarget {
            target: HOGGER,
            icon: 1,
        },
    );
    pump(&mut app);
    raid_targets(&mut app, &bob);

    // Group::AddMember resets the icons of a non-raid group.
    invite_and_accept(&mut app, &alice, &carol, "carol");
    assert_eq!(
        raid_targets(&mut app, &bob).last(),
        Some(&RaidTargetIcons::default())
    );

    send(
        &mut app,
        &alice,
        SetRaidTarget {
            target: HOGGER,
            icon: 1,
        },
    );
    pump(&mut app);
    raid_targets(&mut app, &carol);
    send(&mut app, &carol, LeaveGroup);
    pump(&mut app);
    assert_eq!(
        raid_targets(&mut app, &carol).last(),
        Some(&RaidTargetIcons::default())
    );
    assert_eq!(
        raid_targets(&mut app, &bob).last(),
        Some(&icons(&[(1, HOGGER)]))
    );
}

// Portrait payloads use the real GroupChannel loopback harness, not replication interest.
fn roster_portrait_fixture() -> (
    App,
    Client,
    Client,
    Client,
    shared::protocol::GroupPortraitAppearance,
) {
    use shared::components::{
        CharacterAppearance, CustomizationChoiceSelection, EquipmentAppearance,
        EquipmentVisualSlot, EquippedAppearanceEntry,
    };
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    let cid = connect(&mut app, 3, "Cid");
    let portrait = shared::protocol::GroupPortraitAppearance {
        race: 4,
        appearance: CharacterAppearance {
            sex: 1,
            skin_color: 3,
            face: 2,
            eye_color: 5,
            hair_style: 7,
            hair_color: 8,
            facial_style: 4,
            customization_choices: vec![CustomizationChoiceSelection {
                option_id: 101,
                choice_id: 202,
            }],
            visage: None,
        },
        head: Some(EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Head,
            item_id: Some(19019),
            definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
            display_info_id: Some(12345),
            inventory_type: 1,
            hidden: false,
        }),
    };
    app.world_mut().get_mut::<Player>(bob.player).unwrap().race = portrait.race;
    app.world_mut()
        .get_mut::<Player>(bob.player)
        .unwrap()
        .appearance = portrait.appearance.clone();
    app.world_mut().entity_mut(bob.player).insert((
        EquipmentAppearance {
            entries: vec![
                portrait.head.clone().unwrap(),
                EquippedAppearanceEntry {
                    slot: EquipmentVisualSlot::Chest,
                    item_id: Some(100),
                    definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
                    display_info_id: Some(200),
                    inventory_type: 5,
                    hidden: false,
                },
            ],
        },
        Position {
            x: 50_000.0,
            y: 20_000.0,
            z: 100.0,
        },
        crate::maps::InMap::continent(1),
    ));
    app.world_mut()
        .entity_mut(alice.player)
        .insert(crate::maps::InMap::continent(0));
    invite_and_accept(&mut app, &alice, &bob, "Bob");
    invite_and_accept(&mut app, &alice, &cid, "Cid");
    (app, alice, bob, cid, portrait)
}

#[test]
fn roster_portrait_three_members_far_and_cross_zone() {
    let (mut app, alice, bob, _cid, mut expected) = roster_portrait_fixture();
    let roster = last_roster(&mut app, &alice);
    assert_eq!(roster.members.len(), 3);
    assert_eq!(
        roster
            .members
            .iter()
            .find(|m| m.name == "Bob")
            .unwrap()
            .portrait,
        expected
    );
    app.world_mut()
        .entity_mut(bob.player)
        .insert(crate::maps::InMap::continent(530));
    pump(&mut app);
    assert!(rosters(&mut app, &alice).is_empty());
    // A head change after the transfer reaches A without leaving or rejoining.
    expected.appearance.face = 6;
    app.world_mut()
        .get_mut::<Player>(bob.player)
        .unwrap()
        .appearance = expected.appearance.clone();
    pump(&mut app);
    assert_eq!(
        last_roster(&mut app, &alice)
            .members
            .iter()
            .find(|m| m.name == "Bob")
            .unwrap()
            .portrait,
        expected
    );
}

#[test]
fn roster_portrait_helm_and_customization_changes_send_once() {
    use shared::components::{EquipmentAppearance, EquipmentVisualSlot};
    let (mut app, alice, bob, _cid, mut expected) = roster_portrait_fixture();
    rosters(&mut app, &alice);
    let head = expected.head.as_mut().unwrap();
    head.display_info_id = Some(54321);
    head.hidden = true;
    app.world_mut()
        .get_mut::<EquipmentAppearance>(bob.player)
        .unwrap()
        .entries[0] = head.clone();
    pump(&mut app);
    let updates = rosters(&mut app, &alice);
    assert_eq!(updates.len(), 1);
    assert_eq!(
        updates[0]
            .members
            .iter()
            .find(|m| m.name == "Bob")
            .unwrap()
            .portrait,
        expected
    );
    expected.appearance.hair_color = 9;
    app.world_mut()
        .get_mut::<Player>(bob.player)
        .unwrap()
        .appearance = expected.appearance.clone();
    pump(&mut app);
    assert_eq!(
        last_roster(&mut app, &alice)
            .members
            .iter()
            .find(|m| m.name == "Bob")
            .unwrap()
            .portrait,
        expected
    );
    // Non-head equipment changes do not resend portraits.
    app.world_mut()
        .get_mut::<EquipmentAppearance>(bob.player)
        .unwrap()
        .entries[1]
        .display_info_id = Some(999);
    pump(&mut app);
    assert!(rosters(&mut app, &alice).is_empty());
    app.world_mut()
        .get_mut::<EquipmentAppearance>(bob.player)
        .unwrap()
        .entries
        .retain(|entry| entry.slot != EquipmentVisualSlot::Head);
    expected.head = None;
    pump(&mut app);
    assert_eq!(
        last_roster(&mut app, &alice)
            .members
            .iter()
            .find(|m| m.name == "Bob")
            .unwrap()
            .portrait,
        expected
    );
    pump(&mut app);
    assert!(rosters(&mut app, &alice).is_empty());
}

#[test]
fn roster_portrait_offline_member_retains_last_appearance_for_new_recipient() {
    let (mut app, alice, bob, cid, mut expected) = roster_portrait_fixture();
    rosters(&mut app, &alice);
    expected.head.as_mut().unwrap().display_info_id = Some(54321);
    app.world_mut()
        .get_mut::<EquipmentAppearance>(bob.player)
        .unwrap()
        .entries[0] = expected.head.clone().unwrap();
    pump(&mut app);
    assert_eq!(
        last_roster(&mut app, &alice)
            .members
            .iter()
            .find(|member| member.name == "Bob")
            .unwrap()
            .portrait,
        expected
    );
    app.world_mut().despawn(bob.conn);
    app.world_mut().despawn(bob.player);
    pump(&mut app);
    let roster = last_roster(&mut app, &alice);
    let member = roster.members.iter().find(|m| m.name == "Bob").unwrap();
    assert!(!member.online);
    assert_eq!(member.entity, None);
    assert_eq!(member.portrait, expected);
    // Cid reconnects without ever needing Bob's replicated entity.
    app.world_mut().despawn(cid.conn);
    app.world_mut().despawn(cid.player);
    pump(&mut app);
    let cid = connect(&mut app, 3, "Cid");
    pump(&mut app);
    let roster = last_roster(&mut app, &cid);
    let member = roster.members.iter().find(|m| m.name == "Bob").unwrap();
    assert!(!member.online);
    assert_eq!(member.portrait, expected);
}

#[test]
fn roster_portrait_persisted_helm_equip_reaches_far_member() {
    use crate::item_templates::{ItemTemplate, ItemTemplates};
    use crate::persistence::{self, GameDatabase, InventoryData, InventoryItemData};
    use shared::components::{CharacterAppearance, EquipmentVisualSlot, EquippedAppearanceEntry};
    use shared::protocol::*;

    const HELM: u32 = 32329; // Cowl of Benevolence, head inventory type 1.
    let db = persistence::temp_database();
    persistence::create_character(&db, 1, "Alice", 1, 2).unwrap();
    let (bob_id, mut character) = persistence::create_character(&db, 2, "Bob", 4, 1).unwrap();
    character.appearance = CharacterAppearance {
        sex: 1,
        face: 3,
        hair_color: 8,
        ..Default::default()
    };
    persistence::save_character(&db, bob_id, &character);
    persistence::save_inventory(
        &db,
        bob_id,
        &InventoryData {
            gold: 0,
            items: vec![InventoryItemData {
                item_guid: persistence::next_id(&db, "item_instance"),
                item_id: HELM,
                name: "Cowl of Benevolence".into(),
                quality: 4,
                required_level: 0,
                stack_count: 1,
                vendor_sell_price: 0,
                unique_auction: false,
            }],
        },
    );
    let loaded = persistence::load_character(&db, bob_id).unwrap();
    let mut app = setup_app_with(move |app| {
        app.insert_resource(GameDatabase::new(db));
        app.init_resource::<crate::gold::GoldUpdateQueue>();
        app.insert_resource(ItemTemplates {
            retail: [(
                HELM,
                ItemTemplate {
                    inventory_type: 1,
                    max_stack: 1,
                    allowable_class: -1,
                    allowable_race: u64::MAX,
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            forever: Default::default(),
            loot_need: Default::default(),
            retail_learning_spells: Default::default(),
        });
        app.register_message::<InventorySnapshot>();
        app.register_message::<EquipmentSnapshot>();
        app.register_message::<InventoryDelta>();
        app.register_message::<InventoryError>();
        app.register_message::<SwapItem>();
        app.register_message::<SplitItem>();
        app.register_message::<DestroyItem>();
        app.register_message::<EquipItem>();
        app.register_message::<SortBags>();
        app.register_message::<UseItem>();
        app.add_plugins(crate::inventory::InventoryPlugin);
        app.add_systems(
            Update,
            |mut connections: Query<(&mut Inbox, &mut MessageReceiver<InventoryError>)>| {
                for (mut inbox, mut errors) in &mut connections {
                    inbox.inventory_errors.extend(errors.receive());
                }
            },
        );
    });
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, bob_id, "Bob");
    let cid = connect(&mut app, 3, "Cid");
    app.world_mut().entity_mut(bob.player).insert((
        Player {
            name: loaded.name,
            race: loaded.race,
            class: loaded.class,
            appearance: loaded.appearance.clone(),
        },
        EquipmentAppearance::default(),
        Position {
            x: 50_000.0,
            y: 20_000.0,
            z: 100.0,
        },
        crate::maps::InMap::continent(1),
    ));
    app.world_mut().entity_mut(bob.conn).insert((
        crate::auth::InWorld {
            character_id: bob_id,
        },
        MessageSender::<InventorySnapshot>::default(),
        MessageReceiver::<InventorySnapshot>::default(),
        MessageSender::<EquipmentSnapshot>::default(),
        MessageReceiver::<EquipmentSnapshot>::default(),
        MessageSender::<InventoryDelta>::default(),
        MessageReceiver::<InventoryDelta>::default(),
        MessageSender::<InventoryError>::default(),
        MessageReceiver::<InventoryError>::default(),
        MessageSender::<SwapItem>::default(),
        MessageReceiver::<SwapItem>::default(),
    ));
    invite_and_accept(&mut app, &alice, &bob, "Bob");
    invite_and_accept(&mut app, &alice, &cid, "Cid");
    rosters(&mut app, &alice);
    app.world_mut()
        .get_mut::<MessageSender<SwapItem>>(bob.conn)
        .unwrap()
        .send::<InventoryChannel>(SwapItem {
            from: ItemLocation::Bag { bag: 0, slot: 0 },
            to: ItemLocation::Equipment(EquipmentSlot::Head),
        });
    // Request delivery, inventory PostUpdate, group Update, then roster receipt.
    pump(&mut app);
    app.update();
    assert!(
        inbox(&mut app, &bob).inventory_errors.is_empty(),
        "equip request rejected: {:?}",
        inbox(&mut app, &bob).inventory_errors
    );
    let updates = rosters(&mut app, &alice);
    let expected = GroupPortraitAppearance {
        race: 4,
        appearance: loaded.appearance,
        head: Some(EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Head,
            item_id: Some(HELM),
            definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
            display_info_id: None,
            inventory_type: 1,
            hidden: false,
        }),
    };
    let persisted =
        persistence::load_character(&app.world().resource::<GameDatabase>().db, bob_id).unwrap();
    let expected_equipment = vec![expected.head.clone().unwrap()];
    assert_eq!(persisted.equipment_appearance.entries, expected_equipment);
    assert_eq!(
        app.world()
            .get::<EquipmentAppearance>(bob.player)
            .unwrap()
            .entries,
        expected_equipment
    );
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].members.len(), 3);
    assert_eq!(
        updates[0]
            .members
            .iter()
            .find(|member| member.name == "Bob")
            .unwrap()
            .portrait,
        expected
    );
    pump(&mut app);
    assert!(rosters(&mut app, &alice).is_empty());
}

#[test]
fn minimap_ping_reaches_other_party_members_only() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    let carol = connect(&mut app, 3, "Carol");
    invite_and_accept(&mut app, &alice, &bob, "bob");

    send(
        &mut app,
        &alice,
        MinimapPingRequest {
            x: -8913.5,
            y: -133.25,
        },
    );
    pump(&mut app);

    assert_eq!(
        minimap_pings(&mut app, &bob),
        [MinimapPing {
            sender: alice.player.to_bits(),
            sender_name: "Alice".into(),
            x: -8913.5,
            y: -133.25,
        }]
    );
    assert_eq!(minimap_pings(&mut app, &alice), []);
    assert_eq!(minimap_pings(&mut app, &carol), []);
}

#[test]
fn minimap_ping_off_the_map_or_while_solo_is_dropped() {
    let mut app = setup_app();
    let alice = connect(&mut app, 1, "Alice");
    let bob = connect(&mut app, 2, "Bob");
    let carol = connect(&mut app, 3, "Carol");
    invite_and_accept(&mut app, &alice, &bob, "bob");

    for (x, y) in [
        (MINIMAP_PING_MAP_HALFSIZE + 1.0, 0.0),
        (0.0, f32::NAN),
        (f32::NEG_INFINITY, 0.0),
    ] {
        send(&mut app, &alice, MinimapPingRequest { x, y });
    }
    send(&mut app, &carol, MinimapPingRequest { x: 10.0, y: 20.0 });
    pump(&mut app);

    for client in [&alice, &bob, &carol] {
        assert_eq!(minimap_pings(&mut app, client), []);
    }
}
