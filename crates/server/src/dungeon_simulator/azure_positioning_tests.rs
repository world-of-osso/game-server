//! Full native ordering for Azure seed1's isolated owner at the Bubbling Sapling.
use super::super::{Space, diagnostics, movement};
use super::{MAX_TICKS, spawn_run};
use bevy::prelude::Vec3;
use shared::components::{Health, Position};

#[test]
#[ignore = "explicit Azure70 seed1 native Sapling engagement and isolated-owner Death"]
fn seed_one_keeps_the_native_leader_alive_through_the_sapling_fight() {
    let _rng_scope = crate::random::seed_scope(1);
    let mut run = spawn_run(1, &mut None).unwrap();
    assert_eq!(run.graph.map_id, 2515);
    let owner = run.party.owner;
    let sapling = run.actors[&29001840];
    let mut captured = false;
    for _ in 0..MAX_TICKS {
        run.control().unwrap();
        run.app.update();
        let (current_health, max_health) = {
            let health = run.app.world().get::<Health>(owner).unwrap();
            (health.current, health.max)
        };
        if !captured && run.current_pack == Some(29001840) && current_health < max_health * 0.5 {
            captured = true;
            let watcher = run.actors[&29001853];
            let healer = run.party.members[1];
            let geometry =
                diagnostics::inspect_aggro_pair(run.app.world_mut(), watcher, healer).unwrap();
            let party: Vec<_> = run
                .members()
                .map(|unit| {
                    let world = run.app.world();
                    let from = world.get::<Position>(unit).unwrap();
                    let to = world.get::<Position>(sapling).unwrap();
                    let path = world
                        .resource::<Space>()
                        .route(
                            Vec3::new(from.x, from.y, from.z),
                            Vec3::new(to.x, to.y, to.z),
                        )
                        .map(|points| {
                            points
                                .into_iter()
                                .map(|point| point.to_array())
                                .collect::<Vec<_>>()
                        });
                    serde_json::json!({
                        "unit": diagnostics::inspect_unit(world, None, unit),
                        "travel": movement::inspect_travel(world, unit),
                        "fresh_path_to_sapling": path,
                    })
                })
                .collect();
            println!(
                "AZURE_SAPLING_SEPARATION {}",
                serde_json::json!({
                    "at": run.now(), "party": party,
                    "sapling": diagnostics::inspect_unit(run.app.world(), Some(29001840), sapling),
                    "watcher": diagnostics::inspect_unit(run.app.world(), Some(29001853), watcher),
                    "watcher_healer_geometry": geometry,
                })
            );
        }
        assert!(
            current_health > 0.0,
            "native owner died at{}s before Sapling29001840's Death; captured={captured}",
            run.now()
        );
        if run
            .app
            .world()
            .get::<Health>(sapling)
            .is_some_and(|health| health.current == 0.0)
        {
            return;
        }
    }
    panic!("native Sapling engagement must finish within the unchanged campaign bound");
}
