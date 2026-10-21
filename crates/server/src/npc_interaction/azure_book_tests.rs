use super::*;
use crate::maps::{DungeonDifficulty, InMap, InstanceManager};
use shared::components::MovementControl;

fn fixture() -> Harness {
    let mut h = harness(HUMAN_TEMPLATE).unwrap();
    let maps = crate::maps::store::load_map_store(&h.conn).unwrap();
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(
            &maps,
            2515,
            crate::maps::instances::InstanceEntrant {
                character_id: 1,
                leader: 1,
                difficulty_id: 205,
                ..default()
            },
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 1_792_454_400,
            },
        )
        .unwrap()
        .map;
    h.app.insert_resource(maps);
    h.app.insert_resource(instances);
    h.app.insert_resource(crate::world_data::load_world_data(
        &crate::test_data::world_db_path(),
    ));
    h.app.world_mut().entity_mut(h.player).insert((
        copy,
        DungeonDifficulty(205),
        crate::persistence::ClientId(1),
        MovementControl::default(),
    ));
    h
}

#[test]
fn azure_translocation_book_gossip_moves_to_four_authored_receiving_books() {
    let mut h = fixture();
    let copy = *h.app.world().get::<InMap>(h.player).unwrap();
    for (entry, menu_id, target_guid, destination_text) in [
        (194602, 28961, 29001870, "top ring"),
        (194712, 29009, 29001886, "middle ring"),
        (194713, 29011, 29001915, "Mausoleum of Legends Entrance"),
        (194714, 29012, 29002013, "bottom ring"),
    ] {
        let book = h.spawn_real_npc(entry, 1.0);
        h.app.world_mut().entity_mut(book).insert(copy);
        h.app.world_mut().entity_mut(h.player).insert(copy);
        let (x, y, z): (f32, f32, f32) = h
            .conn
            .query_row(
                "SELECT position_x,position_y,position_z FROM content_creature WHERE guid=?",
                [target_guid],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        h.interact(book);
        let events = h.take::<InteractionOpened>();
        let InteractionKind::Gossip(menu) = &events.last().expect("native book gossip").kind else {
            panic!("book must open gossip");
        };
        assert_eq!(menu.menu_id, menu_id);
        assert!(
            menu.text.contains(destination_text),
            "native destination text: {menu:?}"
        );
        assert_eq!(
            menu.options.len(),
            1,
            "native Proceed onward option must be actionable"
        );
        assert_eq!(menu.options[0].text, "Proceed onward.");
        let before = *h.app.world().get::<MovementControl>(h.player).unwrap();
        h.select(book, menu.options[0].option_id);
        let position = h.app.world().get::<Position>(h.player).unwrap();
        assert_eq!((position.x, position.y, position.z), (x, z, -y));
        assert_eq!(h.app.world().get::<InMap>(h.player), Some(&copy));
        assert_eq!(
            h.app
                .world()
                .get::<MovementControl>(h.player)
                .unwrap()
                .epoch,
            before.epoch + 1
        );
        assert!(
            h.take::<InteractionClosed>()
                .iter()
                .any(|event| event.npc == book.to_bits())
        );
    }
}

#[test]
fn azure_translocation_book_selection_revalidates_source_range() {
    let mut h = fixture();
    let copy = *h.app.world().get::<InMap>(h.player).unwrap();
    let book = h.spawn_real_npc(194602, 1.0);
    h.app.world_mut().entity_mut(book).insert(copy);
    h.app.world_mut().entity_mut(h.player).insert(copy);
    h.interact(book);
    let events = h.take::<InteractionOpened>();
    let InteractionKind::Gossip(menu) = &events.last().unwrap().kind else {
        panic!("book gossip")
    };
    let option = menu
        .options
        .first()
        .expect("native Proceed onward")
        .option_id;
    h.place_player(book, INTERACTION_DISTANCE + 10.0);
    let before = *h.app.world().get::<Position>(h.player).unwrap();
    h.select(book, option);
    assert_eq!(*h.app.world().get::<Position>(h.player).unwrap(), before);
    assert!(
        h.take::<InteractionFailed>()
            .iter()
            .any(|event| event.error == InteractionError::TooFarAway)
    );
}
