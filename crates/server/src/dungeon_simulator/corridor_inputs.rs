//! Explicit native radius qualification; route computation consumes only its frozen data.
use super::super::{diagnostics, runtime};
use super::*;

#[test]
#[ignore = "explicit native corridor-input capture; select map/level in environment"]
fn freeze_native_corridor_inputs() {
    let _rng_scope = crate::random::seed_scope(0);
    let graph = read_graph().unwrap();
    let mut app = runtime::build_app();
    let copy = runtime::create_copy(&mut app);
    let party = crate::followers::simulator::spawn_party(
        app.world_mut(),
        copy,
        shared::components::Position {
            x: graph.entrance[0],
            y: graph.entrance_height,
            z: graph.entrance[1],
        },
    )
    .unwrap();
    let actors = runtime::spawn_population(&mut app, copy);
    let members: Vec<_> = party.members.iter().copied().chain([party.owner]).collect();
    let mut sources = Vec::new();
    for (guid, actor) in actors {
        let world = app.world();
        let position = *world.get::<shared::components::Position>(actor).unwrap();
        let aggressive = world
            .get::<crate::creature_aggro::AggroRange>(actor)
            .is_some()
            && world
                .get::<crate::scripted_ai::ReactState>(actor)
                .is_none_or(|react| *react == crate::scripted_ai::ReactState::Aggressive);
        let eligible = aggressive
            && members.iter().any(|member| {
                crate::spell_cast::targets::relation_matches(
                    world,
                    *member,
                    actor,
                    crate::spell_cast::targets::Relation::Enemy,
                )
            });
        let radius = if eligible {
            let mut maximum = 0.0_f64;
            for member in &members {
                let geometry =
                    diagnostics::inspect_aggro_pair(app.world_mut(), actor, *member).unwrap();
                let radius = geometry["npc_acquisition_radius"]
                    .as_f64()
                    .unwrap_or_else(|| {
                        panic!("native corridor radius unavailable for source{guid}")
                    });
                assert!(radius.is_finite() && radius > 0.0);
                maximum = maximum.max(radius);
            }
            Some(maximum)
        } else {
            None
        };
        sources.push(
            serde_json::json!({"guid":guid,"eligible":eligible,"radius":radius,
            "position":[position.x,position.y,position.z]}),
        );
    }
    let levels: Vec<_> = members
        .iter()
        .map(|member| {
            app.world()
                .get::<shared::components::UnitLevel>(*member)
                .unwrap()
                .0
        })
        .collect();
    let value = serde_json::json!({"map_id":graph.map_id,"boss_order":graph.boss_entries(),
        "party_levels":levels,"qualification_seed":0,"radius_source":"native AggroDistance::center_radius; maximum over default party",
        "sources":sources});
    let path = graph_path().with_file_name(format!(
        "{}-corridor-inputs.json",
        graph_path().file_stem().unwrap().to_string_lossy()
    ));
    std::fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    println!(
        "NATIVE_CORRIDOR_INPUTS map={} sources={} {}",
        graph.map_id,
        value["sources"].as_array().unwrap().len(),
        path.display()
    );
}
