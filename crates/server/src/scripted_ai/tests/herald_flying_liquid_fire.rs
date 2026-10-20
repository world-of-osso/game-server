//! Natural flying-parent qualification precedes any flying Liquid Fire hook.
use super::herald_call::first_west;
use super::*;
use crate::scripted_ai::tests::cast_admission::CastAdmissions;
use crate::spell_cast::test_harness::ObservedCombatEvents;

fn stage_flying_recipient(h: &mut Harness, map: InMap, nazan: Entity) {
    let hover = *h.app.world().get::<Position>(nazan).unwrap();
    let ground = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, Vec3::new(hover.x + 3.0, hover.y, hover.z));
    let crate::ground::GroundProbe::Supported(floor) = ground else {
        panic!("native floor required for ordinary flying-phase recipient: {ground:?}")
    };
    let player = h.player();
    // Harness pose only: a native grounded recipient near actual first West.
    h.app.world_mut().entity_mut(player).insert(Position {
        x: hover.x + 3.0,
        y: floor,
        z: hover.z,
    });
}

fn qualify_flying_parents(difficulty: u32) {
    let (mut h, map, nazan, _) = first_west(difficulty);
    h.app.init_resource::<ObservedCombatEvents>();
    let parents = if difficulty == 2 {
        [32491, 33794]
    } else {
        [30691, 33793]
    };
    for parent in parents {
        let spell = h.app.world().resource::<SpellStore>().get(parent).unwrap();
        assert_eq!(spell.effects.len(), 2);
        assert_eq!((spell.effects[0].index, spell.effects[0].effect), (0, 2));
        assert_eq!((spell.effects[1].index, spell.effects[1].effect), (1, 77));
        assert!(
            spell
                .effects
                .iter()
                .all(|effect| effect.implicit_targets == [6, 0])
        );
        let misc = spell.misc.as_ref().unwrap();
        assert_eq!(misc.cast_time.unwrap().base_ms, 3000);
        assert_eq!(misc.speed, 24.0);
        assert_eq!(misc.launch_delay, 0.0);
        println!(
            "native flying parent: {}",
            serde_json::to_string(spell).unwrap()
        );
    }
    stage_flying_recipient(&mut h, map, nazan);
    let player = h.player();
    let origin = h.app.world().resource::<Time<Fixed>>().elapsed_secs_f64();
    h.app.insert_resource(CastAdmissions {
        actor: nazan,
        origin,
        entries: Vec::new(),
    });
    let mut hits = [false; 2];
    for _ in 0..2240 {
        h.advance(0.05);
        let events = &h.app.world().resource::<ObservedCombatEvents>().0;
        for (index, parent) in parents.into_iter().enumerate() {
            hits[index] = events.iter().any(|event| {
                event.source == Some(nazan.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(parent)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            });
        }
        if hits.into_iter().all(|hit| hit) {
            break;
        }
    }
    let admissions = &h.app.world().resource::<CastAdmissions>().entries;
    println!("natural flying difficulty{difficulty} admissions: {admissions:?}");
    println!("natural flying difficulty{difficulty} positive-hit qualification: {hits:?}");
    for (index, parent) in parents.into_iter().enumerate() {
        assert!(
            admissions
                .iter()
                .any(|entry| entry.spell == parent && entry.result.is_ok()),
            "ordinary naturally scheduled parent{parent} admission must qualify within112s"
        );
        assert!(
            hits[index],
            "ordinary native parent{parent} must really impact within112s; no forced hit"
        );
    }
    assert!(
        h.app
            .world()
            .get::<crate::scripted_ai::waypoint_path::GravityDisabled>(nazan)
            .is_some(),
        "qualification must remain in source flying phase"
    );
}

fn objects_in_copy(h: &mut Harness, map: InMap, entry: u32) -> std::collections::HashSet<Entity> {
    h.app
        .world_mut()
        .query::<(Entity, &shared::protocol::GameObjectInfo, &InMap)>()
        .iter(h.app.world())
        .filter(|(_, info, copy)| info.entry == entry && **copy == map)
        .map(|(entity, _, _)| entity)
        .collect()
}

fn assert_flying_liquid_fire(difficulty: u32, random: bool) {
    use crate::game_objects::summoned::SummonedObject;
    use crate::spell_cast::test_harness::ObservedTriggeredCasts;
    let (mut h, map, nazan, _) = first_west(difficulty);
    stage_flying_recipient(&mut h, map, nazan);
    h.app.init_resource::<ObservedCombatEvents>();
    super::herald_liquid_fire::install_liquid_fire_inputs(&mut h);
    let parent = match (difficulty, random) {
        (1, false) => 30691,
        (1, true) => 33793,
        (2, false) => 32491,
        (2, true) => 33794,
        _ => unreachable!(),
    };
    let (child, entry, duration, damage) = if difficulty == 2 {
        (30928, 182533, 30_000, 32492)
    } else {
        (31706, 181890, 15_000, 30927)
    };
    let player = h.player();
    let mut launched_at = None;
    let mut field = None;
    for _ in 0..2240 {
        let before = objects_in_copy(&mut h, map, entry);
        let events_before = h.app.world().resource::<ObservedCombatEvents>().0.len();
        let casts_before = h.app.world().resource::<ObservedTriggeredCasts>().0.len();
        h.advance(0.05);
        let now = h.app.world().resource::<Time<Fixed>>().elapsed();
        if launched_at.is_none()
            && h.inbox(0)
                .spell_gos
                .iter()
                .any(|go| go.caster == nazan.to_bits() && go.spell_id == parent)
        {
            launched_at = Some(now);
        }
        let hit = h.app.world().resource::<ObservedCombatEvents>().0[events_before..]
            .iter()
            .any(|event| {
                event.source == Some(nazan.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(parent)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            });
        if !hit {
            continue;
        }
        assert!(
            launched_at.expect("ordinary parent launch receipt") < now,
            "native missile must travel before its actual impact/child"
        );
        let casts = &h.app.world().resource::<ObservedTriggeredCasts>().0[casts_before..];
        println!("flying parent{parent} actual-impact-frame child launches: {casts:?}");
        assert!(
            casts.contains(&(player, child, Some(player))),
            "actual flying parent{parent} impact must self-trigger native{child} on that frame; old victim fields cannot qualify random parent"
        );
        let after = objects_in_copy(&mut h, map, entry);
        let created: Vec<_> = after.difference(&before).copied().collect();
        assert_eq!(
            created.len(),
            1,
            "one fresh source GO at this parent impact, not an older victim field"
        );
        let object = created[0];
        let summon = h.app.world().get::<SummonedObject>(object).unwrap();
        assert_eq!((summon.owner, summon.spell_id), (None, child));
        assert_eq!(
            h.app.world().get::<Position>(object),
            h.app.world().get::<Position>(player),
            "child starts at actual hit-unit pose"
        );
        field = Some(object);
        break;
    }
    let object = field.expect("qualified natural flying parent must really impact within112s");
    super::herald_liquid_fire::assert_field_lifetime(&mut h, map, object, duration, damage);
    assert!(
        h.app
            .world()
            .get::<crate::scripted_ai::waypoint_path::GravityDisabled>(nazan)
            .is_some()
    );
}

#[test]
fn encounter_coverage_herald_flying_liquid_fire_normal_victim() {
    assert_flying_liquid_fire(1, false);
}
#[test]
fn encounter_coverage_herald_flying_liquid_fire_normal_random() {
    assert_flying_liquid_fire(1, true);
}
#[test]
fn encounter_coverage_herald_flying_liquid_fire_heroic_victim() {
    assert_flying_liquid_fire(2, false);
}
#[test]
fn encounter_coverage_herald_flying_liquid_fire_heroic_random() {
    assert_flying_liquid_fire(2, true);
}

#[test]
fn encounter_coverage_herald_flying_liquid_fire_normal_admission_qualification() {
    qualify_flying_parents(1);
}

#[test]
fn encounter_coverage_herald_flying_liquid_fire_heroic_admission_qualification() {
    qualify_flying_parents(2);
}
