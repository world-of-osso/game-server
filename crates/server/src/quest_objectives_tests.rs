//! Authored game-object and spell-credit objectives through server requests.
// TC a352b1fa SpellEffects.cpp: EffectKillCreditPersonal (90), EffectKillCredit (134).
use super::*;

#[path = "world_quest_talk_tests.rs"]
mod world_quest_talk_tests;
use shared::components::{Health, UnitFactionTemplate};
use shared::protocol::{
    CloseInteraction, InteractNpc, InteractionClosed, InteractionFailed, InteractionOpened,
    SelectGossipOption, UseGameObject,
};

fn spawn_quest_group_member(app: &mut App, client: &Client) -> (u64, Entity) {
    use crate::group::{GroupRegistry, OnlineMember, OnlineView};
    let (character, _) = persistence::create_character(db(app), 1, "Creditbuddy", 2, 1).unwrap();
    let log = load_quest_log(db(app), client.character_id);
    save_quest_log(db(app), character, &log);
    let player = app.world().get::<Player>(client.player).unwrap().clone();
    let entity = app
        .world_mut()
        .spawn((
            player,
            ClientId(character),
            Position {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            },
            crate::maps::InMap::continent(0),
        ))
        .id();
    let profile = |entity: Entity, name: &str| OnlineMember {
        entity: entity.to_bits(),
        name: name.into(),
        class: 1,
        level: 80,
        portrait: Default::default(),
        dungeon_difficulty: 1,
        raid_difficulty: Default::default(),
    };
    let online = OnlineView::from([
        (client.character_id, profile(client.player, "Questor")),
        (character, profile(entity, "Creditbuddy")),
    ]);
    let mut groups = GroupRegistry::default();
    groups.sync_presence(&online);
    groups.invite(client.character_id, "Creditbuddy", &online);
    groups.respond_invite(character, true);
    assert!(groups.is_in_same_group(client.character_id, character));
    app.insert_resource(groups);
    (character, entity)
}

fn spell_credit_app() -> App {
    let mut app = setup_app_with(|app| {
        app.add_plugins((
            crate::combat::CombatPlugin,
            crate::spell_cast::SpellCastPlugin,
            crate::auras::AuraPlugin,
        ));
    })
    .expect("authentic quest data required");
    let data = crate::spell_cast::test_harness::world_db().unwrap();
    app.insert_resource(crate::spell_info::SpellStore::fixture(
        [94940, 37221, 37320, 37322, 37323, 89040, 420847]
            .into_iter()
            .map(|id| data.store.get(id).unwrap().clone())
            .collect(),
    ));
    app.insert_resource(crate::spell_cast::test_harness::power_table(data));
    app.insert_resource(
        crate::item_stats::load_item_scaling_tables(&crate::test_data::world_db_path()).unwrap(),
    );
    app.insert_resource(crate::faction::world_db_faction_templates().unwrap());
    app
}

