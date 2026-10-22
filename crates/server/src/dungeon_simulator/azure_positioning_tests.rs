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
    for tick in 0..MAX_TICKS {
        run.control().unwrap();
        run.app.update();
        if tick % 1000 == 0 {
            // Match campaign progress capture: this probe enters native admission.
            let _ = crate::followers::simulator::inspect_tank_movement(
                run.app.world_mut(),
                run.party.tank,
            );
        }
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

#[test]
#[ignore = "explicit native Azure Rune chase across the sourced first book hop"]
fn native_rune_unreachable_chase_after_book_hop_evades_without_crossing_components() {
    use crate::creature_ai::{CannotReachTarget, CreatureState};
    let _rng_scope = crate::random::seed_scope(1);
    let mut run = spawn_run(1, &mut None).unwrap();
    assert_eq!(run.graph.map_id, 2515);
    let owner = run.party.owner;
    let rune = run.actors[&29001884];
    let book = run.actors[&29001868];
    let origin = *run.app.world().get::<Position>(rune).unwrap();
    let source = *run.app.world().get::<Position>(book).unwrap();
    assert!(
        !run.app
            .world()
            .get::<shared::components::UnitFlags>(rune)
            .unwrap()
            .is_selectable()
    );
    run.app.world_mut().entity_mut(owner).insert(source);
    assert_eq!(
        crate::npc_interaction::azure_translocation::traverse(run.app.world_mut(), owner, book),
        Ok(5)
    );
    movement::forget_party_travel(run.app.world_mut(), owner);
    let target = *run.app.world().get::<Position>(owner).unwrap();
    assert!(
        run.app
            .world()
            .resource::<Space>()
            .route(
                Vec3::new(origin.x, origin.y, origin.z),
                Vec3::new(target.x, target.y, target.z),
            )
            .is_err(),
        "the real book hop crosses disconnected walk components"
    );
    crate::creature_aggro::attack_start(run.app.world_mut(), rune, owner);
    assert_eq!(
        run.app.world().get::<CreatureState>(rune),
        Some(&CreatureState::Pursuing {
            target: owner.to_bits()
        })
    );
    println!("RUNE_BOOK_BOUNDARY rune={rune:?} origin={origin:?} target={target:?}");
    let mut blocked_seen = false;
    for _ in 0..200 {
        run.app.update();
        let world = run.app.world();
        blocked_seen |= world.get::<CannotReachTarget>(rune).is_some();
        if blocked_seen && world.get::<CreatureState>(rune) == Some(&CreatureState::Idle) {
            assert!(world.get::<CannotReachTarget>(rune).is_none());
            assert_eq!(world.get::<Position>(rune), Some(&origin));
            assert!(
                world
                    .get::<Health>(rune)
                    .is_some_and(|health| health.current > 0.0)
            );
            return;
        }
    }
    panic!(
        "native blocked chase must run its existing evade clock and home arrival, not panic or cross the book gap"
    );
}
