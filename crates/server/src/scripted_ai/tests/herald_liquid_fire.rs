//! Ground-only Liquid Fire native inputs. No flying-family coverage claim.
use super::*;

const CHAINS: &[(u32, u32, i32, u32)] = &[
    (31706, 181890, 15_000, 30927),
    (30928, 182533, 30_000, 32492),
    (23971, 180125, 20_000, 23972),
];

#[test]
fn encounter_coverage_herald_liquid_fire_source_qualification() {
    let (_directory, path) = prepare_encounter_database("herald");
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let store = &crate::spell_cast::test_harness::world_db().unwrap().store;
    for &(child, object, duration, pulse) in CHAINS {
        let spell = store.get(child).expect("native hit-unit child");
        assert_eq!(spell.effects.len(), 1);
        let effect = &spell.effects[0];
        assert_eq!((effect.index, effect.effect), (0, 76));
        assert_eq!(effect.misc_values[0], object as i32);
        assert_eq!(effect.implicit_targets, [18, 0]);
        assert_eq!(
            spell.misc.as_ref().unwrap().duration.unwrap().duration_ms,
            duration
        );
        let source: (u32, u32, u32, String, String) = conn.query_row(
            "SELECT type,displayId,Data3,AIName,ScriptName FROM tdb_gameobject_template WHERE entry=?1",
            [object], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)),
        ).unwrap();
        assert_eq!(
            source,
            (
                6,
                4251,
                pulse,
                String::new(),
                "go_vazruden_liquid_fire".into()
            )
        );
        let damage = store.get(pulse).expect("native ordinary trigger damage");
        assert_eq!(damage.effects.len(), 1);
        assert_eq!(damage.effects[0].effect, 2);
        assert_eq!(damage.effects[0].implicit_targets, [22, 15]);
        println!(
            "native child{} object{} duration{} damage{}: {}",
            child,
            object,
            duration,
            pulse,
            serde_json::to_string(damage).unwrap()
        );
    }
    let data = crate::world_data::load_world_data(std::path::Path::new(conn.path().unwrap()));
    for difficulty in [1, 2] {
        let template = data
            .template_for(21252, difficulty)
            .expect("source trigger difficulty");
        assert_eq!(template.entry, 21252);
        println!(
            "native21252 difficulty{difficulty}: name={} faction={} type={}",
            template.name, template.faction, template.creature_type
        );
    }
}

fn install_liquid_fire_inputs(h: &mut Harness) {
    let (_directory, path) = prepare_encounter_database("herald");
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>();
    let source = crate::world_data::load_world_data(&path);
    let mut data = h.app.world_mut().resource_mut::<WorldData>();
    data.templates
        .insert(21252, source.templates[&21252].clone());
    data.difficulty_templates.extend(
        source
            .difficulty_templates
            .into_iter()
            .filter(|((entry, _), _)| *entry == 21252),
    );
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    h.app
        .insert_resource(crate::game_objects::summoned::load_templates(&conn).unwrap());
}

fn pulses(h: &mut Harness, object: Entity) -> Vec<Entity> {
    h.creatures_of(21252)
        .into_iter()
        .filter(|entity| {
            h.app
                .world()
                .get::<crate::zones::TempSummon>(*entity)
                .is_some_and(|summon| summon.summoner == object)
        })
        .collect()
}

