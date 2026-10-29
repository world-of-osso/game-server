use super::targets::{Relation, area_units};
use bevy::prelude::*;
use shared::components::{Health, Position};
use std::time::Instant;

#[test]
fn area_units_selects_only_living_nearby_units_in_the_same_copy() {
    let mut app = App::new();
    crate::maps::units::install(&mut app);
    let map = crate::maps::InMap {
        map_id: 2521,
        instance_id: 1,
    };
    let other = crate::maps::InMap {
        map_id: 2521,
        instance_id: 2,
    };
    let spawn = |app: &mut App, map, x, health| {
        app.world_mut()
            .spawn((
                map,
                Position { x, y: 0.0, z: 0.0 },
                Health {
                    current: health,
                    max: 100.0,
                },
            ))
            .id()
    };
    let caster = spawn(&mut app, map, 0.0, 100.0);
    let near = spawn(&mut app, map, 4.0, 100.0);
    spawn(&mut app, map, 5.0, 0.0);
    spawn(&mut app, other, 3.0, 100.0);
    for n in 0..30_000 {
        spawn(
            &mut app,
            crate::maps::InMap {
                map_id: 0,
                instance_id: 0,
            },
            1_000.0 + n as f32,
            100.0,
        );
    }
    let start = Instant::now();
    for _ in 0..100 {
        let selected = area_units(app.world_mut(), caster, Vec3::ZERO, 10.0, Relation::Any, 0);
        assert_eq!(selected, vec![caster, near]);
    }
    println!(
        "area_units mean: {:.2} us/call (30000 distant units)",
        start.elapsed().as_secs_f64() * 10_000.0
    );
    for n in 0..5_000 {
        spawn(&mut app, map, 1_000.0 + n as f32, 100.0);
    }
    assert_eq!(
        area_units(app.world_mut(), caster, Vec3::ZERO, 10.0, Relation::Any, 0),
        vec![caster, near]
    );
}
