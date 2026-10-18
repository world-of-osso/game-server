//! TDB1210 Feero Ironhand4484 / Supplies to Auberdine976, real death-hook boundary.
use super::*;

#[path = "smart_feero_combat_tests.rs"]
mod combat;
#[path = "smart_feero_live_setup.rs"]
mod live_setup;
#[path = "smart_feero_quest_tests.rs"]
mod quests;
use crate::scripted_ai::{CreatureAiState, Hook, MotionType, dispatch};
use shared::components::{Health, UnitFactionTemplate, UnitFlags};

fn feero() -> (tempfile::TempDir, World, Entity, Entity) {
    let (temp, mut world, mut ai, creature, player) = erland();
    let source = crate::test_data::world_db_path();
    let scripts = load_smart_scripts(&source).unwrap();
    ai.events = scripts.0[&4484].clone();
    ai.states = vec![EventState::default(); ai.events.len()];
    let conn = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    world.insert_resource(crate::faction::load_faction_templates(&conn).unwrap());
    world.insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&conn).unwrap());
    let expected = crate::world_data::load_expected_stats(&conn);
    let mut templates = crate::world_data::load_templates(&conn, &expected);
    templates.retain(|entry, _| [4484, 3879, 3893, 3898, 3899, 3900].contains(entry));
    crate::world_data::load_template_models(&conn, &mut templates);
    let display_id = templates[&4484].models[0].display_id;
    let mut query = conn
        .prepare("SELECT level,class,basehp0,basehp1,basehp2 FROM creature_classlevelstats")
        .unwrap();
    let class_level_base_hp = query
        .query_map([], |r| {
            Ok(((r.get(0)?, r.get(1)?), [r.get(2)?, r.get(3)?, r.get(4)?]))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    world.insert_resource(crate::world_data::WorldData {
        templates,
        class_level_base_hp,
        ..Default::default()
    });
    world.insert_resource(scripts);
    world.insert_resource(Time::<()>::default());
    let map = crate::maps::InMap::continent(1);
    world.entity_mut(creature).insert((
        Npc {
            template_id: 4484,
            name: "Feero Ironhand".into(),
        },
        Health {
            current: 100.0,
            max: 100.0,
        },
        UnitFactionTemplate(35),
        shared::components::ModelDisplay { display_id },
        UnitFlags(0x200),
        map,
    ));
    world.entity_mut(player).insert((
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        map,
    ));
    ai.reset(&mut Ctx {
        world: &mut world,
        me: creature,
    });
    world.entity_mut(creature).insert(CreatureAiState {
        ai: Some(Box::new(ai)),
        pending: Default::default(),
    });
    dispatch(
        &mut world,
        creature,
        Hook::QuestAccepted {
            player,
            quest_id: 976,
        },
    );
    dispatch(&mut world, creature, Hook::UpdateAi(0));
    dispatch(&mut world, creature, Hook::UpdateAi(1000));
    dispatch(&mut world, creature, Hook::UpdateAi(3000));
    (temp, world, creature, player)
}

fn reach(world: &mut World, creature: Entity, player: Entity, node: u32) {
    let movement = world
        .get::<crate::scripted_ai::movement::PointMovement>(creature)
        .unwrap_or_else(|| panic!("escort must own movement before authored node {node}"));
    assert_eq!(movement.id, node);
    let position = movement.target;
    world
        .entity_mut(creature)
        .insert(position)
        .remove::<crate::scripted_ai::movement::PointMovement>();
    world.entity_mut(player).insert(position);
    dispatch(
        world,
        creature,
        Hook::MovementInform(MotionType::Point, node),
    );
}

#[test]
fn smartai_feero_four_real_ambusher_deaths_release_first_camp() {
    let (_temp, mut world, creature, player) = feero();
    for node in 1..=19 {
        reach(&mut world, creature, player, node);
    }
    let mut ambushers: Vec<_> = world
        .query::<(Entity, &Npc)>()
        .iter(&world)
        .filter(|(_, npc)| npc.template_id == 3879)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(ambushers.len(), 4, "authored summon group0");
    for &ambusher in &ambushers {
        let ai = creature_ai(&world, ambusher).unwrap();
        world.entity_mut(ambusher).insert(CreatureAiState {
            ai: Some(ai),
            pending: Default::default(),
        });
    }
    let last = ambushers.pop().unwrap();
    for ambusher in ambushers {
        world.get_mut::<Health>(ambusher).unwrap().current = 0.0;
        dispatch(&mut world, ambusher, Hook::JustDied(Some(player)));
    }
    dispatch(&mut world, creature, Hook::UpdateAi(12000));
    assert!(
        world
            .get::<crate::scripted_ai::movement::PointMovement>(creature)
            .is_none(),
        "three kills must not resume the four-assassin camp"
    );
    world.get_mut::<Health>(last).unwrap().current = 0.0;
    dispatch(&mut world, last, Hook::JustDied(Some(player)));
    dispatch(&mut world, creature, Hook::UpdateAi(0));
    dispatch(&mut world, creature, Hook::UpdateAi(1000));
    dispatch(&mut world, creature, Hook::UpdateAi(11000));
    dispatch(&mut world, creature, Hook::UpdateAi(50));
    assert_eq!(
        world
            .get::<crate::scripted_ai::movement::PointMovement>(creature)
            .expect("fourth death must execute counter77 list448402 and resume")
            .id,
        20
    );
}
