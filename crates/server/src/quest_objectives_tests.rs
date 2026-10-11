//! Authored game-object and spell-credit objectives through server requests.
use super::*;
use shared::components::{Health, UnitFactionTemplate};
use shared::protocol::{
    CloseInteraction, InteractNpc, InteractionClosed, InteractionFailed, InteractionOpened,
    SelectGossipOption, UseGameObject,
};

fn object_app() -> App {
    setup_app_with(|app| {
        app.insert_resource(crate::faction::world_db_faction_templates().unwrap());
        app.add_plugins(crate::npc_interaction::NpcInteractionPlugin);
        app.register_message::<UseGameObject>();
        app.register_message::<InteractNpc>();
        app.register_message::<SelectGossipOption>();
        app.register_message::<CloseInteraction>();
        app.register_message::<InteractionFailed>();
        app.register_message::<InteractionOpened>();
        app.register_message::<InteractionClosed>();
    })
    .expect("authentic quest data required")
}

fn object_client(app: &mut App) -> Client {
    let client = spawn_client(app, 80);
    app.world_mut().entity_mut(client.player).insert((
        Health {
            current: 100.0,
            max: 100.0,
        },
        UnitFactionTemplate(1),
    ));
    add_message_pair::<UseGameObject>(app, client.connection);
    add_message_pair::<InteractionFailed>(app, client.connection);
    add_message_pair::<InteractionOpened>(app, client.connection);
    add_message_pair::<InteractionClosed>(app, client.connection);
    enter_world(app, &client);
    client
}

fn real_object(app: &mut App, entry: u32) -> Entity {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let spawns = crate::game_objects::load_game_object_spawns(&conn).unwrap();
    let spawn = spawns
        .0
        .iter()
        .find(|spawn| spawn.info.entry == entry)
        .expect("authored quest object must spawn");
    let (_, rotation, info, bounds) = crate::game_objects::game_object_bundle(spawn);
    // Same map and geometry as the authored spawn, translated to the test origin.
    app.world_mut()
        .spawn((
            origin(),
            rotation,
            info,
            bounds,
            crate::maps::InMap::continent(0),
            spawn.quest_object.clone().expect("quest object metadata"),
        ))
        .id()
}

#[test]
fn quest_objectives_go_use_credits_only_reachable_objects_on_the_players_map() {
    let mut app = object_app();
    let client = object_client(&mut app);
    app.world_mut()
        .get_mut::<Player>(client.player)
        .unwrap()
        .race = 2;
    mark_rewarded(&app, &client, 11598);
    let giver = spawn_npc(&mut app, 25279, 0.0);
    accept(&mut app, &client, giver, 11602); // Cutting Off the Source: ten Nerub'ar egg sacs.
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    let egg = real_object(&mut app, 187655);
    app.world_mut().entity_mut(egg).insert(Position {
        x: 50.0,
        y: 0.0,
        z: 0.0,
    });
    request(
        &mut app,
        &client,
        UseGameObject {
            object: egg.to_bits(),
        },
    );
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(11602)
            .unwrap()
            .counts,
        vec![0]
    );
    app.world_mut()
        .entity_mut(egg)
        .insert((origin(), crate::maps::InMap::continent(1)));
    request(
        &mut app,
        &client,
        UseGameObject {
            object: egg.to_bits(),
        },
    );
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(11602)
            .unwrap()
            .counts,
        vec![0]
    );
    app.world_mut()
        .entity_mut(egg)
        .insert(crate::maps::InMap::continent(0));
    request(
        &mut app,
        &client,
        UseGameObject {
            object: egg.to_bits(),
        },
    );
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(11602)
            .unwrap()
            .counts,
        vec![1]
    );
    // A consumed egg cannot be clicked ten times to finish the quest.
    request(
        &mut app,
        &client,
        UseGameObject {
            object: egg.to_bits(),
        },
    );
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(11602)
            .unwrap()
            .counts,
        vec![1]
    );
    for _ in 1..10 {
        let egg = real_object(&mut app, 187655);
        request(
            &mut app,
            &client,
            UseGameObject {
                object: egg.to_bits(),
            },
        );
    }
    turn_in(&mut app, &client, giver, 11602, Some(0));
    assert!(load_quest_log(db(&app), client.character_id).is_rewarded(11602));
}
