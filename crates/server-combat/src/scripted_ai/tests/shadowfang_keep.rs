//! Source Silverlaine spell/lifecycle boundary; not a complete fight or native clear.
use super::*;

const SILVERLAINE: u32 = 3887;
const SHADOWFANG_MAP: u32 = 33;
const NORMAL_DIFFICULTY: u32 = 1;
const SILVERLAINE_ENCOUNTER: u32 = 1070;
const FIXTURE_OWNER: u64 = 2001;
const FRESH_OWNER: u64 = 2002;
const OTHER_PRIMARIES: [u32; 4] = [46962, 4278, 46963, 46964];
const VEIL_OF_SHADOW: u32 = 7068;
const SILVERLAINE_BIT: u32 = 1;
const HEALING_POTION: u32 = 8141;
const HEAL_INPUT: f32 = 1000.0;

/// A controlled noncritical heal at the shared healing-modifier boundary.
fn apply_potion_heal(h: &mut Harness, target: Entity) -> f32 {
    let before = h.health(target).current;
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(HEALING_POTION)
        .unwrap()
        .clone();
    let context = EffectCtx {
        caster: target,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::apply_heal(
        h.app.world_mut(),
        &context,
        target,
        HEAL_INPUT,
        false,
        false,
    );
    h.health(target).current - before
}

#[test]
fn encounter_coverage_silverlaine_real_source_cast_repeat_and_death_credit() {
    let (mut h, boss, map) = encounter_fixture(
        SILVERLAINE,
        SHADOWFANG_MAP,
        SILVERLAINE_BIT,
        NORMAL_DIFFICULTY,
        None,
        EncounterState::NotStarted,
        false,
    );
    let boss = boss.unwrap();
    let player = h.player();
    // Isolate the source clock from player survival; actor, spell, timer and chance stay real.
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 900_000.0,
        max: 1_000_000.0,
    });
    assert_eq!(apply_potion_heal(&mut h, player), HEAL_INPUT);
    h.engage(boss);
    h.advance(0.8);
    assert_eq!(h.casts(0, boss, VEIL_OF_SHADOW), 0);
    assert!(!h.has_aura(player, VEIL_OF_SHADOW));
    h.advance(3.0);
    assert_eq!(h.casts(0, boss, VEIL_OF_SHADOW), 1);
    assert!(h.has_aura(player, VEIL_OF_SHADOW));
    // Actual7068 effect0 MOD_HEALING_PCT118/-50, TC Unit::SpellHealingBonusTaken.
    assert_eq!(apply_potion_heal(&mut h, player), HEAL_INPUT * 0.5);
    assert!(!h.has_aura(h.players[1].0, VEIL_OF_SHADOW));
    h.advance(36.0);
    assert_eq!(h.casts(0, boss, VEIL_OF_SHADOW), 2);
    assert_eq!(h.casts(1, boss, VEIL_OF_SHADOW), 0);
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, SILVERLAINE_BIT),
        Some(EncounterState::Done)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == SILVERLAINE_ENCOUNTER && end.success)
    );
    assert!(h.inbox(1).ends.is_empty());
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1 << SILVERLAINE_BIT
    );
}

#[test]
fn encounter_coverage_silverlaine_normal_progress_reopens_and_filters_only_its_primary() {
    use crate::maps::{InstanceManager, locks::InstanceLocks, persistence};
    let (mut h, _, map) = encounter_fixture(
        SILVERLAINE,
        SHADOWFANG_MAP,
        SILVERLAINE_BIT,
        NORMAL_DIFFICULTY,
        None,
        EncounterState::NotStarted,
        true,
    );
    let bosses = h.creatures_of(SILVERLAINE);
    assert_eq!(bosses.len(), 1, "one authored Silverlaine spawn");
    h.engage(bosses[0]);
    h.hit_pct(bosses[0], 200.0);
    h.advance(0.1);
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("shadowfang.redb");
    {
        let database = redb::Database::create(&file).unwrap();
        let instances = h.app.world().resource::<InstanceManager>();
        let normal: Vec<_> = instances.normal_locks.values().cloned().collect();
        persistence::save_instance_locks(&database, &instances.locks.saved(), &normal);
    }
    let database = redb::Database::open(&file).unwrap();
    let saved = persistence::load_instance_locks(&database);
    let normal = persistence::load_normal_instance_locks(&database);
    assert!(
        saved.locks.is_empty(),
        "Normal progress is not a scheduled raid/Heroic lock"
    );
    assert!(
        saved.shared.is_empty(),
        "Normal progress has no scheduled shared lock state"
    );
    assert_eq!(normal.len(), 1);
    let mut instances =
        InstanceManager::with_locks_and_normal(InstanceLocks::from_saved(saved), normal);
    let entrant = crate::maps::instances::InstanceEntrant {
        character_id: FIXTURE_OWNER,
        leader: FIXTURE_OWNER,
        difficulty_id: NORMAL_DIFFICULTY,
        ..Default::default()
    };
    let now = crate::maps::instances::Now {
        elapsed: 0.0,
        unix: 1000,
    };
    let maps = h.app.world().resource::<crate::maps::MapStore>();
    let restored = instances
        .create_map(maps, SHADOWFANG_MAP, entrant, now)
        .unwrap();
    assert_eq!(restored.map, map);
    assert_eq!(instances.completed_encounters(map), 1 << SILVERLAINE_BIT);
    let done = instances.normal_locks[&map.instance_id].data.done_bosses;
    let dead_bosses = instance_script::done_boss_entries(maps, SHADOWFANG_MAP, done);
    assert_eq!(dead_bosses, vec![SILVERLAINE]);
    let new_owner = crate::maps::instances::InstanceEntrant {
        character_id: FRESH_OWNER,
        leader: FRESH_OWNER,
        ..entrant
    };
    let fresh = instances
        .create_map(maps, SHADOWFANG_MAP, new_owner, now)
        .unwrap();
    assert_ne!(fresh.map, map);
    assert_eq!(instances.completed_encounters(fresh.map), 0);
    let existing: std::collections::HashSet<_> = h
        .app
        .world_mut()
        .query_filtered::<Entity, With<shared::components::Npc>>()
        .iter(h.app.world())
        .collect();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    crate::zones::spawn_map_creatures(
        &mut commands,
        h.app.world().resource::<WorldData>(),
        crate::zones::MapContent {
            map,
            difficulty_id: NORMAL_DIFFICULTY,
            dead_bosses,
        },
        false,
        Some(&mut crate::test_data::ground_map()),
    );
    queue.apply(h.app.world_mut());
    let new_entries: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &shared::components::Npc, &InMap)>()
        .iter(h.app.world())
        .filter(|(entity, _, actor_map)| !existing.contains(entity) && **actor_map == map)
        .map(|(_, npc, _)| npc.template_id)
        .collect();
    assert!(!new_entries.contains(&SILVERLAINE));
    // Existing authored primaries remain populated; no credit binding for them is invented.
    for entry in OTHER_PRIMARIES {
        assert!(
            new_entries.contains(&entry),
            "uncompleted source actor {entry} retained"
        );
    }
}
