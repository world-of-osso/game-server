use super::*;
use shared::protocol::GameObjectAnimation;

// Concrete source entry sets and authored placement counts, not production registry reads.
const CASES: &[(u32, &[u32], usize)] = &[
    (2106, &[309704, 309705, 309883, 242273, 352709, 352710], 6),
    (2107, &[322273, 322274, 180322], 4),
    (
        726,
        &[
            208205, 208207, 208206, 203710, 206653, 206654, 206655, 180322,
        ],
        11,
    ),
    (761, &[207177, 207178, 205495, 205496], 4),
    (566, &[184719, 184720], 2),
    (30, &[180424, 180322], 4),
    (628, &[195703, 195491, 195436, 195437], 10),
    (998, &[215760, 215787], 4),
    (727, &[212939, 212940, 212941, 212942], 4),
    (2656, &[437274, 446147], 2),
];

#[test]
fn bg_preparation_authored_loader_includes_all_source_gates_and_preserves_missing_placement_gap() {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let loaded = crate::game_objects::load_game_object_spawns(&db).unwrap();
    for &(map, entries, expected) in CASES {
        let gates: Vec<_> = loaded
            .0
            .iter()
            .filter(|spawn| spawn.map_id == map && entries.contains(&spawn.info.entry))
            .collect();
        assert_eq!(gates.len(), expected, "map{map} actual authored placements");
        assert!(
            gates.iter().all(|gate| gate.initial_state == 1),
            "map{map} authored closed state"
        );
    }
    assert!(
        !loaded
            .0
            .iter()
            .any(|spawn| spawn.map_id == 1803 && [281224, 281226].contains(&spawn.info.entry)),
        "no invented Seething gates"
    );
}

#[test]
fn bg_preparation_all_sourced_map_gates_follow_actual_clock_callbacks_and_keep_other_copy_closed() {
    for &(map, entries, expected) in CASES {
        assert_map_gate_lifecycle(map, entries, expected);
    }
}

fn starts_for_map(map: u32) -> [u32; 2] {
    if let Some(rules) = node_rules(map) {
        return rules.starts;
    }
    if let Some(rules) = shared::battleground::flag_rules(map) {
        return rules.starts;
    }
    match map {
        566 => shared::battleground::eye_rules(map).unwrap().starts,
        30 => [611, 610],
        628 => [1299, 1245],
        998 => shared::battleground::KOTMOGU_STARTS,
        727 => shared::battleground::SILVERSHARD_STARTS,
        2656 => shared::battleground::DEEPHAUL_STARTS,
        _ => panic!("no sourced fixture for map{map}"),
    }
}

fn advance_until(h: &mut Harness, deadline: f64) {
    while h.app.world().resource::<Time<Fixed>>().elapsed_secs_f64() < deadline {
        h.advance(1);
    }
}

fn assert_map_gate_lifecycle(map: u32, entries: &[u32], expected: usize) {
    let template = shared::battleground::all_templates()
        .into_iter()
        .find(|template| template.map_id == map)
        .unwrap();
    let gates = source_gate_spawns(map, entries);
    assert_eq!(gates.len(), expected);
    let other_spawn = gates[0].clone();
    let arrival = std::rc::Rc::new(std::cell::Cell::new(0.0));
    let capture = arrival.clone();
    let mut entered = enter_source_match_preparing(
        &template,
        starts_for_map(map),
        |app| {
            app.world_mut()
                .resource_mut::<GameObjectSpawns>()
                .0
                .extend(gates);
        },
        SourceQueueFixture::SoloMinimum,
        move |h, _, index| {
            if index == 0 {
                capture.set(h.app.world().resource::<Time<Fixed>>().elapsed_secs_f64());
            }
        },
    );
    let h = &mut entered.h;
    let other_copy = InMap {
        map_id: map,
        instance_id: entered.copy.instance_id + 1000,
    };
    let other = h
        .app
        .world_mut()
        .spawn((
            crate::game_objects::game_object_bundle(&other_spawn),
            other_copy,
        ))
        .id();
    for &entry in entries {
        for (gate, _) in objects_in(h, entered.copy, entry) {
            assert_eq!(
                h.app.world().get::<GameObjectCollision>(gate),
                Some(&GameObjectCollision(true))
            );
            assert_eq!(
                h.app.world().get::<GameObjectAnimation>(gate),
                Some(&GameObjectAnimation {
                    animation_id: 147,
                    looping: true
                })
            );
        }
    }
    use_second_frames(h);
    let start = arrival.get();
    advance_until(h, start + 113.0);
    for &entry in entries {
        for (gate, _) in objects_in(h, entered.copy, entry) {
            assert_eq!(
                h.app.world().get::<GameObjectCollision>(gate),
                Some(&GameObjectCollision(true)),
                "map{map} entry{entry} before early deadline"
            );
        }
    }
    advance_until(h, start + 116.0);
    assert_eq!(battleground(h, &entered.players[0]).phase, BgPhase::Waiting);
    for &entry in entries {
        for (gate, _) in objects_in(h, entered.copy, entry) {
            let closed = !(map == 998 && entry == 215760);
            assert_eq!(
                h.app.world().get::<GameObjectCollision>(gate),
                Some(&GameObjectCollision(closed)),
                "map{map} entry{entry} source early-open stage"
            );
        }
    }
    advance_until(h, start + 121.1);
    assert_eq!(
        battleground(h, &entered.players[0]).phase,
        BgPhase::InProgress
    );
    for &entry in entries {
        let objects = objects_in(h, entered.copy, entry);
        if [30, 2107].contains(&map) && entry == 180322 {
            assert!(
                objects.is_empty(),
                "map{map} ghost blockers removed immediately"
            );
            continue;
        }
        assert!(
            !objects.is_empty(),
            "map{map} entry{entry} opens before delayed removal"
        );
        for (gate, _) in objects {
            assert_eq!(
                h.app.world().get::<GameObjectCollision>(gate),
                Some(&GameObjectCollision(false))
            );
            assert_eq!(
                h.app.world().get::<GameObjectAnimation>(gate),
                Some(&GameObjectAnimation {
                    animation_id: 148,
                    looping: false
                })
            );
        }
    }
    if map == 628 {
        h.advance(5);
        assert!(
            !objects_in(h, entered.copy, 195703).is_empty(),
            "Isle doors retain their source20second lifetime"
        );
    }
    h.advance(21);
    for &entry in entries {
        let retained = map == 998 || (map == 628 && [195436, 195437].contains(&entry));
        assert_eq!(
            !objects_in(h, entered.copy, entry).is_empty(),
            retained,
            "map{map} entry{entry} source removal/retention"
        );
    }
    assert_eq!(
        h.app.world().get::<GameObjectCollision>(other),
        Some(&GameObjectCollision(true)),
        "other copy remains closed/alive"
    );
}
