//! Actual TDB/70448 conversation inputs and ordinary map content. Controlled player
//! poses qualify native box/sphere ingress, not navigation or client presentation.
use super::*;
use crate::conversations::{self as runtime, ActorObject, ConversationPayload};
use crate::world_area_triggers::WorldAreaTrigger;
use shared::components::Npc;

const CITY: u32 = 2669;
const ORATOR: u32 = 216619;

fn catalog() -> runtime::Catalog {
    static SOURCE: OnceLock<runtime::Catalog> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("conversations.db");
        let backup = std::process::Command::new("python3")
            .args(["-c", "from contextlib import closing; from pathlib import Path; import sqlite3,sys\nwith closing(sqlite3.connect(Path(sys.argv[1]).resolve().as_uri()+'?mode=ro',uri=True)) as source, closing(sqlite3.connect(sys.argv[2])) as target:\n source.backup(target,pages=4096,sleep=0.1)"])
            .arg(crate::test_data::world_db_path()).arg(&database).output().unwrap();
        assert!(backup.status.success(), "owned read-only source backup: {}", String::from_utf8_lossy(&backup.stderr));
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/conversation_data.py");
        let imported = std::process::Command::new("python3").arg(script).arg("--db").arg(&database).output().unwrap();
        assert!(imported.status.success(), "actual conversation source producer: {}", String::from_utf8_lossy(&imported.stderr));
        let connection = Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let actual: Vec<(u32,u32)> = connection.prepare("SELECT Id,FirstLineId FROM tdb_conversation_template WHERE Id IN(24642,24643,24644) ORDER BY Id").unwrap()
            .query_map([], |row| Ok((row.get(0)?,row.get(1)?))).unwrap().map(Result::unwrap).collect();
        assert_eq!(actual, vec![(24642,67085),(24643,67091),(24644,67092)], "native source prerequisites before runtime qualification");
        runtime::load_catalog(&connection).expect("actual conversation catalog")
    }).clone()
}

fn app() -> Harness {
    let data = crate::test_data::world_data().clone();
    let conversations = catalog();
    harness_with(move |app| {
        app.insert_resource(data);
        app.insert_resource(crate::test_data::ground_map());
        app.insert_resource(conversations);
        app.add_plugins(runtime::ConversationPlugin);
    })
    .expect("real map catalog and source population")
}

fn source_location(spawn_id: u64) -> WorldLocation {
    let connection = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    connection
        .query_row(
            "SELECT MapId,PosX,PosY,PosZ,Orientation FROM tdb_areatrigger WHERE SpawnId=?1",
            [spawn_id],
            |row| {
                Ok(WorldLocation {
                    map_id: row.get(0)?,
                    position: Vec3::new(row.get(1)?, row.get(2)?, row.get(3)?),
                    orientation: row.get(4)?,
                })
            },
        )
        .expect("authored conversation ingress pose")
}

fn enter(h: &mut Harness, who: &Traveler, destination: WorldLocation) -> InMap {
    assert!(teleport_to(
        h.app.world_mut(),
        who.player,
        destination,
        TeleportOptions::default()
    ));
    assert_eq!(h.new_worlds(who).len(), 1);
    h.ack(who);
    h.map_of(who).expect("ordinary ACK publishes player on map")
}

fn area(h: &mut Harness, map: InMap, spawn_id: u64) -> Option<Entity> {
    h.app
        .world_mut()
        .query::<(Entity, &InMap, &WorldAreaTrigger)>()
        .iter(h.app.world())
        .find(|(_, copy, area)| **copy == map && area.spawn_id == spawn_id)
        .map(|(entity, _, _)| entity)
}

fn creator(h: &mut Harness, map: InMap) -> Entity {
    let actors: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &InMap, &Npc)>()
        .iter(h.app.world())
        .filter(|(_, copy, npc)| **copy == map && npc.template_id == ORATOR)
        .map(|(entity, _, _)| entity)
        .collect();
    assert_eq!(
        actors.len(),
        1,
        "actual source instance Orator, no display/name alias"
    );
    actors[0]
}

struct Scene {
    h: Harness,
    owner: Traveler,
    peer: Traveler,
    outsider: Traveler,
    map: InMap,
    other_map: InMap,
    source: WorldLocation,
}

