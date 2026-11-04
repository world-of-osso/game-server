use super::*;
use crate::maps::{InMap, InstanceManager};
use crate::scripted_ai::instance_script::{self, EncounterState};

#[test]
fn temple_avatar_gossip_starts_only_its_encounter_copy() {
    let mut h = harness(HUMAN_TEMPLATE).unwrap();
    // Shadow only the candidate tables in this read-only fixture connection.
    h.conn.execute_batch("\
        CREATE TEMP TABLE content_creature_template AS SELECT * FROM main.content_creature_template; \
        CREATE TEMP TABLE content_gossip_menu AS SELECT * FROM main.content_gossip_menu; \
        CREATE TEMP TABLE content_gossip_menu_option AS SELECT * FROM main.content_gossip_menu_option; \
        UPDATE content_creature_template SET gossip_menu_id=22561 WHERE entry=133392; \
        INSERT INTO content_gossip_menu (MenuID,TextID) VALUES (22561,34714); \
        INSERT INTO content_gossip_menu_option (MenuID,OptionID,OptionIcon,OptionText,OptionBroadcastTextID,OptionType,OptionNpcFlag,ActionMenuID) \
        SELECT MenuID,OptionID,0,OptionText,OptionBroadcastTextID,1,1,0 FROM tdb_gossip_menu_option WHERE MenuID=22561 AND OptionID=0;").unwrap();
    let mut maps = crate::maps::store::load_map_store(&h.conn).unwrap();
    maps.encounter_heal_objectives.insert((1877, 133392), 25.0);
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(
            &maps,
            1877,
            crate::maps::instances::InstanceEntrant {
                character_id: 1,
                leader: 1,
                difficulty_id: 8,
                ..default()
            },
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 1_792_454_400,
            },
        )
        .unwrap()
        .map;
    h.app.insert_resource(load_gossip_catalog(&h.conn).unwrap());
    h.app.insert_resource(maps);
    h.app.insert_resource(instances);
    let avatar = h.spawn_real_npc(133392, 46.31);
    h.app.world_mut().entity_mut(h.player).insert(copy);
    h.app.world_mut().entity_mut(avatar).insert(copy);
    assert_eq!(
        crate::scripted_ai::model_reach::combat_reach(h.app.world(), avatar),
        Some(20.4)
    );
    h.interact(avatar);
    assert_eq!(
        h.take::<InteractionFailed>(),
        vec![InteractionFailed {
            npc: avatar.to_bits(),
            error: InteractionError::TooFarAway,
        }]
    );
    h.place_player(avatar, 15.17);
    h.app.world_mut().entity_mut(h.player).insert(copy);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), copy, 3),
        Some(EncounterState::NotStarted)
    );
    h.interact(avatar);
    let opened = h.take::<InteractionOpened>();
    let InteractionKind::Gossip(menu) = &opened.last().unwrap().kind else {
        panic!("Avatar gossip expected")
    };
    assert_eq!(menu.options[0].text, "We will restore you!");
    h.select(avatar, menu.options[0].option_id);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), copy, 3),
        Some(EncounterState::InProgress)
    );
    let other = InMap {
        instance_id: copy.instance_id + 1,
        ..copy
    };
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), other, 3),
        Some(EncounterState::NotStarted)
    );
}
