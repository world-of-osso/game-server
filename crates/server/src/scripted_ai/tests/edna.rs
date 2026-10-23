//! TC a352b1fa Stonevault: template-inherited StringId, real deaths and intro.
//! Normal source placements;205 explicitly projects identical authored actors.
use super::*;

const EDNA: u32 = 210108;
const GOLEM: u32 = 210109;
const MAP: u32 = 2652;
const INTRO_STATE: u32 = 6;
const IMMUNE_TO_PC: u32 = 256;
const ANCHOR: u32 = 45313;
const COMBAT_POSITION: Position = Position {
    x: 1.94097,
    y: 361.66537,
    z: -0.512153,
};

fn source_copy(difficulty: u32) -> (Harness, Entity, InMap, Vec<Entity>) {
    let (mut h, boss, map) = bound_boss_at_difficulty(EDNA, MAP, 0, difficulty, None);
    let connection =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut spawns: Vec<_> = crate::world_data::load_spawns(&connection)
        .into_iter()
        .filter(|spawn| spawn.template_entry == GOLEM && u32::from(spawn.map) == MAP)
        .collect();
    spawns.sort_by_key(|spawn| spawn.guid);
    assert_eq!(
        spawns.iter().map(|spawn| spawn.guid).collect::<Vec<_>>(),
        [30002119, 30002123, 30002124, 30002127, 30002177]
    );
    let golems = spawns
        .into_iter()
        .map(|mut spawn| {
            assert_eq!(spawn.spawn_difficulties, [1, 2, 8, 23]);
            spawn.spawn_difficulties = vec![difficulty];
            spawn_prepared_actor(&mut h.app, spawn, map, difficulty).0
        })
        .collect::<Vec<_>>();
    for &golem in &golems {
        let ids = h
            .app
            .world()
            .get::<crate::zones::CreatureStringIds>(golem)
            .unwrap();
        assert_eq!(ids.template.as_deref(), Some("edna_intro_trash"));
        assert!(ids.spawn.is_none());
        assert!(ids.has("edna_intro_trash"));
    }
    h.advance(0.05);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 320);
    (h, boss, map, golems)
}

fn intro_state(h: &mut Harness, map: InMap) -> u32 {
    instance_script::get_data(h.app.world_mut(), map, INTRO_STATE)
}

fn immune(h: &Harness, actor: Entity) -> bool {
    h.app.world().get::<UnitFlags>(actor).unwrap().0 & IMMUNE_TO_PC != 0
}

#[test]
#[ignore = "EDNA source intro RED must execute before implementation"]
fn encounter_coverage_edna_five_inherited_stringid_deaths_force_walk_arrival_unlock() {
    for difficulty in [1, 205] {
        let (mut h, boss, map, golems) = source_copy(difficulty);
        let origin = *h.app.world().get::<Position>(boss).unwrap();
        for &golem in &golems[..4] {
            super::herald_intro::kill_actor(&mut h, golem);
        }
        h.advance(4.0);
        assert_eq!(intro_state(&mut h, map), 0);
        assert_eq!(h.app.world().get::<Position>(boss), Some(&origin));
        assert!(immune(&h, boss));
        super::herald_intro::kill_actor(&mut h, golems[4]);
        assert_eq!(
            intro_state(&mut h, map),
            1,
            "fifth real death starts instance intro"
        );
        h.advance(2.8);
        assert_eq!(h.app.world().get::<Position>(boss), Some(&origin));
        assert!(immune(&h, boss));
        h.advance(0.3);
        assert!(h.app.world().get::<movement::Walking>(boss).is_some());
        assert!(h.app.world().get::<movement::PointMovement>(boss).is_some());
        assert!(
            !h.has_aura(boss, 447230),
            "source removes Skardyn spawn aura before walking"
        );
        assert_eq!(intro_state(&mut h, map), 1);
        for _ in 0..500 {
            h.advance(0.05);
            if h.app.world().get::<movement::PointMovement>(boss).is_none() {
                break;
            }
        }
        assert_eq!(h.app.world().get::<Position>(boss), Some(&COMBAT_POSITION));
        assert!(
            immune(&h, boss),
            "MovementInform schedules +1s, not immediate unlock"
        );
        assert_eq!(intro_state(&mut h, map), 1);
        h.advance(0.8);
        assert!(immune(&h, boss));
        h.advance(0.3);
        assert_eq!(
            intro_state(&mut h, map),
            3,
            "TC DONE from authored arrival callback"
        );
        assert!(!immune(&h, boss));
        assert!(
            h.has_aura(boss, ANCHOR),
            "native45313 anchor cast after arrival+1s"
        );
        assert_eq!(
            get_boss_state(h.app.world_mut(), map, 0),
            Some(EncounterState::NotStarted)
        );
    }
}

#[test]
#[ignore = "EDNA source intro RED must execute before implementation"]
fn encounter_coverage_edna_death_counter_is_copy_owned_and_idle_time_does_not_unlock() {
    let (mut h, boss, map, golems) = source_copy(1);
    let other = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    // Existing fixture peer belongs to this distinct copy. Moving one authentic
    // golem there lets the real lethal boundary prove copy ownership.
    h.app.world_mut().entity_mut(golems[0]).insert(other);
    super::herald_intro::kill_actor(&mut h, golems[0]);
    for &golem in &golems[1..] {
        super::herald_intro::kill_actor(&mut h, golem);
    }
    h.advance(25.0);
    assert_eq!(intro_state(&mut h, map), 0);
    assert_eq!(intro_state(&mut h, other), 0);
    assert!(immune(&h, boss));
    assert!(!h.has_aura(boss, ANCHOR));
    assert!(h.app.world().get::<movement::PointMovement>(boss).is_none());
}
