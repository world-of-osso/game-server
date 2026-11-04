use super::*;
use crate::maps::InstanceManager;
use crate::scripted_ai::instance_script::{self, EncounterState};
use crate::test_support::objectives::{self, InteractionStep};

#[test]
fn healing_objective_uses_native_gossip_then_waits_for_healing() {
    let mut h = harness(HUMAN_TEMPLATE).unwrap();
    h.conn.execute_batch("\
        CREATE TEMP TABLE content_creature_template AS SELECT * FROM main.content_creature_template; \
        CREATE TEMP TABLE content_gossip_menu AS SELECT * FROM main.content_gossip_menu; \
        CREATE TEMP TABLE content_gossip_menu_option AS SELECT * FROM main.content_gossip_menu_option; \
        UPDATE content_creature_template SET gossip_menu_id=22561 WHERE entry=133392; \
        DELETE FROM content_gossip_menu WHERE MenuID=22561; \
        DELETE FROM content_gossip_menu_option WHERE MenuID=22561; \
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
    let avatar = h.spawn_real_npc(133392, 47.0);
    h.app.world_mut().entity_mut(h.player).insert(copy);
    h.app.world_mut().entity_mut(avatar).insert(copy);
    assert!(
        matches!(objectives::next(h.app.world_mut(), h.player, avatar).unwrap(),
        Some(InteractionStep::Approach { target }) if target == avatar)
    );
    assert!(objectives::interact(h.app.world_mut(), h.player, avatar).is_err());
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), copy, 3),
        Some(EncounterState::NotStarted)
    );
    h.place_player(avatar, 15.17);
    h.app.world_mut().entity_mut(h.player).insert(copy);
    assert!(
        matches!(objectives::next(h.app.world_mut(), h.player, avatar).unwrap(),
        Some(InteractionStep::Interact { target }) if target == avatar)
    );
    objectives::interact(h.app.world_mut(), h.player, avatar).unwrap();
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), copy, 3),
        Some(EncounterState::InProgress)
    );
    assert!(matches!(
        objectives::next(h.app.world_mut(), h.player, avatar).unwrap(),
        Some(InteractionStep::Wait)
    ));
}
