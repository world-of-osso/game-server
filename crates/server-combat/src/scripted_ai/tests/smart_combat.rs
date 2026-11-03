//! SmartAI combat on the Geomancer's actual template, spawn, spells and script rows.
use super::*;

const GEOMANCER: u32 = 476;
const FIREBALL: u32 = 20793;

fn geomancer() -> (Harness, Entity) {
    geomancer_with_inferred(None)
}

fn geomancer_with_inferred(inferred: Option<Vec<smart_ai::SmartEvent>>) -> (Harness, Entity) {
    let data = data().expect("world.db fixture");
    let mut app = stockade_app(data);
    let path = world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let expected = crate::world_data::load_expected_stats(&conn);
    let mut templates = crate::world_data::load_templates(&conn, &expected);
    templates.retain(|entry, _| *entry == GEOMANCER);
    crate::world_data::load_template_models(&conn, &mut templates);
    app.insert_resource(WorldData {
        templates: templates.into(),
        class_level_base_mana: data.base_mana.clone(),
        ..Default::default()
    });
    let all = crate::spell_cast::test_harness::world_db().unwrap();
    app.insert_resource(SpellStore::fixture(
        [FIREBALL, 12544, 1246821, 1246825]
            .iter()
            .map(|id| all.store.get(*id).unwrap().clone())
            .collect(),
    ));
    let mut scripts = smart_ai::load_smart_scripts(&path).unwrap();
    if let Some(events) = inferred {
        scripts.0.remove(&GEOMANCER);
        scripts
            .4
            .insert((0, 0, GEOMANCER), std::sync::Arc::new(events));
    }
    app.insert_resource(scripts);
    let spawn = conn.query_row(
        "SELECT guid, position_x, position_y, position_z, orientation, spawntimesecs FROM content_creature WHERE id1 = 476 AND map = 0 ORDER BY guid LIMIT 1",
        [],
        |row| Ok(CreatureSpawn {
            guid: row.get(0)?, template_entry: GEOMANCER, map: 0, zone_id: 12,
            position_x: row.get(1)?, position_y: row.get(2)?, position_z: row.get(3)?,
            orientation: row.get(4)?, wander_distance: 0.0, movement_type: 0,
            spawntimesecs: row.get(5)?, display_id: None, unit_flags: None,
            spawn_difficulties: vec![0], equipment_id: 0,
        }),
    ).unwrap();
    let map = InMap::continent(0);
    let wd = app.world().resource::<WorldData>();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    let creature = crate::zones::spawn_creature(
        &mut commands,
        wd,
        &wd.templates[&GEOMANCER],
        &mut crate::zones::CreatureSpawnContext {
            map: map,
            difficulty_id: 0,
            ground: Some(&mut crate::test_data::ground_map()),
            temporary: false,
        },
        false,
        &spawn,
        &mut rand::thread_rng(),
    )
    .expect("source geomancer level context");
    queue.apply(app.world_mut());
    // Outside proximity aggro until explicitly engaged; inside Fireball's range.
    let (player, connection) = spawn_player(
        &mut app,
        map,
        [
            spawn.position_x + 25.0,
            spawn.position_y,
            spawn.position_z,
            std::f32::consts::PI,
        ],
        1,
    );
    let mut h = Harness {
        app,
        players: vec![(player, connection, Inbox::default())],
    };
    h.advance(0.05);
    (h, creature)
}

#[test]
fn creature_self_cast_logs_once_to_visible_player() {
    let (mut h, creature) = geomancer();
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(creature).insert(visibility);
    crate::spell_cast::creature_cast(h.app.world_mut(), creature, 1246821, Some(creature), true)
        .unwrap();
    h.advance(0.05);
    assert_eq!(h.casts(0, creature, 1246821), 1);
}

