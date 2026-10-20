//! TC331–346/355–420: real fall+2s release and ordinary ground parent spells.
//! NAZAN_LIQUID_FIRE_EFFECT77_CHILDREN_UNIMPLEMENTED: no child/object claim.
use super::herald_call::{damage_to_percent, first_west};
use super::herald_intro::{actors_in_copy, spawn_second_copy};
use super::herald_intro2::received_casts;
use super::*;
use crate::scripted_ai::tests::cast_admission::{AiTickTrace, CastAdmissions};
use crate::spell_cast::test_harness::ObservedCombatEvents;

const DESCEND: Position = Position {
    x: -1429.1720,
    y: 84.34760,
    z: -1749.1290,
};
const GROUND_SPELLS: [u32; 5] = [34653, 36920, 30926, 36921, 39427];

pub(super) fn reach_fall(difficulty: u32) -> (Harness, InMap, Entity, Entity, Entity, Duration) {
    let (mut h, map, nazan, vazruden) = first_west(difficulty);
    h.app.init_resource::<ObservedCombatEvents>();
    let floor = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, Vec3::new(DESCEND.x, DESCEND.y, DESCEND.z));
    let crate::ground::GroundProbe::Supported(floor) = floor else {
        panic!("native floor required")
    };
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: DESCEND.x + 3.0,
        y: floor,
        z: DESCEND.z,
    });
    let other_map = spawn_second_copy(&mut h);
    let other = actors_in_copy(&mut h, 17307, other_map)[0];
    damage_to_percent(&mut h, nazan, 29.0);
    for _ in 0..100 {
        h.advance(0.05);
        if received_casts(&h, nazan, 20538) == 1 {
            let landed = h.app.world().resource::<Time<Fixed>>().elapsed();
            assert!(
                h.app
                    .world()
                    .get::<crate::scripted_ai::waypoint_path::GravityDisabled>(nazan)
                    .is_none()
            );
            let origin = landed.as_secs_f64();
            h.app.insert_resource(CastAdmissions {
                actor: nazan,
                origin,
                entries: Vec::new(),
            });
            return (h, map, nazan, vazruden, other, landed);
        }
    }
    panic!("source descent and actual EFFECT5 must finish naturally");
}

fn qualify_parents(h: &Harness) {
    let store = h.app.world().resource::<SpellStore>();
    for id in GROUND_SPELLS {
        let spell = store.get(id).expect("native ground parent spell");
        assert!(spell.power_costs.is_empty());
        if id == 39427 {
            assert_eq!((spell.effects[0].effect, spell.effects[0].aura), (6, 7));
            assert_eq!(spell.effects[0].implicit_targets, [22, 15]);
            assert_eq!(
                spell.misc.as_ref().unwrap().cast_time.unwrap().base_ms,
                1500
            );
            assert_eq!(
                spell.misc.as_ref().unwrap().duration.unwrap().duration_ms,
                4000
            );
        } else {
            assert_eq!(spell.effects[0].effect, 2);
            assert_eq!((spell.effects[1].index, spell.effects[1].effect), (1, 77));
            assert_eq!(spell.misc.as_ref().unwrap().cast_time.unwrap().base_ms, 0);
        }
        println!(
            "native ground parent {}",
            serde_json::to_string(spell).unwrap()
        );
    }
}

#[test]
fn encounter_coverage_herald_ground_source_qualification() {
    let (h, _, _, _, _, _) = reach_fall(1);
    qualify_parents(&h);
}

fn assert_release(difficulty: u32) {
    let (mut h, map, nazan, vazruden, other, landed) = reach_fall(difficulty);
    let logic_before = received_casts(&h, nazan, 30694);
    let other_health = h.health(other).current;
    let lethal = h.health(nazan).max * 2.0;
    crate::combat::deal_direct_damage(h.app.world_mut(), nazan, nazan, lethal);
    assert_eq!(h.health(nazan).current, 1.0);
    for _ in 0..39 {
        h.advance(0.05);
        assert_eq!(
            h.app.world().get::<ReactState>(nazan),
            Some(&ReactState::Passive)
        );
        assert_eq!(received_casts(&h, nazan, 30694), logic_before);
        assert!(
            h.app
                .world()
                .get::<crate::combat::DeathState>(nazan)
                .is_none()
        );
    }
    assert_eq!(
        h.app.world().resource::<Time<Fixed>>().elapsed() - landed,
        Duration::from_millis(1950)
    );
    crate::combat::deal_direct_damage(h.app.world_mut(), nazan, nazan, lethal);
    assert_eq!(
        h.health(nazan).current,
        1.0,
        "actual lethal loss remains protected through1950ms"
    );
    h.advance(0.05);
    assert_eq!(
        h.app.world().resource::<Time<Fixed>>().elapsed() - landed,
        Duration::from_millis(2000)
    );
    assert_eq!(
        h.app.world().get::<ReactState>(nazan),
        Some(&ReactState::Aggressive),
        "DESCEND4 exactly realfall+2000ms"
    );
    assert_eq!(
        received_casts(&h, nazan, 30694),
        logic_before + 1,
        "ordinary source logic timer at release"
    );
    crate::combat::deal_direct_damage(h.app.world_mut(), nazan, nazan, lethal);
    assert_eq!(
        h.health(nazan).current,
        0.0,
        "actual lethal commit must no longer clamp after release"
    );
    h.advance(0.05);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(nazan)
            .is_some()
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(vazruden)
            .is_none()
    );
    assert_eq!(h.health(other).current, other_health);
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0,
        "not paired-credit slice"
    );
    assert!(h.inbox(0).ends.iter().all(|end| !end.success));
}