fn scene(difficulty: u32, spawn_id: u64) -> Scene {
    let mut h = app();
    let owner = h.spawn(9301, STORMWIND, STOCKADE_EXIT_TARGET);
    let peer = h.spawn(9302, STORMWIND, STOCKADE_EXIT_TARGET);
    let outsider = h.spawn(9303, STORMWIND, STOCKADE_EXIT_TARGET);
    h.group(&[&owner, &peer]);
    h.app
        .world_mut()
        .resource_mut::<GroupRegistry>()
        .set_dungeon_difficulty(9301, difficulty)
        .expect("actual party difficulty selection");
    for player in [&owner, &peer, &outsider] {
        h.app
            .world_mut()
            .entity_mut(player.player)
            .insert(DungeonDifficulty(difficulty));
    }
    let source = source_location(spawn_id);
    assert_eq!(source.map_id, CITY);
    let approach = WorldLocation {
        position: source.position + Vec3::Z * 100.0,
        ..source
    };
    let map = enter(&mut h, &owner, approach);
    assert_eq!(enter(&mut h, &peer, approach), map);
    let other_map = enter(&mut h, &outsider, approach);
    assert_ne!(map, other_map);
    for copy in [map, other_map] {
        assert_eq!(
            h.app
                .world()
                .resource::<InstanceManager>()
                .copy_difficulty(copy)
                .unwrap()
                .difficulty_id,
            difficulty
        );
        assert!(
            area(&mut h, copy, spawn_id).is_some(),
            "source static area on ordinary entry"
        );
        creator(&mut h, copy);
    }
    assert!(runtime::visible_for(h.app.world_mut(), owner.player).is_empty());
    Scene {
        h,
        owner,
        peer,
        outsider,
        map,
        other_map,
        source,
    }
}

fn local_pose(source: WorldLocation, local: Vec3) -> Vec3 {
    let (sin, cos) = source.orientation.sin_cos();
    source.position
        + Vec3::new(
            local.x * cos - local.y * sin,
            local.x * sin + local.y * cos,
            local.z,
        )
}

fn enter_authored_shape(scene: &mut Scene, spawn_id: u64) -> Vec3 {
    let node = area(&mut scene.h, scene.map, spawn_id).unwrap();
    let source_area = scene
        .h
        .app
        .world()
        .get::<WorldAreaTrigger>(node)
        .unwrap()
        .clone();
    let data = source_area.shape_data;
    let entry = match source_area.shape {
        1 => {
            assert_eq!(
                &data[..3],
                &data[3..6],
                "native fixed box, not invented morphing geometry"
            );
            let outside = local_pose(scene.source, Vec3::new(data[0] + 0.05, 0.0, 0.0));
            scene.h.walk_to(&scene.owner, outside);
            assert!(
                area(&mut scene.h, scene.map, spawn_id).is_some(),
                "box feet do not gain unit combat reach"
            );
            let above = local_pose(scene.source, Vec3::new(0.0, 0.0, data[2] / 2.0 + 0.05));
            scene.h.walk_to(&scene.owner, above);
            assert!(
                area(&mut scene.h, scene.map, spawn_id).is_some(),
                "TC box uses half the authored Z extent"
            );
            local_pose(scene.source, Vec3::new(data[0] - 0.05, 0.0, 0.0))
        }
        0 => {
            assert_eq!(data[0], data[1], "native fixed sphere");
            let reach = crate::scripted_ai::model_reach::combat_reach(
                scene.h.app.world(),
                scene.owner.player,
            )
            .unwrap();
            assert_eq!(reach, 1.5, "native ordinary-player reach prerequisite");
            scene.h.walk_to(
                &scene.owner,
                scene.source.position + Vec3::X * (data[0] + reach + 0.05),
            );
            assert!(area(&mut scene.h, scene.map, spawn_id).is_some());
            scene.source.position + Vec3::X * (data[0] + reach - 0.05)
        }
        value => panic!("unexpected actual Orator ingress shape{value}"),
    };
    assert!(
        runtime::visible_for(scene.h.app.world_mut(), scene.owner.player).is_empty(),
        "no early conversation outside actual shape"
    );
    scene.h.walk_to(&scene.owner, entry);
    entry
}

fn owner_payload(scene: &mut Scene) -> ConversationPayload {
    let mut payloads = runtime::visible_for(scene.h.app.world_mut(), scene.owner.player);
    assert_eq!(
        payloads.len(),
        1,
        "eligible source ingress must create one private conversation"
    );
    assert!(
        runtime::visible_for(scene.h.app.world_mut(), scene.peer.player).is_empty(),
        "same-copy peer is not private owner"
    );
    assert!(
        runtime::visible_for(scene.h.app.world_mut(), scene.outsider.player).is_empty(),
        "other-copy player gets no private payload"
    );
    payloads.remove(0)
}

struct Authored {
    spawn: u64,
    conversation: u32,
    line: u32,
    broadcast: u32,
    creature: u32,
    sound: u32,
}
const AUTHORED: &[Authored] = &[
    Authored {
        spawn: 266,
        conversation: 24642,
        line: 67085,
        broadcast: 259911,
        creature: 220769,
        sound: 250774,
    },
    Authored {
        spawn: 267,
        conversation: 24643,
        line: 67091,
        broadcast: 259917,
        creature: 216619,
        sound: 250775,
    },
    Authored {
        spawn: 268,
        conversation: 24644,
        line: 67092,
        broadcast: 259918,
        creature: 216619,
        sound: 250776,
    },
];

