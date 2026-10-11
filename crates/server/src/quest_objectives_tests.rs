//! Authored game-object and spell-credit objectives through server requests.
use super::*;
use shared::components::{Health, UnitFactionTemplate};
use shared::protocol::{
    CloseInteraction, InteractNpc, InteractionClosed, InteractionFailed, InteractionOpened,
    SelectGossipOption, UseGameObject,
};

fn add_object_plugins(app: &mut App) {
    app.insert_resource(crate::faction::world_db_faction_templates().unwrap());
    app.add_plugins(crate::npc_interaction::NpcInteractionPlugin);
    app.register_message::<UseGameObject>();
    app.register_message::<InteractNpc>();
    app.register_message::<SelectGossipOption>();
    app.register_message::<CloseInteraction>();
    app.register_message::<InteractionFailed>();
    app.register_message::<InteractionOpened>();
    app.register_message::<InteractionClosed>();
}

fn object_app() -> App {
    setup_app_with(add_object_plugins).expect("authentic quest data required")
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

#[test]
fn quest_objectives_chest_loot_is_not_credit_until_the_quest_item_enters_bags() {
    use shared::protocol::{
        CorpseLootable, LootClosed, LootFailed, LootRelease, LootResponse, LootSlotRemoved,
        LootSlotRequest, LootUnit,
    };
    let mut app = setup_app_with(|app| {
        add_object_plugins(app);
        let conn = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        app.insert_resource(crate::merchant::load_vendor_catalog(&conn).unwrap());
        app.add_plugins(crate::loot::LootPlugin);
        app.insert_resource(crate::loot::load_loot_tables(&conn).unwrap());
        app.register_message::<LootUnit>();
        app.register_message::<LootSlotRequest>();
        app.register_message::<LootRelease>();
        app.register_message::<LootResponse>();
        app.register_message::<LootSlotRemoved>();
        app.register_message::<LootClosed>();
        app.register_message::<LootFailed>();
        app.register_message::<CorpseLootable>();
    })
    .expect("authentic loot data required");
    let client = object_client(&mut app);
    add_message_pair::<LootUnit>(&mut app, client.connection);
    add_message_pair::<LootSlotRequest>(&mut app, client.connection);
    add_message_pair::<LootRelease>(&mut app, client.connection);
    add_message_pair::<LootResponse>(&mut app, client.connection);
    add_message_pair::<LootSlotRemoved>(&mut app, client.connection);
    add_message_pair::<LootClosed>(&mut app, client.connection);
    add_message_pair::<LootFailed>(&mut app, client.connection);
    add_message_pair::<CorpseLootable>(&mut app, client.connection);
    mark_rewarded(&app, &client, 11019);
    let giver = spawn_npc(&mut app, 23141, 0.0);
    accept(&mut app, &client, giver, 11049); // The Great Netherwing Egg Hunt
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    let egg = real_object(&mut app, 185915);
    request(
        &mut app,
        &client,
        UseGameObject {
            object: egg.to_bits(),
        },
    );
    let responses = received::<LootResponse>(&mut app, &client);
    assert_eq!(responses.len(), 1, "using the egg opens its loot");
    assert!(responses[0].slots.iter().any(|slot| matches!(
        slot.content,
        shared::protocol::LootContent::Item {
            item_id: 32506,
            count: 1,
            ..
        }
    )));
    assert!(
        persistence::load_inventory(db(&app), client.character_id)
            .items
            .iter()
            .all(|item| item.item_id != 32506)
    );
    assert!(!crate::quest_rules::is_complete_in_log(
        app.world().resource::<QuestDb>(),
        &load_quest_log(db(&app), client.character_id),
        11049,
        |_| 0
    ));
    request(
        &mut app,
        &client,
        LootUnit {
            corpse: egg.to_bits(),
            auto: true,
        },
    );
    assert_eq!(received::<LootFailed>(&mut app, &client), Vec::new());
    assert_eq!(
        persistence::load_inventory(db(&app), client.character_id)
            .items
            .iter()
            .find(|item| item.item_id == 32506)
            .unwrap()
            .stack_count,
        1
    );
    // Reopening a consumed source cannot reroll the egg.
    request(
        &mut app,
        &client,
        UseGameObject {
            object: egg.to_bits(),
        },
    );
    assert!(received::<LootResponse>(&mut app, &client).is_empty());
    turn_in(&mut app, &client, giver, 11049, None);
    assert!(load_quest_log(db(&app), client.character_id).is_rewarded(11049));
}