fn assert_dispatch_windows(
    h: &Harness,
    spell: u32,
    first_max: f32,
    repeat_min: f32,
    repeat_max: f32,
) {
    let entries = &h.app.world().resource::<CastAdmissions>().entries;
    // Permanent evidence precedes every assertion, including a failing boundary.
    println!(
        "native ground asserted{spell}/Roar admissions: {:?}",
        entries
            .iter()
            .filter(|entry| entry.spell == spell || entry.spell == 39427)
            .collect::<Vec<_>>()
    );
    let attempts: Vec<_> = entries
        .iter()
        .filter(|entry| entry.spell == spell)
        .collect();
    assert!(
        attempts.len() >= 2,
        "source{spell} must actually dispatch first and repeat; observed{}",
        attempts.len()
    );
    // EventMap advances this same release tick by50ms, as in source UpdateAI.
    let first = attempts[0].time - 2.0;
    assert!(
        first >= -0.05 && first <= first_max,
        "native first{spell} at{first}"
    );
    for pair in attempts.windows(2) {
        let elapsed = pair[1].time - pair[0].time;
        // Only a real accepted Roar spanning the latest possible deadline can
        // justify an attempt later than the source upper bound.
        let latest_due = pair[0].time + repeat_max;
        let latest_dispatch = entries
            .iter()
            .filter(|entry| entry.spell == 39427 && entry.result.is_ok())
            .filter(|entry| entry.time <= latest_due && entry.time + 1.5 > latest_due)
            .map(|entry| entry.time + 1.5)
            .fold(latest_due, f32::max);
        assert!(
            elapsed >= repeat_min - 0.001 && pair[1].time <= latest_dispatch + 0.001,
            "source repeat{spell}: {elapsed}, latest permitted dispatch{latest_dispatch}"
        );
    }
}

fn assert_ground_parents(difficulty: u32) {
    let (mut h, map, nazan, vazruden, other, _) = reach_fall(difficulty);
    qualify_parents(&h);
    let fireball = if difficulty == 2 { 36920 } else { 34653 };
    let cone = if difficulty == 2 { 36921 } else { 30926 };
    let other_health = h.health(other).current;
    let other_player = h.players[1].0;
    let other_player_health = h.health(other_player).current;
    let mut saw_fear = false;
    if std::env::var_os("GAME_SERVER_HERALD_AI_TRACE").is_some() {
        let origin = h.app.world().resource::<CastAdmissions>().origin;
        h.app.insert_resource(AiTickTrace {
            actor: nazan,
            origin,
            frames: Vec::new(),
        });
    }
    // First source windows fail fast before a longer natural repeat observation.
    for tick in 1..=2240 {
        h.advance(0.05);
        if tick == 260 {
            let entries = &h.app.world().resource::<CastAdmissions>().entries;
            assert!(
                entries.iter().any(|entry| entry.spell == fireball),
                "missing native Fireball source dispatch"
            );
            assert!(
                entries.iter().any(|entry| entry.spell == cone),
                "missing native Cone source dispatch"
            );
        }
        let player = h.player();
        saw_fear |= h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(player)
            .is_some_and(|auras| auras.is_feared());
        assert!(
            h.app
                .world()
                .get::<crate::combat::DeathState>(nazan)
                .is_none()
        );
        assert!(
            h.app
                .world()
                .get::<crate::combat::DeathState>(vazruden)
                .is_none()
        );
        assert_eq!(h.health(other).current, other_health);
        assert_eq!(h.health(other_player).current, other_player_health);
        assert!(h.inbox(0).ends.iter().all(|end| !end.success));
    }
    if let Some(trace) = h.app.world().get_resource::<AiTickTrace>() {
        for frame in &trace.frames {
            println!(
                "native Herald AI tick {}",
                serde_json::to_string(frame).unwrap()
            );
        }
    }
    assert_dispatch_windows(&h, fireball, 5.0, 5.0, 15.0);
    assert_dispatch_windows(&h, cone, 10.0, 10.0, 15.0);
    let admissions = &h.app.world().resource::<CastAdmissions>().entries;
    let wrong_difficulty = if difficulty == 2 {
        [34653, 30926]
    } else {
        [36920, 36921]
    };
    assert!(
        admissions
            .iter()
            .all(|entry| !wrong_difficulty.contains(&entry.spell))
    );
    assert_eq!(
        received_casts(&h, nazan, 30694),
        2,
        "DESCEND4 cannot replay during real parent windows"
    );
    println!(
        "native ground admissions difficulty{difficulty}: {:?}",
        admissions
            .iter()
            .map(|entry| (entry.spell, entry.time, &entry.result))
            .collect::<Vec<_>>()
    );
    if difficulty == 2 {
        assert_dispatch_windows(&h, 39427, 2.0, 40.0, 50.0);
        let first_roar = admissions
            .iter()
            .find(|entry| entry.spell == 39427)
            .unwrap();
        assert!(
            (3.95..=4.0).contains(&first_roar.time),
            "source2s ground clock, including same-tick50ms advancement only when victim exists"
        );
        assert!(saw_fear, "actual native aura7 must fear a real player");
    } else {
        assert!(!saw_fear);
        assert!(
            !h.app
                .world()
                .resource::<CastAdmissions>()
                .entries
                .iter()
                .any(|entry| entry.spell == 39427)
        );
    }
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    for spell in [fireball, cone] {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(nazan.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "actual native parent{spell} damage"
        );
    }
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
}