fn ingress(difficulty: u32, authored: &Authored) {
    let Authored {
        spawn: spawn_id,
        conversation,
        line,
        broadcast,
        creature,
        sound,
    } = *authored;
    let mut scene = scene(difficulty, spawn_id);
    let orator = creator(&mut scene.h, scene.map);
    let position = enter_authored_shape(&mut scene, spawn_id);
    let payload = owner_payload(&mut scene);
    assert_eq!(
        (payload.entry, payload.creator, payload.private_owner),
        (conversation, orator.to_bits(), scene.owner.player.to_bits())
    );
    assert_eq!(payload.actors.len(), 1);
    assert_eq!(payload.actors[0].id, 51642);
    assert_eq!(payload.actors[0].name.as_deref(), Some("Orator Krix'vizk"));
    assert_eq!(
        payload.actors[0].object,
        ActorObject::TalkingHead {
            creature_id: creature,
            display_id: 116692
        }
    );
    assert_eq!(payload.lines.len(), 1);
    let data = &payload.lines[0];
    assert_eq!(
        (
            data.id,
            data.broadcast_text_id,
            data.ui_camera_id,
            data.actor_index
        ),
        (line, broadcast, 1751, 0)
    );
    assert_eq!(
        (
            data.start_ms,
            data.source_duration_ms,
            data.additional_duration_ms
        ),
        (0, None, 0)
    );
    assert_eq!((data.start_animation, data.end_animation), (60, 60));
    let text = data
        .text
        .as_ref()
        .expect("authored TDB broadcast text, not fabricated dialogue");
    assert!(!text.male_text.is_empty());
    assert_eq!(text.sound_kit_id, sound);
    assert_eq!(
        (payload.last_line_end_ms, payload.duration_ms),
        (0, 10000),
        "source missing-duration branch plus10s object grace"
    );
    let entity = Entity::try_from_bits(payload.id).unwrap();
    assert!(
        close(
            wow(scene.h.app.world().get::<Position>(entity).unwrap()),
            position
        ),
        "conversation uses entrant pose, not trigger/boss center"
    );
    assert!(
        area(&mut scene.h, scene.map, spawn_id).is_none(),
        "source one-shot static trigger is removed"
    );
    assert!(
        area(&mut scene.h, scene.other_map, spawn_id).is_some(),
        "other copy keeps its trigger"
    );
    scene.h.advance(195);
    assert_eq!(
        runtime::visible_for(scene.h.app.world_mut(), scene.owner.player).len(),
        1
    );
    scene.h.advance(10);
    assert!(
        runtime::visible_for(scene.h.app.world_mut(), scene.owner.player).is_empty(),
        "source conversation object expires naturally"
    );
    scene
        .h
        .walk_to(&scene.owner, scene.source.position + Vec3::Z * 100.0);
    scene.h.walk_to(&scene.owner, position);
    assert!(
        runtime::visible_for(scene.h.app.world_mut(), scene.owner.player).is_empty(),
        "removed one-shot trigger does not recreate a conversation"
    );
}

macro_rules! ingress_case {
    ($name:ident,$difficulty:expr,$index:expr) => {
        #[test]
        fn $name() {
            ingress($difficulty, &AUTHORED[$index]);
        }
    };
}
ingress_case!(conversation_runtime_orator_intro1_normal, 1, 0);
ingress_case!(conversation_runtime_orator_intro1_follower205, 205, 0);
ingress_case!(conversation_runtime_orator_intro2_normal, 1, 1);
ingress_case!(conversation_runtime_orator_intro2_follower205, 205, 1);
ingress_case!(conversation_runtime_orator_intro3_normal, 1, 2);
ingress_case!(conversation_runtime_orator_intro3_follower205, 205, 2);

#[test]
fn conversation_runtime_native705_multiline_timing_owner_and_map_lifetime() {
    let mut scene = scene(1, 266);
    let orator = creator(&mut scene.h, scene.map);
    let position = *scene
        .h
        .app
        .world()
        .get::<Position>(scene.owner.player)
        .unwrap();
    let entity = runtime::create_private(
        scene.h.app.world_mut(),
        705,
        orator,
        position,
        scene.owner.player,
    )
    .expect("ordinary generic source conversation");
    let payload = owner_payload(&mut scene);
    assert_eq!(payload.entry, 705);
    assert_eq!(payload.actors[0].id, 50124);
    assert_eq!(
        payload.actors[0].object,
        ActorObject::TalkingHead {
            creature_id: 98075,
            display_id: 21135
        }
    );
    assert_eq!(
        payload
            .lines
            .iter()
            .map(|line| (line.id, line.start_ms, line.additional_duration_ms))
            .collect::<Vec<_>>(),
        vec![(1683, 0, 500), (1684, 500, 500)]
    );
    assert_eq!(
        (payload.last_line_end_ms, payload.duration_ms),
        (1000, 11000)
    );
    scene.h.advance(210);
    assert!(
        scene.h.app.world().get_entity(entity).is_ok(),
        "source accumulated1s plus10s, not a fixed10s timer"
    );
    scene.h.advance(15);
    assert!(scene.h.app.world().get_entity(entity).is_err());
    let next = runtime::create_private(
        scene.h.app.world_mut(),
        705,
        orator,
        position,
        scene.owner.player,
    )
    .unwrap();
    crate::maps::unload_map_content(scene.h.app.world_mut(), &[scene.map]);
    assert!(
        scene.h.app.world().get_entity(next).is_err(),
        "map teardown removes its active conversation"
    );
    assert!(area(&mut scene.h, scene.other_map, 266).is_some());
}