#[test]
fn world_quest_gain_aura_criteria_credits_only_the_authoritative_applied_aura() {
    use crate::persistence::quest_store::{ActiveQuestData, QuestLogData};
    use crate::quest_data::{ObjectiveDef, ObjectiveKind, QuestTemplate};
    let mut app = spell_credit_app();
    let client = spawn_client(&mut app, 80);
    app.world_mut().entity_mut(client.player).insert(Health {
        current: 100.,
        max: 100.,
    });
    enter_world(&mut app, &client);
    {
        let mut quests = app.world_mut().resource_mut::<QuestDb>();
        quests.task_criteria.insert(163995, (69, 420847));
        quests.quests.insert(
            82521,
            QuestTemplate {
                id: 82521,
                objectives: vec![
                    ObjectiveDef {
                        kind: ObjectiveKind::CriteriaTree,
                        object_id: 163995,
                        required: 1,
                        text: "Obtain stacks of Unseeming Shift".into(),
                    },
                    ObjectiveDef {
                        kind: ObjectiveKind::Monster,
                        object_id: 420847,
                        required: 1,
                        text: "Kind isolation".into(),
                    },
                ],
                ..Default::default()
            },
        );
    }
    save_quest_log(
        db(&app),
        client.character_id,
        &QuestLogData {
            active: vec![ActiveQuestData {
                quest_id: 82521,
                counts: vec![0; 2],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    crate::auras::apply_aura(
        app.world_mut(),
        &crate::auras::AuraApplication {
            caster: client.player,
            target: client.player,
            spell_id: 420847,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(
        app.world()
            .get::<crate::auras::ActiveAuras>(client.player)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 420847)
    );
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(82521)
            .unwrap()
            .counts,
        vec![1, 0]
    );
    crate::auras::apply_aura(
        app.world_mut(),
        &crate::auras::AuraApplication {
            caster: client.player,
            target: client.player,
            spell_id: 420847,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(82521)
            .unwrap()
            .counts,
        vec![1, 0]
    );
}

#[test]
fn quest_objectives_real_tear_item_cast_credits_shining_through_the_dark() {
    let mut app = spell_credit_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    let client = spawn_client(&mut app, 80);
    app.world_mut().entity_mut(client.player).insert((
        Health {
            current: 100.0,
            max: 100.0,
        },
        UnitFactionTemplate(1),
        crate::networking::Moving(false),
    ));
    enter_world(&mut app, &client);
    let giver = spawn_npc(&mut app, 47904, 0.0);
    accept(&mut app, &client, giver, 28175);
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    let location = bag_location(&app, &client, 63139); // Alexstrasza's Tear → spell89040
    request(
        &mut app,
        &client,
        UseItem {
            location,
            target: None,
        },
    );
    for _ in 0..120 {
        app.update();
    }
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(28175)
            .unwrap()
            .counts,
        vec![1]
    );
    turn_in(&mut app, &client, giver, 28175, None);
    assert!(load_quest_log(db(&app), client.character_id).is_rewarded(28175));
}

#[test]
fn quest_objectives_unmatched_spell_credit_does_not_require_quest_update_delivery() {
    let mut app = spell_credit_app();
    let client = spawn_client(&mut app, 80);
    app.world_mut().entity_mut(client.player).insert(Health {
        current: 100.0,
        max: 100.0,
    });
    enter_world(&mut app, &client);
    // A spell/catalog consumer without quest-update delivery still resolves the hit.
    // No matching accepted quest means no progress persistence or delivery is needed.
    app.world_mut().remove_resource::<QuestDirty>();
    crate::spell_cast::cast_triggered(
        app.world_mut(),
        client.player,
        94940,
        Some(client.player),
        0,
    );
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active
            .is_empty()
    );
}

#[test]
fn quest_objectives_personal_spell_credit_completes_eye_spy_without_a_kill() {
    let mut app = spell_credit_app();
    let client = spawn_client(&mut app, 80);
    app.world_mut()
        .get_mut::<Player>(client.player)
        .unwrap()
        .race = 2;
    app.world_mut().entity_mut(client.player).insert(Health {
        current: 100.0,
        max: 100.0,
    });
    enter_world(&mut app, &client);
    mark_rewarded(&app, &client, 27954);
    let giver = spawn_npc(&mut app, 46323, 0.0);
    accept(&mut app, &client, giver, 27955);
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    let (buddy, _) = spawn_quest_group_member(&mut app, &client);
    crate::spell_cast::cast_triggered(
        app.world_mut(),
        client.player,
        94940,
        Some(client.player),
        0,
    );
    app.update();
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(27955)
            .unwrap()
            .counts,
        vec![1]
    );
    assert_eq!(
        load_quest_log(db(&app), buddy)
            .active(27955)
            .unwrap()
            .counts,
        vec![0]
    );
    // The credit is capped; another hit cannot award a second count.
    crate::spell_cast::cast_triggered(
        app.world_mut(),
        client.player,
        94940,
        Some(client.player),
        0,
    );
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(27955)
            .unwrap()
            .counts,
        vec![1]
    );
    turn_in(&mut app, &client, giver, 27955, None);
    assert!(load_quest_log(db(&app), client.character_id).is_rewarded(27955));
}

#[test]
fn quest_objectives_event_spell_credit_matches_each_trial_and_error_wand() {
    let mut app = spell_credit_app();
    let client = spawn_client(&mut app, 80);
    app.world_mut()
        .get_mut::<Player>(client.player)
        .unwrap()
        .race = 2;
    app.world_mut().entity_mut(client.player).insert(Health {
        current: 100.0,
        max: 100.0,
    });
    enter_world(&mut app, &client);
    mark_rewarded(&app, &client, 10565);
    let giver = spawn_npc(&mut app, 21496, 0.0);
    accept(&mut app, &client, giver, 10566);
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    let (buddy, buddy_entity) = spawn_quest_group_member(&mut app, &client);
    let target = Entity::try_from_bits(spawn_npc(&mut app, 21709, 0.0)).unwrap();
    for (index, spell) in [37221, 37320, 37322, 37323].into_iter().enumerate() {
        crate::spell_cast::cast_triggered(app.world_mut(), client.player, spell, Some(target), 0);
        app.update();
        let expected = (0..4)
            .map(|objective| u32::from(objective <= index))
            .collect::<Vec<_>>();
        assert_eq!(
            load_quest_log(db(&app), client.character_id)
                .active(10566)
                .unwrap()
                .counts,
            expected
        );
        match index {
            0 => {
                app.world_mut().entity_mut(buddy_entity).insert(Position {
                    x: 100.0,
                    y: 0.0,
                    z: 0.0,
                });
            }
            1 => {
                app.world_mut()
                    .entity_mut(buddy_entity)
                    .insert((origin(), crate::maps::InMap::continent(1)));
            }
            2 => {
                app.world_mut()
                    .entity_mut(buddy_entity)
                    .insert(crate::maps::InMap::continent(0));
            }
            _ => {}
        }
    }
    assert_eq!(
        load_quest_log(db(&app), buddy)
            .active(10566)
            .unwrap()
            .counts,
        vec![1, 0, 0, 1]
    );
    turn_in(&mut app, &client, giver, 10566, None);
    assert!(load_quest_log(db(&app), client.character_id).is_rewarded(10566));
}

fn bag_location(app: &App, client: &Client, item: u32) -> shared::protocol::ItemLocation {
    let model = crate::inventory::load_model(
        db(app),
        app.world().resource::<ItemTemplates>(),
        client.character_id,
    )
    .unwrap();
    let bags = model.inventory_snapshot();
    bags.bags
        .iter()
        .find_map(|bag| {
            bag.items
                .iter()
                .find(|slot| slot.item.item_id == item)
                .map(|slot| shared::protocol::ItemLocation::Bag {
                    bag: bag.bag,
                    slot: slot.slot,
                })
        })
        .expect("authored quest item in bags")
}

#[test]
fn quest_objectives_go_lock_requires_the_authored_power_source() {
    let mut app = object_app();
    let client = object_client(&mut app);
    app.world_mut()
        .get_mut::<Player>(client.player)
        .unwrap()
        .race = 10;
    let giver = spawn_npc(&mut app, 15924, 0.0);
    accept(&mut app, &client, giver, 8889); // Deactivating the Spire
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    let first = real_object(&mut app, 180916);
    request(
        &mut app,
        &client,
        UseGameObject {
            object: first.to_bits(),
        },
    );
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(8889)
            .unwrap()
            .counts[0],
        1
    );
    let source = bag_location(&app, &client, 24337);
    request(
        &mut app,
        &client,
        DestroyItem {
            location: source,
            count: 1,
        },
    );
    let second = real_object(&mut app, 180919);
    request(
        &mut app,
        &client,
        UseGameObject {
            object: second.to_bits(),
        },
    );
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(8889)
            .unwrap()
            .counts[1],
        0,
        "without the Deactivating Jewel the second spire cannot award credit"
    );
}

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
    let (_, rotation, info, bounds, _, _) = crate::game_objects::game_object_bundle(spawn);
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
    // Chest Data8 is quest highlighting metadata, not the goober active-quest gate.
    // Crate of Candles1586 names quest24985, which this character has not accepted.
    let crate_of_candles = real_object(&mut app, 1586);
    request(
        &mut app,
        &client,
        UseGameObject {
            object: crate_of_candles.to_bits(),
        },
    );
    assert_eq!(
        received::<LootResponse>(&mut app, &client).len(),
        1,
        "a chest must not borrow the goober quest gate"
    );
    request(
        &mut app,
        &client,
        LootRelease {
            corpse: crate_of_candles.to_bits(),
        },
    );
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
