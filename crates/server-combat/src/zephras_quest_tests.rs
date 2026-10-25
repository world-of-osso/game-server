//! Imported scratch Forever rows through the existing quest message and redb boundary.
use super::*;

fn spawn_skyborne_client(app: &mut App, race: u8, level: u8) -> Client {
    let kit = crate::player_create_info::OwnedKit::from_world_db().expect("real creation kit");
    let (character_id, data) = persistence::create_character_with_appearance(
        db(app),
        1,
        "Skyquest",
        race,
        1,
        CharacterAppearance::default(),
        &kit.kit(),
    )
    .expect("create Skyborne character");
    assert_eq!(
        crate::maps::persistence::load_character_map(db(app), character_id),
        2991
    );
    let (x, y, z) = crate::terrain_height::wow_to_bevy(data.x, data.y, data.z);
    let client = spawn_client_connection(app, character_id, race, level, Position { x, y, z });
    enter_world(app, &client);
    client
}

fn spawn_client_connection(
    app: &mut App,
    character_id: u64,
    race: u8,
    level: u8,
    position: Position,
) -> Client {
    let player = app
        .world_mut()
        .spawn((
            crate::maps::InMap::continent(2991),
            Player {
                name: "Skyquest".into(),
                race,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            position,
            ClientId(character_id),
            Level(level),
            EquipmentAppearance::default(),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            PlayerLink(player),
        ))
        .id();
    for_quest_messages!(add_message_pair, app, connection);
    Client {
        character_id,
        player,
        connection,
    }
}

fn spawn_imported_giver(app: &mut App, client: &Client, entry: u32) -> u64 {
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let position = app.world().get::<Position>(client.player).unwrap();
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let guid: u32 = conn
        .query_row(
            "SELECT guid FROM content_creature WHERE map=2991 AND id1=?1
         AND (position_x-?2)*(position_x-?2)+(position_y-?3)*(position_y-?3)
             +(position_z-?4)*(position_z-?4) <= 25 ORDER BY guid LIMIT 1",
            rusqlite::params![entry, x, y, z],
            |row| row.get(0),
        )
        .expect("imported giver within 5 yards of approved start on map 2991");
    let spawn = crate::world_data::load_spawn(&conn, guid).expect("runtime spawn loader");
    assert_eq!(spawn.template_entry, entry);
    let (display_id, name): (u32, String) = conn
        .query_row(
            "SELECT m.CreatureDisplayID,t.name FROM content_creature_template_model m
         JOIN content_creature_template t ON t.entry=m.CreatureID
         WHERE m.CreatureID=?1 ORDER BY m.Idx LIMIT 1",
            [entry],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("captured NPC-page appearance");
    let expected_display = match entry {
        251362 => 136968,
        251487 => 139694,
        251361 => 136966,
        _ => panic!("unexpected fixture giver {entry}"),
    };
    assert_eq!(display_id, expected_display);
    let (x, y, z) =
        crate::terrain_height::wow_to_bevy(spawn.position_x, spawn.position_y, spawn.position_z);
    app.world_mut()
        .spawn((
            crate::maps::InMap::continent(2991),
            Npc {
                template_id: entry,
                name,
            },
            shared::components::ModelDisplay { display_id },
            Position { x, y, z },
        ))
        .id()
        .to_bits()
}

#[test]
fn skyborne_zephras_coming_of_age_accepts_and_persists_captured_objectives() {
    let mut app = setup_app().expect("scratch content database");
    let quest = &app.world().resource::<QuestDb>().quests[&92460];
    assert_eq!(quest.allowable_races as u64, 1_u64 << 32);
    assert!(quest.objectives.is_empty());
    assert_eq!(
        quest.objectives_text,
        "Speak with Rorian the Dayseeker in Thendal Grove."
    );
    let next = &app.world().resource::<QuestDb>().quests[&92461];
    assert_eq!(next.objectives.len(), 1);
    assert_eq!(
        (next.objectives[0].object_id, next.objectives[0].required),
        (250873, 8)
    );
    let client = spawn_skyborne_client(&mut app, 95, 1);
    let giver = spawn_imported_giver(&mut app, &client, 251362);
    accept(&mut app, &client, giver, 92460);
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    let log = load_quest_log(db(&app), client.character_id);
    assert_eq!(log.active(92460).unwrap().counts, Vec::<u32>::new());
}

#[test]
fn skyborne_zephras_coming_of_age_pays_captured_xp_at_its_ender() {
    let mut app = setup_app().expect("scratch content database");
    let client = spawn_skyborne_client(&mut app, 95, 1);
    let giver = spawn_imported_giver(&mut app, &client, 251362);
    accept(&mut app, &client, giver, 92460);
    walk_to(&mut app, &client, (4088.501, 1895.0, 976.515));
    let ender = spawn_imported_giver(&mut app, &client, 251361);
    turn_in(&mut app, &client, ender, 92460, None);
    let complete = received::<QuestGiverQuestComplete>(&mut app, &client);
    assert_eq!(complete.len(), 1);
    assert_eq!(complete[0].xp, 40);
    assert_eq!(
        crate::persistence_progression::load_xp(db(&app), client.character_id),
        40
    );
}

#[test]
fn skyborne_zephras_reactions_match_captured_alliance_horde_pairs() {
    use crate::faction::Reaction::{Friendly, Hostile, Neutral};
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let factions = crate::faction::load_faction_templates(&conn).unwrap();
    for (entry, alliance, horde) in [
        (251362, Friendly, Friendly),
        (251487, Hostile, Friendly),
        (250873, Neutral, Neutral),
        (251404, Hostile, Hostile),
    ] {
        let template: u32 = conn
            .query_row(
                "SELECT faction FROM content_creature_template WHERE entry=?1",
                [entry],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(factions.reaction(template, 3629), alliance);
        assert_eq!(factions.reaction(template, 3628), horde);
        // Wowhead react describes the NPC's reaction; no NPC faction identity is captured.
        // Friendly reactions are symmetric in the native reaction rules.
        if alliance == Friendly {
            assert_eq!(factions.reaction(3629, template), Friendly);
        }
        if horde == Friendly {
            assert_eq!(factions.reaction(3628, template), Friendly);
        }
    }
}

#[test]
fn skyborne_zephras_high_order_intro_keeps_its_race_mask() {
    let quests = crate::quest_data::load_quest_db(&crate::test_data::world_db_path()).unwrap();
    let player = crate::quest_rules::QuestPlayer {
        level: 1,
        race: 96,
        class: 1,
    };
    assert_eq!(
        crate::quest_rules::can_take(
            &quests,
            &QuestLogData::default(),
            player,
            &quests.quests[&92460]
        ),
        Err(QuestFailedReason::WrongRace)
    );
}

#[test]
fn skyborne_zephras_skysight_enforces_level_two_and_blocks_unimplemented_event() {
    let mut app = setup_app().expect("scratch content database");
    assert_eq!(
        app.world().resource::<QuestDb>().quests[&92598].allowable_races,
        0
    );
    let client = spawn_skyborne_client(&mut app, 96, 1);
    let giver = spawn_imported_giver(&mut app, &client, 251487);
    accept(&mut app, &client, giver, 92598);
    assert_eq!(
        received::<QuestFailed>(&mut app, &client),
        vec![QuestFailed {
            quest_id: 92598,
            reason: QuestFailedReason::LowLevel
        }]
    );
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active(92598)
            .is_none()
    );
    app.world_mut().get_mut::<Level>(client.player).unwrap().0 = 2;
    accept(&mut app, &client, giver, 92598);
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active(92598)
            .is_some()
    );
    turn_in(&mut app, &client, giver, 92598, None);
    assert_eq!(
        received::<QuestFailed>(&mut app, &client),
        vec![QuestFailed {
            quest_id: 92598,
            reason: QuestFailedReason::ObjectivesIncomplete
        }]
    );
    assert!(!load_quest_log(db(&app), client.character_id).is_rewarded(92598));
}
