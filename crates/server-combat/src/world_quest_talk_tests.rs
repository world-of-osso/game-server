//! TalkTo credit through validated interaction messages and persisted character logs.
//! TrinityCore a352b1fa Player.cpp:16697-16700: TalkedToCreature credits TALKTO only.
use super::*;
use crate::maps::InMap;
use crate::quest_data::{ObjectiveDef, ObjectiveKind, QuestTemplate};
use shared::protocol::{InteractionChannel, InteractionError, InteractionKind, NpcFlags};

// Synthetic quest fixture, not imported production content. Farley's gossip menu
// and faction/flags below are read from the existing world.db reference rows.
const TALK_FIXTURE: u32 = 4_000_000_001;
const FARLEY: u32 = 295;

fn talk_app() -> (App, Client, Entity) {
    let mut app = object_app();
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    app.insert_resource(crate::npc_interaction::load_gossip_catalog(&conn).unwrap());
    let (name, faction, flags): (String, u32, u64) = conn
        .query_row(
            "SELECT name, faction, npcflag FROM content_creature_template WHERE entry = ?1",
            [FARLEY],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let objectives = [
        (ObjectiveKind::TalkTo, FARLEY, 2),
        (ObjectiveKind::Monster, FARLEY, 1),
        (ObjectiveKind::TalkTo, 6929, 1),
    ]
    .into_iter()
    .map(|(kind, object_id, required)| ObjectiveDef {
        kind,
        object_id,
        required,
        text: "Test objective".into(),
    })
    .collect();
    app.world_mut().resource_mut::<QuestDb>().quests.insert(
        TALK_FIXTURE,
        QuestTemplate {
            id: TALK_FIXTURE,
            title: "TalkTo test fixture".into(),
            objectives,
            world_objective_rules: Some(vec![Default::default(); 3]),
            ..Default::default()
        },
    );
    let client = object_client(&mut app);
    add_message_pair::<InteractNpc>(&mut app, client.connection);
    add_message_pair::<SelectGossipOption>(&mut app, client.connection);
    let mut log = load_quest_log(db(&app), client.character_id);
    log.active
        .push(crate::persistence::quest_store::ActiveQuestData {
            quest_id: TALK_FIXTURE,
            counts: vec![0; 3],
            ..Default::default()
        });
    save_quest_log(db(&app), client.character_id, &log);
    let npc = app
        .world_mut()
        .spawn((
            Npc {
                template_id: FARLEY,
                name,
            },
            NpcFlags(flags),
            origin(),
            Health {
                current: 100.,
                max: 100.,
            },
            UnitFactionTemplate(faction),
            InMap::continent(0),
        ))
        .id();
    (app, client, npc)
}

fn talk_request<M: Message>(app: &mut App, client: &Client, message: M) {
    app.world_mut()
        .get_mut::<MessageSender<M>>(client.connection)
        .unwrap()
        .send::<InteractionChannel>(message);
    for _ in 0..2 {
        app.update();
    }
}

fn talk_counts(app: &App, character: u64) -> Vec<u32> {
    load_quest_log(db(app), character)
        .active(TALK_FIXTURE)
        .unwrap()
        .counts
        .clone()
}

#[test]
fn world_quest_talk_opening_persists_actor_only_capped_kind_and_template_credit() {
    let (mut app, client, npc) = talk_app();
    let (buddy, _) = spawn_quest_group_member(&mut app, &client);
    assert_eq!(talk_counts(&app, client.character_id), vec![0, 0, 0]);
    for expected in [1, 2, 2] {
        talk_request(&mut app, &client, InteractNpc { npc: npc.to_bits() });
        let opened = received::<InteractionOpened>(&mut app, &client);
        assert!(
            matches!(
                &opened[..],
                [InteractionOpened { kind: InteractionKind::Gossip(menu), .. }]
                    if menu.menu_id == 1291
            ),
            "{opened:?}"
        );
        assert!(received::<InteractionFailed>(&mut app, &client).is_empty());
        assert_eq!(talk_counts(&app, client.character_id), vec![expected, 0, 0]);
        assert_eq!(talk_counts(&app, buddy), vec![0, 0, 0]);
    }
    // Login reconstructs the counts from redb, rather than transient ECS state.
    app.world_mut()
        .entity_mut(client.connection)
        .remove::<InWorld>();
    let snapshot = enter_world(&mut app, &client);
    let entry = snapshot
        .entries
        .iter()
        .find(|quest| quest.quest_id == TALK_FIXTURE)
        .unwrap();
    assert_eq!(entry.objectives[0].current, 2);
    assert_eq!(
        entry.objectives[0].kind,
        shared::protocol::QuestObjectiveKind::TalkTo
    );
    assert_eq!(entry.objectives[1].current, 0);
}

#[test]
fn world_quest_talk_hello_and_submenu_do_not_credit() {
    let (mut app, client, npc) = talk_app();
    request(&mut app, &client, QuestGiverHello { npc: npc.to_bits() });
    assert_eq!(received::<QuestGiverQuestList>(&mut app, &client).len(), 1);
    assert_eq!(talk_counts(&app, client.character_id), vec![0, 0, 0]);
    talk_request(&mut app, &client, InteractNpc { npc: npc.to_bits() });
    received::<InteractionOpened>(&mut app, &client);
    talk_request(
        &mut app,
        &client,
        SelectGossipOption {
            npc: npc.to_bits(),
            option_id: 0,
        },
    );
    let opened = received::<InteractionOpened>(&mut app, &client);
    assert!(
        matches!(
            &opened[..],
            [InteractionOpened { kind: InteractionKind::Gossip(menu), .. }]
                if menu.menu_id == 1221
        ),
        "{opened:?}"
    );
    assert_eq!(talk_counts(&app, client.character_id), vec![1, 0, 0]);
    request(&mut app, &client, QuestGiverHello { npc: npc.to_bits() });
    assert_eq!(talk_counts(&app, client.character_id), vec![1, 0, 0]);
}

#[test]
fn world_quest_talk_rejects_hostile_dead_distant_and_crosscopy_before_opening() {
    let (mut app, client, npc) = talk_app();
    let faction = *app.world().get::<UnitFactionTemplate>(npc).unwrap();
    let flags = *app.world().get::<NpcFlags>(npc).unwrap();
    assert_eq!(
        app.world()
            .resource::<crate::faction::FactionTemplates>()
            .reaction(29, 1),
        crate::faction::Reaction::Hostile,
    );
    for (case, error) in [
        ("hostile", InteractionError::TargetHostile),
        ("dead", InteractionError::TargetDead),
        ("distant", InteractionError::TooFarAway),
        ("map", InteractionError::InvalidTarget),
        ("instance", InteractionError::InvalidTarget),
        ("missing-map", InteractionError::InvalidTarget),
        ("no-opening", InteractionError::InvalidTarget),
        ("player-dead", InteractionError::PlayerDead),
    ] {
        app.world_mut().entity_mut(npc).insert((
            faction,
            flags,
            origin(),
            InMap::continent(0),
            Health {
                current: 100.,
                max: 100.,
            },
        ));
        app.world_mut().entity_mut(client.player).insert(Health {
            current: 100.,
            max: 100.,
        });
        match case {
            "hostile" => {
                app.world_mut()
                    .entity_mut(npc)
                    .insert(UnitFactionTemplate(29));
            }
            "dead" => {
                app.world_mut().get_mut::<Health>(npc).unwrap().current = 0.;
            }
            "distant" => {
                app.world_mut().get_mut::<Position>(npc).unwrap().x = 50.;
            }
            "map" => {
                app.world_mut().entity_mut(npc).insert(InMap::continent(1));
            }
            "instance" => {
                app.world_mut().entity_mut(npc).insert(InMap {
                    map_id: 0,
                    instance_id: 7,
                });
            }
            "missing-map" => {
                app.world_mut().entity_mut(npc).remove::<InMap>();
            }
            "no-opening" => {
                app.world_mut().entity_mut(npc).insert(NpcFlags(0));
            }
            "player-dead" => {
                app.world_mut()
                    .get_mut::<Health>(client.player)
                    .unwrap()
                    .current = 0.;
            }
            _ => unreachable!(),
        }
        talk_request(&mut app, &client, InteractNpc { npc: npc.to_bits() });
        assert!(
            received::<InteractionOpened>(&mut app, &client).is_empty(),
            "{case}"
        );
        assert_eq!(
            received::<InteractionFailed>(&mut app, &client),
            vec![InteractionFailed {
                npc: npc.to_bits(),
                error
            }],
            "{case}"
        );
        assert_eq!(
            talk_counts(&app, client.character_id),
            vec![0, 0, 0],
            "{case}"
        );
        assert!(
            app.world()
                .get::<crate::npc_interaction::ActiveInteraction>(client.connection)
                .is_none(),
            "{case}"
        );
    }
    // Nonzero instances remain valid when both entities occupy the same copy.
    let map = InMap {
        map_id: 36,
        instance_id: 7,
    };
    app.world_mut()
        .entity_mut(npc)
        .insert((faction, flags, origin(), map));
    app.world_mut().entity_mut(client.player).insert((
        map,
        Health {
            current: 100.,
            max: 100.,
        },
    ));
    talk_request(&mut app, &client, InteractNpc { npc: npc.to_bits() });
    assert_eq!(received::<InteractionOpened>(&mut app, &client).len(), 1);
    assert!(received::<InteractionFailed>(&mut app, &client).is_empty());
    assert_eq!(talk_counts(&app, client.character_id), vec![1, 0, 0]);
}