#[test]
fn inferred_ability_casts_using_observed_timers() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE content_creature_template(entry INTEGER, AIName TEXT, ScriptName TEXT);
        INSERT INTO content_creature_template VALUES (476, '', '');
        CREATE TABLE content_inferred_creature_ability(map INTEGER, difficulty INTEGER, entry INTEGER, spell INTEGER, target_type INTEGER);
        CREATE TABLE content_inferred_creature_ability_sample(map INTEGER, difficulty INTEGER, entry INTEGER, spell INTEGER, sample_id INTEGER, timing_kind TEXT, delay_ms INTEGER);
        INSERT INTO content_inferred_creature_ability VALUES (0, 0, 476, 20793, 2);
        INSERT INTO content_inferred_creature_ability_sample VALUES
            (0, 0, 476, 20793, 1, 'first_activity_proxy', 250),
            (0, 0, 476, 20793, 2, 'successive_cast', 7000);").unwrap();
    let mut samples = smart_ai::inferred::load(&conn).unwrap();
    let (mut h, creature) = geomancer_with_inferred(samples.remove(&(0, 0, GEOMANCER)));
    h.engage(creature);
    h.advance(0.15);
    assert!(h.app.world().get::<CastState>(creature).is_none());
    h.advance(0.2);
    let first = h
        .app
        .world()
        .get::<CastState>(creature)
        .expect("first inferred cast")
        .clone();
    assert_eq!(first.spell_id, FIREBALL);
    assert_eq!(first.target, h.player().to_bits());
    h.advance(first.duration - first.elapsed + 0.1);
    assert_eq!(h.casts(0, creature, FIREBALL), 1);
    h.advance(6.0 - first.duration);
    assert!(h.app.world().get::<CastState>(creature).is_none());
    let second = wait_for_fireball(&mut h, creature);
    assert_eq!(second.target, h.player().to_bits());
}

#[test]
fn inferred_summon_casts_once_when_created() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE content_creature_template(entry INTEGER, AIName TEXT, ScriptName TEXT);
        INSERT INTO content_creature_template VALUES (476, '', '');
        CREATE TABLE content_inferred_summon_spell(map INTEGER, difficulty INTEGER, entry INTEGER, spell INTEGER);
        INSERT INTO content_inferred_summon_spell VALUES (0, 0, 476, 1246821);").unwrap();
    let mut events = smart_ai::inferred::load_summon_events(&conn).unwrap();
    let (mut h, original) = geomancer_with_inferred(events.remove(&(0, 0, GEOMANCER)));
    assert_eq!(
        crate::auras::aura_count(h.app.world(), original, 1246821),
        0
    );
    let owner = h.player();
    let pos = *h.app.world().get::<Position>(owner).unwrap();
    let summoned = crate::zones::summon_creature(
        h.app.world_mut(),
        owner,
        GEOMANCER,
        [pos.x + 5.0, pos.y, pos.z, 0.0],
    )
    .expect("summon geomancer fixture");
    h.advance(0.2);
    assert_eq!(
        crate::auras::aura_count(h.app.world(), summoned, 1246821),
        1
    );
    h.advance(2.0);
    assert_eq!(
        crate::auras::aura_count(h.app.world(), summoned, 1246821),
        1
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        summoned,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1246821,
    );
    h.advance(2.0);
    assert_eq!(
        crate::auras::aura_count(h.app.world(), summoned, 1246821),
        0
    );
}

fn wait_for_fireball(h: &mut Harness, creature: Entity) -> CastState {
    for _ in 0..102 {
        h.advance(0.05);
        if let Some(cast) = h.app.world().get::<CastState>(creature) {
            if cast.spell_id == FIREBALL {
                return cast.clone();
            }
        }
    }
    panic!("engaged Geomancer must start Fireball within 5.1 seconds");
}

// Keep the exhaustion condition stable despite the real template's mana regen.
fn advance_without_mana(h: &mut Harness, creature: Entity, seconds: f32) {
    for _ in 0..(seconds / 0.05).round() as usize {
        h.app.world_mut().get_mut::<Mana>(creature).unwrap().current = 0.0;
        h.advance(0.05);
    }
}

