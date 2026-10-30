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
fn azure_simulator_registrations_use_real_book_transport() {
    let mut h = fixture();
    let copy = *h.app.world().get::<InMap>(h.player).unwrap();
    let registrations = crate::test_support::transport::read_registrations(2515, &h.conn).unwrap();
    assert_eq!(registrations.len(), 9);
    for registration in registrations {
        let book = h.spawn_real_npc(registration.entry, 1.0);
        h.app.world_mut().entity_mut(book).insert(copy);
        let edge = crate::test_support::graph::TransportEdge {
            from: 0,
            to: 1,
            source_guid: 0,
            radius: registration.radius,
            departure: None,
            landing: registration.destination.to_array(),
            action: registration.action,
        };
        crate::test_support::transport::activate(h.app.world_mut(), h.player, Some(book), &edge)
            .unwrap();
        let position = h.app.world().get::<Position>(h.player).unwrap();
        assert_eq!(
            Vec3::new(position.x, position.y, position.z),
            registration.destination
        );
    }
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
fn azure_translocation_moves_all_living_owned_pets_in_the_same_copy() {
    let mut h = fixture();
    let copy = *h.app.world().get::<InMap>(h.player).unwrap();
    let book = h.spawn_real_npc(194602, 1.0);
    let first = h.spawn_real_npc(187159, 1.0);
    let second = h.spawn_real_npc(187159, 1.0);
    let remote = h.spawn_real_npc(187159, 1.0);
    let guardian = h.spawn_real_npc(187159, 1.0);
    h.app.world_mut().entity_mut(guardian).insert((
        copy,
        crate::spell_cast::summon::SummonedCreature {
            owner: h.player,
            spell_id: 104316,
            properties_id: 61,
            properties: crate::spell_cast::summon::SummonProperties {
                control: 0,
                faction: 0,
                title: 2,
                slot: 0,
                flags: 0,
            },
            display_scale: 1.0,
            expires_at: None,
        },
    ));
    h.app.world_mut().entity_mut(book).insert(copy);
    for pet in [first, second, remote] {
        h.app
            .world_mut()
            .entity_mut(pet)
            .insert(crate::hunter_pet::HunterPet {
                owner: h.player,
                slot: 1,
            });
    }
    for pet in [first, second] {
        h.app.world_mut().entity_mut(pet).insert(copy);
    }
    h.app.world_mut().entity_mut(remote).insert(InMap {
        instance_id: copy.instance_id + 1,
        ..copy
    });
    let remote_before = *h.app.world().get::<Position>(remote).unwrap();
    h.place_player(book, 1.0);
    h.interact(book);
    let events = h.take::<InteractionOpened>();
    let InteractionKind::Gossip(menu) = &events.last().unwrap().kind else {
        panic!("book gossip")
    };
    h.select(book, menu.options[0].option_id);
    let endpoint = *h.app.world().get::<Position>(h.player).unwrap();
    assert_eq!(h.app.world().get::<Position>(first), Some(&endpoint));
    assert_eq!(h.app.world().get::<Position>(second), Some(&endpoint));
    assert_eq!(h.app.world().get::<Position>(guardian), Some(&endpoint));
    assert_eq!(h.app.world().get::<Position>(remote), Some(&remote_before));
}

#[test]
fn azure_translocation_final_book_uses_qualified_authored_poi_inside_umbrelskul_component() {
    let mut h = fixture();
    let copy = *h.app.world().get::<InMap>(h.player).unwrap();
    let book = h.spawn_real_npc(194715, 1.0);
    h.app.world_mut().entity_mut(book).insert(copy);
    h.app.world_mut().entity_mut(h.player).insert(copy);
    h.interact(book);
    let events = h.take::<InteractionOpened>();
    let InteractionKind::Gossip(menu) = &events.last().unwrap().kind else {
        panic!("book gossip")
    };
    assert!(menu.text.contains("Sindragosa's Lament"));
    h.select(
        book,
        menu.options
            .first()
            .expect("native Proceed onward")
            .option_id,
    );
    let at = h.app.world().get::<Position>(h.player).unwrap();
    // Authenticated70448 AreaPOI7116, not a guessed boss offset.
    assert_eq!((at.x, at.y, at.z), (-5364.33008, 157.716995, -1035.76001));
    assert_eq!(h.app.world().get::<InMap>(h.player), Some(&copy));
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
        h.take::<InteractionClosed>()
            .iter()
            .any(|event| event.npc == book.to_bits())
    );
}
