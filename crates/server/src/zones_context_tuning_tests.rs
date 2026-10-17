//! Source17536 fixtures through ordinary copy spawning, never canonical placements.
use super::*;
use rusqlite::{Connection, OpenFlags};

const NAZAN: u32 = 17_536;
const RAMPARTS: u32 = 543;

fn sourced_nazan_fixture() -> WorldData {
    let source = crate::test_data::world_db_path();
    let connection = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    // Temporary views project actual TDB base/difficulty fields; the source stays read-only.
    // CT0 intentionally has no stored range: the ordinary spawn must resolve its context.
    connection
        .execute_batch(
            "CREATE TEMP VIEW content_creature_template AS
         SELECT t.entry,COALESCE(l.minlevel,1) minlevel,COALESCE(l.maxlevel,1) maxlevel,
           t.faction,t.npcflag,t.speed_walk,d.HealthModifier,t.name,t.Classification rank,
           t.ExperienceModifier,t.type,t.flags_extra,t.speed_run,t.ScriptName,t.unit_class,
           d.HealthScalingExpansion exp,t.unit_flags,d.ContentTuningID,d.HealthScalingExpansion,
           d.LevelScalingDeltaMin,d.LevelScalingDeltaMax,d.DamageModifier,d.ArmorModifier,
           t.subname,d.ManaModifier,t.unit_flags2,t.RegenHealth,t.family,d.TypeFlags type_flags,
           NULL detection_range
         FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry
         LEFT JOIN content_tuning_level l ON l.ID=d.ContentTuningID
         WHERE t.entry=17536 AND d.DifficultyID=0;
         CREATE TEMP VIEW content_creature_template_difficulty AS
         SELECT d.*,COALESCE(l.minlevel,1) minlevel,COALESCE(l.maxlevel,1) maxlevel,
           d.HealthScalingExpansion exp
         FROM tdb_creature_template_difficulty d
         LEFT JOIN content_tuning_level l ON l.ID=d.ContentTuningID WHERE d.Entry=17536;
         CREATE TEMP VIEW content_creature_template_model AS
         SELECT * FROM tdb_creature_template_model WHERE CreatureID=17536;",
        )
        .unwrap();
    let expected_stats = crate::world_data::load_expected_stats(&connection);
    let mut templates = crate::world_data::load_templates(&connection, &expected_stats);
    crate::world_data::load_template_models(&connection, &mut templates);
    let difficulty_templates =
        crate::world_data::load_difficulty_templates(&connection, &expected_stats, &templates);
    assert!(templates[&NAZAN].tuning.is_none(), "source base CT0");
    assert!(
        difficulty_templates[&(NAZAN, 2)].tuning.is_none(),
        "source Heroic CT0"
    );
    assert_eq!(templates[&NAZAN].health_modifier, 9.0);
    assert_eq!(difficulty_templates[&(NAZAN, 2)].health_modifier, 12.0);
    // Pinned Herald CenterPos, used only as a fixture spawn; no population is imported.
    let spawn = CreatureSpawn {
        guid: 1,
        template_entry: NAZAN,
        map: RAMPARTS as u16,
        zone_id: 0,
        position_x: -1410.5204,
        position_y: 1740.5852,
        position_z: 89.05998,
        orientation: 0.0,
        wander_distance: 0.0,
        movement_type: 0,
        spawntimesecs: 120,
        display_id: None,
        unit_flags: None,
        spawn_difficulties: vec![1, 2],
        equipment_id: 0,
    };
    WorldData {
        summon_levels: crate::world_data::summon_levels::SummonLevels::load(&connection).unwrap(),
        templates,
        difficulty_templates,
        expected_stats,
        class_level_base_hp: crate::test_data::world_data().class_level_base_hp.clone(),
        spawns: vec![spawn],
        spawns_by_map: HashMap::from([(RAMPARTS as u16, vec![0])]),
        ..Default::default()
    }
}

#[test]
#[ignore = "RED: ordinary CT0 spawn has no map context tuning"]
fn herald_context_ordinary_nazan_spawn_uses_authored_normal_and_heroic_map_tuning() {
    use crate::level_scaling::{LevelStats, TuningInputs};
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let data = sourced_nazan_fixture();
    for (difficulty, tuning_id, health_modifier, damage_modifier) in
        [(1, 220, 9.0, 1.0), (2, 1065, 12.0, 13.0)]
    {
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, app.world());
        let content = MapContent {
            map: crate::maps::InMap {
                map_id: RAMPARTS,
                instance_id: difficulty,
            },
            difficulty_id: difficulty,
            dead_bosses: vec![],
        };
        assert_eq!(
            spawn_map_creatures(&mut commands, &data, content.clone(), false),
            1
        );
        queue.apply(app.world_mut());
        let world = app.world_mut();
        let mut actors = world.query::<(Entity, &Npc, &crate::maps::InMap)>();
        let actor = actors
            .iter(world)
            .find(|(_, npc, map)| npc.template_id == NAZAN && **map == content.map)
            .unwrap()
            .0;
        let scaling = world
            .get::<LevelScaling>(actor)
            .expect("ordinary sourced CT0 Nazan must resolve actual map difficulty tuning");
        assert_eq!(scaling.content_tuning_id, tuning_id);
        assert_eq!(scaling.delta, 0, "Nazan retains its authored zero delta");
        let level = scaling.native_level();
        assert!(level > 1, "map range, not unresolved template minimum");
        assert_eq!(world.get::<UnitLevel>(actor).unwrap().0, level);
        let stats = LevelStats::build(
            &data.expected_stats,
            &TuningInputs {
                content_tuning_id: tuning_id,
                health_scaling_expansion: 1,
                unit_class: 1,
                health_modifier,
                damage_modifier,
                armor_modifier: 1.0,
            },
        );
        assert_eq!(
            world.get::<Health>(actor).unwrap().max,
            stats.max_health(level)
        );
        assert_eq!(
            world.get::<CreatureArmor>(actor).unwrap().0,
            stats.armor(level)
        );
        let scaled = world
            .get::<ScaledStats>(actor)
            .expect("regional stats are retained");
        assert_eq!(scaled.0.max_health(10), stats.max_health(10));
        assert_eq!(
            scaled.0.swing_damage_range(level, 2.0),
            stats.swing_damage_range(level, 2.0)
        );
    }
}