#[test]
fn geomancer_casts_at_victim_and_holds_position_through_repeat_window() {
    let (mut h, creature) = geomancer();
    h.engage(creature);
    let cast = wait_for_fireball(&mut h, creature);
    let first_start = h.app.world().resource::<Time>().elapsed_secs_f64();
    assert_eq!(cast.target, h.player().to_bits());
    let position = *h.app.world().get::<Position>(creature).unwrap();
    h.advance(0.2);
    assert_eq!(*h.app.world().get::<Position>(creature).unwrap(), position);
    assert_eq!(
        *h.app.world().get::<CreatureMotion>(creature).unwrap(),
        CreatureMotion::Still
    );
    h.advance(cast.duration - cast.elapsed - 0.2 + 0.1);
    assert_eq!(h.casts(0, creature, FIREBALL), 1);
    assert_eq!(
        *h.app.world().get::<Position>(creature).unwrap(),
        position,
        "CMC holds even between casts"
    );
    let next = wait_for_fireball(&mut h, creature);
    let repeat_seconds = h.app.world().resource::<Time>().elapsed_secs_f64() - first_start;
    assert!(
        (3.0..=5.1).contains(&repeat_seconds),
        "repeat after {repeat_seconds}s"
    );
    assert_eq!(next.target, h.player().to_bits());
    assert_eq!(*h.app.world().get::<Position>(creature).unwrap(), position);
}

#[test]
fn geomancer_out_of_mana_resumes_chase_and_melee() {
    let (mut h, creature) = geomancer();
    h.engage(creature);
    let cast = wait_for_fireball(&mut h, creature);
    h.advance(cast.duration - cast.elapsed + 0.1);
    let position = *h.app.world().get::<Position>(creature).unwrap();
    advance_without_mana(&mut h, creature, 5.2);
    assert!(h.app.world().get::<CastState>(creature).is_none());
    assert_ne!(*h.app.world().get::<Position>(creature).unwrap(), position);
    advance_without_mana(&mut h, creature, 6.0);
    assert_eq!(
        h.casts(0, creature, FIREBALL),
        1,
        "no casts while exhausted"
    );
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.source == Some(creature.to_bits())
                && event.target == Some(h.player().to_bits())
                && event.spell_id.is_none()
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_)))
    );
    // Failed casts retry promptly rather than consuming a 3-5 s repeat cooldown.
    let max = h.app.world().get::<Mana>(creature).unwrap().max;
    h.app.world_mut().get_mut::<Mana>(creature).unwrap().current = max;
    h.advance(0.1);
    assert_eq!(
        h.app.world().get::<CastState>(creature).unwrap().spell_id,
        FIREBALL
    );
}

#[test]
fn geomancer_failed_cast_resumes_chase_and_melee() {
    let (mut h, creature) = geomancer();
    h.engage(creature);
    let cast = wait_for_fireball(&mut h, creature);
    h.advance(cast.duration - cast.elapsed + 0.1);
    let position = *h.app.world().get::<Position>(creature).unwrap();
    let now = h.app.world().resource::<Time>().elapsed_secs_f64();
    let mut history = crate::spell_cast::SpellHistory::default();
    history.lock_schools(4, now + 30.0);
    h.app.world_mut().entity_mut(creature).insert(history);
    h.advance(5.2);
    assert!(h.app.world().get::<CastState>(creature).is_none());
    assert_ne!(*h.app.world().get::<Position>(creature).unwrap(), position);
    h.advance(6.0);
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.source == Some(creature.to_bits())
                && event.target == Some(h.player().to_bits())
                && event.spell_id.is_none()
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_)))
    );
}

#[test]
fn geomancer_combat_timer_does_not_cast_out_of_combat() {
    let (mut h, creature) = geomancer();
    h.advance(6.0);
    assert_eq!(h.casts(0, creature, FIREBALL), 0);
    h.engage(creature);
    assert_eq!(
        wait_for_fireball(&mut h, creature).target,
        h.player().to_bits()
    );
}
