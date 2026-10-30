use super::*;
use crate::maps::{DungeonDifficulty, InMap, InstanceManager};
use shared::components::MovementControl;

#[test]
#[ignore = "requires native map2825 navigation assets"]
fn nalorakk_pyre_landings_connect_to_entrance_and_hoardmonger() {
    let root = std::path::PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
    let mut nav = ::navigation::NavigationMap::open(&root.join("2825")).unwrap();
    let lower = Vec3::new(4484.93, 13.4853, 4906.93);
    let upper = Vec3::new(4339.33, 104.31152, 4947.34);
    for point in [lower, upper] {
        let projected = nav.complete_walk_path(point, point).unwrap();
        assert!(
            projected[0].distance(point) < 0.01,
            "landing must be on the detail mesh"
        );
    }
    for (start, goal) in [
        (Vec3::new(4410.67, 21.4535, 4919.94), lower),
        (upper, Vec3::new(4242.39, 109.04, 5250.65)),
        (lower, Vec3::new(5015.96, -23.4112, 4846.0)),
    ] {
        let mut cursor = start;
        let mut plan = ::navigation::NavigationPlan::default();
        let mut reached = false;
        for _ in 0..100 {
            let segment = nav.path_with_plan(cursor, goal, &mut plan).unwrap();
            let endpoint = *segment.points.last().unwrap();
            if segment.complete {
                assert!(endpoint.distance(goal) < 2.0);
                reached = true;
                break;
            }
            assert!(
                endpoint.distance(cursor) > 0.01,
                "route stopped before destination"
            );
            cursor = endpoint;
        }
        assert!(reached, "route did not reach goal");
    }
}

#[test]
fn nalorakk_pyre_gossip_moves_owner_and_owned_follower_in_both_directions() {
    let mut h = harness(HUMAN_TEMPLATE).unwrap();
    let maps = crate::maps::store::load_map_store(&h.conn).unwrap();
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(
            &maps,
            2825,
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
    h.app.world_mut().entity_mut(h.player).insert((
        copy,
        DungeonDifficulty(205),
        crate::persistence::ClientId(1),
        MovementControl::default(),
    ));
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert(pair::<UseGameObject>());
    let follower = h
        .app
        .world_mut()
        .spawn((
            copy,
            Position {
                x: 4480.69,
                y: 13.435155,
                z: 4912.04,
            },
            crate::followers::Follower {
                owner: h.player,
                profile: crate::followers::Profile {
                    entry: 198595,
                    name: "Garrick".into(),
                    class_id: 1,
                    spec_id: 73,
                    role: shared::group::GroupRole::Tank,
                    attack_spell: 0,
                    cooldown_spell: 0,
                    heal_spell: 0,
                    taunt_spell: 0,
                    form_spell: 0,
                    melee: true,
                },
            },
        ))
        .id();
    let mut companions = Vec::new();
    for owner in [h.player, follower] {
        let pet = h
            .app
            .world_mut()
            .spawn((
                copy,
                Position {
                    x: 4480.69,
                    y: 13.435155,
                    z: 4912.04,
                },
                shared::components::Health {
                    current: 100.0,
                    max: 100.0,
                },
                crate::hunter_pet::HunterPet { owner, slot: 0 },
            ))
            .id();
        let guardian = h
            .app
            .world_mut()
            .spawn((
                copy,
                Position {
                    x: 4480.69,
                    y: 13.435155,
                    z: 4912.04,
                },
                shared::components::Health {
                    current: 100.0,
                    max: 100.0,
                },
                crate::spell_cast::summon::SummonedCreature {
                    owner,
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
            ))
            .id();
        companions.extend([pet, guardian]);
    }
    for (entry, menu, target) in [
        (616377, 40087, (4339.33, 104.31152, 4947.34)),
        (613266, 40088, (4484.93, 13.4853, 4906.93)),
    ] {
        let (go_type, display_id, name, scale, x, y, z): (u8, u32, String, f32, f32, f32, f32) = h.conn.query_row(
            "SELECT t.type,t.displayId,t.name,t.size,g.position_x,g.position_y,g.position_z FROM tdb_gameobject_template t JOIN tdb_gameobject g ON g.id=t.entry WHERE t.entry=?1 AND g.map=2825", [entry],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))
        ).unwrap();
        let bounds = h.conn.query_row(
            "SELECT GeoBox_0,GeoBox_1,GeoBox_2,GeoBox_3,GeoBox_4,GeoBox_5 FROM game_object_display_info WHERE ID=?1", [display_id],
            |r| Ok(GameObjectBounds { min: Vec3::new(r.get(0)?,r.get(1)?,r.get(2)?), max: Vec3::new(r.get(3)?,r.get(4)?,r.get(5)?), scale, orientation: 0.0 })
        ).unwrap();
        let pyre = h
            .app
            .world_mut()
            .spawn((
                GameObjectInfo {
                    entry,
                    go_type,
                    display_id,
                    name,
                    scale,
                },
                bounds,
                Position { x, y: z, z: -y },
                copy,
            ))
            .id();
        h.place_player(pyre, 1.0);
        let before = *h.app.world().get::<Position>(h.player).unwrap();
        h.send::<_, InteractionChannel>(UseGameObject {
            object: pyre.to_bits(),
        });
        assert_eq!(
            *h.app.world().get::<Position>(h.player).unwrap(),
            before,
            "opening gossip must not teleport"
        );
        let opened = h.take::<InteractionOpened>();
        let InteractionKind::Gossip(gossip) = &opened.last().unwrap().kind else {
            panic!("pyre opens gossip")
        };
        assert_eq!(gossip.menu_id, menu);
        assert_eq!(gossip.options.len(), 1);
        assert_eq!(
            gossip.options[0].text,
            "<Meditate on the sound of the flames.>"
        );
        h.select(pyre, 99);
        assert_eq!(
            *h.app.world().get::<Position>(h.player).unwrap(),
            before,
            "unknown option must not teleport"
        );
        let epoch = h
            .app
            .world()
            .get::<MovementControl>(h.player)
            .unwrap()
            .epoch;
        h.select(pyre, gossip.options[0].option_id);
        for unit in [h.player, follower]
            .into_iter()
            .chain(companions.iter().copied())
        {
            let pos = h.app.world().get::<Position>(unit).unwrap();
            assert_eq!((pos.x, pos.y, pos.z), target);
            assert_eq!(h.app.world().get::<InMap>(unit), Some(&copy));
        }
        assert_eq!(
            h.app
                .world()
                .get::<MovementControl>(h.player)
                .unwrap()
                .epoch,
            epoch + 1
        );
        assert!(
            h.take::<InteractionClosed>()
                .iter()
                .any(|event| event.npc == pyre.to_bits())
        );
    }
}
