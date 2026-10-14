//! Generic movement-validation regressions, not invented encounter path data.
use super::*;
use crate::scripted_ai::{CreatureAi, CreatureAiState, Ctx};
use std::time::Duration;

#[test]
fn waypoint_source_short_initial_segment_still_moves_to_the_real_destination() {
    // TC MoveSpline.cpp::_checkPathLengths starts at segment1, not segment0.
    let spline = launch(
        vec![
            Vec3::ZERO,
            Vec3::new(0.05, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
        ],
        false,
        0.0,
        5.0,
    )
    .expect("source admits a short first crossing");
    assert!((spline.position(501).x - 2.5).abs() < 0.001);
    assert_eq!(spline.position(spline.end_ms), Vec3::new(5.0, 0.0, 0.0));
}

#[derive(Resource, Default)]
struct ArrivalReceipts(Vec<(u32, u32)>);

struct ArrivalConsumer;
impl CreatureAi for ArrivalConsumer {
    fn waypoint_path_ended(&mut self, ctx: &mut Ctx, node: u32, path: u32) {
        ctx.world
            .resource_mut::<ArrivalReceipts>()
            .0
            .push((node, path));
    }
}

#[test]
fn waypoint_rejected_interior_segment_never_moves_or_reports_a_path_arrival() {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    world.init_resource::<ArrivalReceipts>();
    world.insert_resource(ScriptPaths(HashMap::from([(
        1,
        Arc::new(ScriptPath {
            id: 1,
            move_type: MoveType::Run,
            flags: FLAG_EXACT_SPLINE,
            velocity: None,
            has_node_timing: false,
            nodes: vec![
                (0, Vec3::new(2.0, 0.0, 0.0)),
                (1, Vec3::new(2.05, 0.0, 0.0)),
                (2, Vec3::new(5.0, 0.0, 0.0)),
            ],
        }),
    )])));
    let actor = world
        .spawn((
            bevy(Vec3::ZERO),
            crate::maps::InMap::continent(0),
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            CreatureMotion::Still,
            CreatureAiState {
                ai: Some(Box::new(ArrivalConsumer)),
                pending: Default::default(),
            },
        ))
        .id();
    assert!(move_path(&mut world, actor, 1));
    for _ in 0..200 {
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(50));
        path_movement_system(&mut world);
    }
    assert_eq!(*world.get::<Position>(actor).unwrap(), bevy(Vec3::ZERO));
    assert!(
        world.resource::<ArrivalReceipts>().0.is_empty(),
        "no synthetic script arrival after rejected controls"
    );
}