/// Source Spell.cpp4265 expires1500ms at the30th actual50ms update, not31st.
#[test]
fn encounter_coverage_herald_ground_roar_cast_1500ms_ai_deadline() {
    let (mut h, _, nazan, _, _, _) = reach_fall(2);
    // StdRng0 yields authored initial Fireball3656ms/Cone7735ms rolls,
    // both after the first Roar2000ms request and its ordinary1500ms cast.
    h.app.world_mut().entity_mut(nazan).insert(
        crate::scripted_ai::tests::cast_admission::GroundCombatSeed(0),
    );
    let origin = h.app.world().resource::<CastAdmissions>().origin;
    h.app.insert_resource(AiTickTrace {
        actor: nazan,
        origin,
        frames: Vec::new(),
    });
    let mut started = None;
    // Fixture-only combat RNG starts at natural release; real source ranges
    // place competing first requests after Roar, without bypassing admission.
    for _ in 0..100 {
        h.advance(0.05);
        let accepted_roar = h
            .app
            .world()
            .resource::<CastAdmissions>()
            .entries
            .iter()
            .any(|entry| entry.spell == 39427 && entry.result.is_ok());
        if accepted_roar {
            started = Some(h.app.world().resource::<Time<Fixed>>().elapsed());
            break;
        }
    }
    println!(
        "native1500ms setup admissions: {:?}",
        h.app.world().resource::<CastAdmissions>().entries
    );
    let started = started.expect("seeded source rolls admit ordinary first Roar");
    let first = h
        .app
        .world()
        .resource::<AiTickTrace>()
        .frames
        .last()
        .unwrap();
    let cast = first
        .cast
        .as_ref()
        .expect("real cast exists at its AI dispatch");
    assert_eq!(
        (
            cast.spell_id,
            cast.duration,
            cast.elapsed,
            cast.pushback_count
        ),
        (39427, 1.5, 0.0, 0)
    );
    for _ in 0..29 {
        h.advance(0.05);
        let frame = h
            .app
            .world()
            .resource::<AiTickTrace>()
            .frames
            .last()
            .unwrap();
        assert!(frame.cast.is_some(), "no early release before native1500ms");
    }
    assert_eq!(
        h.app.world().resource::<Time<Fixed>>().elapsed() - started,
        Duration::from_millis(1450)
    );
    h.advance(0.05);
    assert_eq!(
        h.app.world().resource::<Time<Fixed>>().elapsed() - started,
        Duration::from_millis(1500)
    );
    let frame = h
        .app
        .world()
        .resource::<AiTickTrace>()
        .frames
        .last()
        .unwrap();
    assert!(
        frame.cast.is_none(),
        "AI must observe native1500ms expiry at30ticks, not31; actual {:?}",
        frame.cast
    );
}

#[test]
fn encounter_coverage_herald_ground_release_normal() {
    assert_release(1);
}
#[test]
fn encounter_coverage_herald_ground_release_heroic() {
    assert_release(2);
}
#[test]
fn encounter_coverage_herald_ground_parents_normal() {
    assert_ground_parents(1);
}
#[test]
fn encounter_coverage_herald_ground_parents_heroic() {
    assert_ground_parents(2);
}