fn assert_liquid_fire(difficulty: u32, cone: bool) {
    use crate::game_objects::summoned::SummonedObject;
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::GameObjectInfo;
    let (mut h, map, nazan, _, _, _) = super::herald_ground::reach_fall(difficulty);
    install_liquid_fire_inputs(&mut h);
    let (parent, child, entry, duration, damage) = match (difficulty, cone) {
        (1, false) => (34653, 31706, 181890, 15_000, 30927),
        (2, false) => (36920, 30928, 182533, 30_000, 32492),
        (_, true) => (
            if difficulty == 2 { 36921 } else { 30926 },
            23971,
            180125,
            20_000,
            23972,
        ),
        _ => unreachable!(),
    };
    let player = h.player();
    let mut hit = false;
    // Actual source requests, ordinary admission, native hit resolution. Never force a hit.
    for _ in 0..2240 {
        h.advance(0.05);
        hit = h
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(nazan.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(parent)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            });
        if hit {
            break;
        }
    }
    assert!(
        hit,
        "ordinary source parent{parent} must really hit the native player"
    );
    let child_casts = &h
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
        .0;
    println!("native parent{parent} actual successful triggered launches: {child_casts:?}");
    assert!(
        child_casts.contains(&(player, child, Some(player))),
        "actual parent{parent} hit unit must self-trigger native{child}"
    );
    let objects: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &GameObjectInfo, &SummonedObject, &InMap)>()
        .iter(h.app.world())
        .filter(|(_, info, summon, copy)| {
            info.entry == entry && summon.spell_id == child && **copy == map
        })
        .map(|(entity, _, summon, _)| (entity, summon.owner, summon.expires_at))
        .collect();
    assert_eq!(
        objects.len(),
        1,
        "actual hit produces one real source wild GO"
    );
    let (object, owner, expiry) = objects[0];
    assert_eq!(owner, None, "source wild GO has no owner");
    let created = h.app.world().resource::<Time<Real>>().elapsed();
    assert_eq!(
        ((expiry.unwrap() - created.as_secs_f64()) * 1000.0).round() as u64,
        duration
    );
    let location = *h.app.world().get::<Position>(object).unwrap();
    // Pose-only harness setup places a real recipient inside the actual field.
    h.app.world_mut().entity_mut(player).insert(location);
    let mut born = std::collections::HashMap::new();
    let mut died = std::collections::HashSet::new();
    let mut first = None;
    let until = created + Duration::from_millis(duration + 2050);
    loop {
        let now = h.app.world().resource::<Time<Real>>().elapsed();
        for trigger in pulses(&mut h, object) {
            if born.contains_key(&trigger) {
                continue;
            }
            let first = *first.get_or_insert(now);
            assert!(
                first <= created + Duration::from_millis(50),
                "source zero-delay first GO update"
            );
            // TaskScheduler.cpp319–329 repeats from the previous scheduled
            // deadline0, not callback time50: first gap1950ms, then2000ms.
            let expected = if born.is_empty() {
                first
            } else {
                first - Duration::from_millis(50) + Duration::from_millis(2000 * born.len() as u64)
            };
            assert_eq!(now, expected, "exact source task deadline");
            assert_eq!(h.app.world().get::<InMap>(trigger), Some(&map));
            assert_eq!(h.app.world().get::<Position>(trigger), Some(&location));
            born.insert(trigger, now);
        }
        for (&trigger, &at) in &born {
            if h.app.world().get_entity(trigger).is_err() && died.insert(trigger) {
                assert_eq!(
                    now - at,
                    Duration::from_millis(1000),
                    "actual21252 timed despawn1s"
                );
            }
        }
        assert_eq!(
            h.app.world().get_entity(object).is_ok(),
            now < created + Duration::from_millis(duration),
            "strict source GO lifetime"
        );
        if now >= until {
            break;
        }
        h.advance(0.05);
    }
    let scheduler_origin = first.expect("source zero-delay pulse") - Duration::from_millis(50);
    let expiry = created + Duration::from_millis(duration);
    let repeats = (1..)
        .take_while(|index| scheduler_origin + Duration::from_millis(2000 * index) < expiry)
        .count();
    assert_eq!(
        born.len(),
        1 + repeats,
        "no later pulses after source expiry"
    );
    assert_eq!(died.len(), born.len());
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    for trigger in born.keys() {
        assert_eq!(
            events
                .iter()
                .filter(|event| event.source == Some(trigger.to_bits())
                    && event.target == Some(trigger.to_bits())
                    && event.spell_id == Some(damage)
                    && event.kind == CombatLogKind::CastSuccess)
                .count(),
            1,
            "each real source21252 ordinarily self-casts trap.spell once"
        );
    }
    assert!(
        events.iter().any(|event| event
            .source
            .is_some_and(|source| born.keys().any(|trigger| trigger.to_bits() == source))
            && event.target == Some(player.to_bits())
            && event.spell_id == Some(damage)
            && event.kind == CombatLogKind::Damage
            && event.amount > 0),
        "actual native field damage"
    );
    let other_player = h.players[1].0;
    assert!(
        !events
            .iter()
            .any(|event| event.target == Some(other_player.to_bits())
                && event.spell_id == Some(damage)),
        "unrelated copy cannot receive field damage"
    );
}

#[test]
fn encounter_coverage_herald_liquid_fire_normal_fireball() {
    assert_liquid_fire(1, false);
}
#[test]
fn encounter_coverage_herald_liquid_fire_heroic_fireball() {
    assert_liquid_fire(2, false);
}
#[test]
fn encounter_coverage_herald_liquid_fire_normal_cone() {
    assert_liquid_fire(1, true);
}
#[test]
fn encounter_coverage_herald_liquid_fire_heroic_cone() {
    assert_liquid_fire(2, true);
}

#[test]
fn encounter_coverage_herald_liquid_fire_unrelated_script_refused() {
    let (mut h, _, _, _, _, _) = super::herald_ground::reach_fall(1);
    install_liquid_fire_inputs(&mut h);
    let effect = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(46369)
        .unwrap()
        .effects[0]
        .clone();
    let reason =
        crate::game_objects::summoned::wild_unsupported_reason(h.app.world(), &effect).unwrap();
    assert!(
        reason.contains("requires custom script"),
        "unrelated Ahune GO must remain refused: {reason}"
    );
}