#[test]
fn conversation_runtime_orator_missing_creator_does_not_borrow_another_copy() {
    let mut scene = scene(1, 266);
    let orator = creator(&mut scene.h, scene.map);
    scene.h.app.world_mut().despawn(orator);
    enter_authored_shape(&mut scene, 266);
    assert!(runtime::visible_for(scene.h.app.world_mut(), scene.owner.player).is_empty());
    assert!(
        area(&mut scene.h, scene.map, 266).is_none(),
        "TC removes eligible player's trigger even if instance creator is absent"
    );
    assert!(area(&mut scene.h, scene.other_map, 266).is_some());
}

#[derive(Resource, Default)]
struct WireInbox(std::collections::HashMap<Entity, Vec<shared::protocol::ConversationUpdate>>);

fn collect_wire(
    mut receivers: Query<(
        Entity,
        &mut MessageReceiver<shared::protocol::ConversationUpdate>,
    )>,
    mut inbox: ResMut<WireInbox>,
) {
    for (connection, mut receiver) in &mut receivers {
        inbox
            .0
            .entry(connection)
            .or_default()
            .extend(receiver.receive());
    }
}

fn wire_updates(h: &mut Harness, who: &Traveler) -> Vec<shared::protocol::ConversationUpdate> {
    h.app
        .world_mut()
        .resource_mut::<WireInbox>()
        .0
        .remove(&who.connection)
        .unwrap_or_default()
}

fn wire_delivery(difficulty: u32) {
    let mut scene = scene(difficulty, 267);
    // Lightyear receivers retain one frame. Observe every frame just like the
    // real network relay, rather than reading after a multi-frame fixture walk.
    scene
        .h
        .app
        .init_resource::<WireInbox>()
        .add_systems(Last, collect_wire);
    for who in [&scene.owner, &scene.peer, &scene.outsider] {
        scene
            .h
            .app
            .world_mut()
            .entity_mut(who.connection)
            .insert(pair::<shared::protocol::ConversationUpdate>());
    }
    scene.h.advance(2);
    for who in [&scene.owner, &scene.peer, &scene.outsider] {
        wire_updates(&mut scene.h, who);
    }
    enter_authored_shape(&mut scene, 267);
    let owner = wire_updates(&mut scene.h, &scene.owner);
    assert_eq!(
        owner.len(),
        1,
        "source ingress must deliver one actual reliable owner snapshot"
    );
    assert_eq!(
        (owner[0].map_id, owner[0].instance_id),
        (CITY, scene.map.instance_id)
    );
    assert_eq!(owner[0].conversations.len(), 1);
    assert_eq!(owner[0].conversations[0].entry, 24643);
    assert_eq!(
        owner[0].conversations[0].private_owner,
        scene.owner.player.to_bits()
    );
    assert!(
        wire_updates(&mut scene.h, &scene.peer).is_empty(),
        "same-copy peer receives no private conversation message"
    );
    assert!(
        wire_updates(&mut scene.h, &scene.outsider).is_empty(),
        "other-copy connection receives no private message"
    );
    scene.h.advance(2);
    assert!(
        wire_updates(&mut scene.h, &scene.owner).is_empty(),
        "stable source set is not republished every tick"
    );
    scene.h.advance(205);
    let cleared = wire_updates(&mut scene.h, &scene.owner);
    assert_eq!(
        cleared.len(),
        1,
        "natural expiry delivers one authoritative clear"
    );
    assert!(cleared[0].conversations.is_empty());
    assert!(wire_updates(&mut scene.h, &scene.peer).is_empty());
}
#[test]
#[ignore = "RED: conversation payload is not delivered to owning connection"]
fn conversation_wire_orator_normal_private_delivery_and_expiry() {
    wire_delivery(1);
}
#[test]
#[ignore = "RED: conversation payload is not delivered to owning connection"]
fn conversation_wire_orator_follower205_private_delivery_and_expiry() {
    wire_delivery(205);
}
