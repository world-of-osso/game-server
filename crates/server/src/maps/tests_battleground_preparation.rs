use super::*;
use shared::protocol::GameObjectCollision;

#[test]
#[ignore = "RED: ordinary BG starts immediately without source preparation interval"]
fn bg_preparation_concrete_full_roster_waits_and_blocks_objectives_until_source_deadline() {
    assert_source_preparation(false);
}

#[test]
#[ignore = "RED: ordinary BG starts immediately without source preparation interval"]
fn bg_preparation_random_full_roster_waits_and_blocks_objectives_until_source_deadline() {
    assert_source_preparation(true);
}

fn source_gate_spawns(map_id: u32, entries: &[u32]) -> Vec<crate::game_objects::GameObjectSpawn> {
    use crate::game_objects::{GameObjectBounds, GameObjectSpawn};
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut rows = db.prepare("SELECT g.guid,g.id,g.position_x,g.position_y,g.position_z,g.orientation,g.state,t.type,t.displayId,t.name,t.size,d.GeoBox_0,d.GeoBox_1,d.GeoBox_2,d.GeoBox_3,d.GeoBox_4,d.GeoBox_5 FROM content_gameobject g JOIN content_gameobject_template t ON t.entry=g.id JOIN game_object_display_info d ON d.ID=t.displayId WHERE g.map=? ORDER BY g.guid").unwrap();
    rows.query_map([map_id], |r| {
        let orientation = r.get::<_, f64>(5)? as f32;
        let scale = r.get::<_, f64>(10)? as f32;
        Ok(GameObjectSpawn {
            guid: r.get(0)?,
            map_id,
            info: GameObjectInfo {
                entry: r.get(1)?,
                go_type: r.get(7)?,
                display_id: r.get(8)?,
                name: r.get(9)?,
                scale,
            },
            position: Vec3::new(
                r.get::<_, f64>(2)? as f32,
                r.get::<_, f64>(3)? as f32,
                r.get::<_, f64>(4)? as f32,
            ),
            orientation,
            initial_state: r.get(6)?,
            bounds: GameObjectBounds {
                min: Vec3::new(
                    r.get::<_, f64>(11)? as f32,
                    r.get::<_, f64>(12)? as f32,
                    r.get::<_, f64>(13)? as f32,
                ),
                max: Vec3::new(
                    r.get::<_, f64>(14)? as f32,
                    r.get::<_, f64>(15)? as f32,
                    r.get::<_, f64>(16)? as f32,
                ),
                scale,
                orientation,
            },
            spawn_difficulties: vec![0],
            chair: None,
            quest_object: None,
            script_name: String::new(),
        })
    })
    .unwrap()
    .map(Result::unwrap)
    .filter(|spawn| entries.contains(&spawn.info.entry))
    .collect()
}

fn assert_source_preparation(random: bool) {
    let template = shared::battleground::arathi_basin();
    let gates = source_gate_spawns(template.map_id, &[322273, 322274, 180322]);
    assert_eq!(
        gates.len(),
        4,
        "two authored faction doors and two ghost gates"
    );
    let first_arrival = std::rc::Rc::new(std::cell::Cell::new(None));
    let capture = first_arrival.clone();
    let mut entered = enter_source_match_with_arrival(
        &template,
        node_rules(template.map_id).unwrap().starts,
        |app| {
            app.insert_resource(crate::pvp::random_bg::tests::source_catalog(&[1018]));
            app.world_mut()
                .resource_mut::<GameObjectSpawns>()
                .0
                .extend(gates);
        },
        if random {
            SourceQueueFixture::RandomFourPartiesOfFive
        } else {
            SourceQueueFixture::FourPartiesOfFiveAt25
        },
        move |h, _, index| {
            if index == 0 {
                capture.set(Some(
                    h.app.world().resource::<Time<Fixed>>().elapsed_secs_f64(),
                ));
            }
        },
    );
    let actor = &entered.players[0];
    let h = &mut entered.h;
    assert_eq!(
        battleground(h, actor).phase,
        BgPhase::Waiting,
        "full roster must not bypass source120s preparation"
    );
    for entry in [322273, 322274] {
        let (door, _) = objects_in(h, entered.copy, entry)[0];
        assert_eq!(
            h.app.world().get::<GameObjectCollision>(door),
            Some(&GameObjectCollision(true)),
            "authored preparation gate stays closed"
        );
    }
    let (banner, at) = objects_in(
        h,
        entered.copy,
        source_battleground_objects(template.map_id).0[0].info.entry,
    )[0];
    let before = battleground(h, actor).world_states.clone();
    h.walk_to(actor, at);
    use_object(h, actor, banner);
    assert_eq!(
        battleground(h, actor).world_states,
        before,
        "objective use blocked during preparation"
    );
    use_second_frames(h);
    let start = first_arrival.get().unwrap();
    for remaining in [60.0, 30.0, 1.0] {
        let deadline = start + 120.0 - remaining;
        while h.app.world().resource::<Time<Fixed>>().elapsed_secs_f64() + 1.0 < deadline {
            h.advance(1);
        }
        assert_eq!(
            battleground(h, actor).phase,
            BgPhase::Waiting,
            "source countdown stage remains Waiting"
        );
        assert_eq!(
            (
                battleground(h, actor).score_a,
                battleground(h, actor).score_b
            ),
            (0, 0)
        );
    }
    while h.app.world().resource::<Time<Fixed>>().elapsed_secs_f64() < start + 120.1 {
        h.advance(1);
    }
    assert_eq!(
        battleground(h, actor).phase,
        BgPhase::InProgress,
        "source deadline starts match without extra roster-relative delay"
    );
    for entry in [322273, 322274] {
        let (door, _) = objects_in(h, entered.copy, entry)[0];
        assert_eq!(
            h.app.world().get::<GameObjectCollision>(door),
            Some(&GameObjectCollision(false)),
            "actual OnStart event opens gate"
        );
    }
    assert!(
        objects_in(h, entered.copy, 180322).is_empty(),
        "ghost blockers removed at source start"
    );
    h.advance(3);
    assert!(objects_in(h, entered.copy, 322273).is_empty());
    assert!(objects_in(h, entered.copy, 322274).is_empty());
    h.walk_to(actor, at);
    use_object(h, actor, banner);
    assert_ne!(
        battleground(h, actor).world_states,
        before,
        "normal post-start objective action works"
    );
}
