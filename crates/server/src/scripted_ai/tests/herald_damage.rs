//! TC217–220: nonlethal own damage may descend only after intro and only once.
use super::herald_call::{damage_to_percent, descent_emotes, first_west};
use super::herald_intro::{actors_in_copy, kill_actor, spawn_second_copy};
use super::herald_intro2::{WEST, prepared_ramparts, received_casts};
use super::*;

const DESCEND: Position = Position {
    x: -1429.1720,
    y: 84.34760,
    z: -1749.1290,
};
const CENTER: Position = Position {
    x: -1410.5204,
    y: 89.05998,
    z: -1740.5852,
};

fn assert_source_step(h: &Harness, actor: Entity, from: Position, target: Position) {
    let speed = h
        .app
        .world()
        .get::<crate::creature_ai::SpawnPoint>(actor)
        .unwrap()
        .speed_run
        * 7.0;
    let delta = Vec3::new(target.x - from.x, target.y - from.y, target.z - from.z);
    let travelled = delta / delta.length() * (speed * 0.05);
    assert_eq!(
        h.app.world().get::<Position>(actor),
        Some(&Position {
            x: from.x + travelled.x,
            y: from.y + travelled.y,
            z: from.z + travelled.z,
        }),
        "actual next50ms must follow the authored source point at native speed"
    );
}

fn assert_own_damage_ingress(difficulty: u32) {
    let (mut h, map, nazan, vazruden) = first_west(difficulty);
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: WEST.x + 3.0,
        ..WEST
    });
    let other_map = spawn_second_copy(&mut h);
    let other = actors_in_copy(&mut h, 17307, other_map)[0];
    damage_to_percent(&mut h, nazan, 35.0);
    assert!(h.health(nazan).current > h.health(nazan).max * 0.3);
    h.advance(0.05);
    assert_eq!(h.app.world().get::<Position>(nazan), Some(&WEST));
    assert_eq!(descent_emotes(&h, nazan), 0);
    assert!(h.has_aura(nazan, 18430));
    // Preserve the source3.2s victim repeat; qualify an actual non-melee cast.
    for _ in 0..65 {
        if h.app
            .world()
            .get::<shared::casting::CastState>(nazan)
            .is_some()
        {
            break;
        }
        h.advance(0.05);
    }
    let fireball = if difficulty == 2 { 32491 } else { 30691 };
    assert_eq!(
        h.app
            .world()
            .get::<shared::casting::CastState>(nazan)
            .unwrap()
            .spell_id,
        fireball
    );
    damage_to_percent(&mut h, nazan, 29.0);
    assert!(h.health(nazan).current < h.health(nazan).max * 0.3);
    assert!(
        h.health(nazan).current > 1.0,
        "nonlethal slice, not the min1 guard"
    );
    for _ in 0..2 {
        h.advance(0.05);
        if h.app.world().get::<Position>(nazan) != Some(&WEST) {
            break;
        }
    }
    assert_eq!(
        descent_emotes(&h, nazan),
        1,
        "own below30 must start source descent without Call30693"
    );
    assert_eq!(
        h.app.world().get::<ReactState>(nazan),
        Some(&ReactState::Passive)
    );
    assert!(!h.has_aura(nazan, 18430));
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(nazan)
            .is_none()
    );
    assert_source_step(&h, nazan, WEST, DESCEND);
    assert_ne!(
        h.app.world().get::<Position>(nazan),
        Some(&DESCEND),
        "stop before arrival/fall"
    );
    let first_step = *h.app.world().get::<Position>(nazan).unwrap();
    damage_to_percent(&mut h, nazan, 28.0);
    h.advance(0.05);
    assert_source_step(&h, nazan, first_step, DESCEND);
    assert_eq!(
        descent_emotes(&h, nazan),
        1,
        "further own damage cannot replay descent"
    );
    assert!(h.health(vazruden).current > h.health(vazruden).max * 0.4);
    assert_eq!(
        received_casts(&h, vazruden, 30693),
        0,
        "no Call spell is fabricated"
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Npc>(other)
            .unwrap()
            .template_id,
        17307
    );
    assert_eq!(descent_emotes(&h, other), 0);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 1),
        Some(EncounterState::InProgress)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
    assert!(h.inbox(0).ends.iter().all(|end| !end.success));
}

#[test]
#[ignore = "RED: own nonlethal below30 descent ingress missing"]
fn encounter_coverage_herald_own_damage_normal_first_descent_step() {
    assert_own_damage_ingress(1);
}

#[test]
#[ignore = "RED: own nonlethal below30 descent ingress missing"]
fn encounter_coverage_herald_own_damage_heroic_first_descent_step() {
    assert_own_damage_ingress(2);
}

#[test]
fn encounter_coverage_herald_own_damage_during_intro_keeps_center_motion() {
    for difficulty in [1, 2] {
        let (mut h, _map, herald, sentries) = prepared_ramparts(difficulty);
        for sentry in sentries {
            kill_actor(&mut h, sentry);
        }
        h.advance(0.05);
        let before = *h.app.world().get::<Position>(herald).unwrap();
        assert_ne!(before, CENTER, "real INTRO1 is still moving");
        damage_to_percent(&mut h, herald, 29.0);
        assert!(h.health(herald).current < h.health(herald).max * 0.3);
        h.advance(0.05);
        assert_source_step(&h, herald, before, CENTER);
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::Npc>(herald)
                .unwrap()
                .template_id,
            17307
        );
        assert_eq!(descent_emotes(&h, herald), 0);
        assert_eq!(
            h.app.world().get::<ReactState>(herald),
            Some(&ReactState::Passive)
        );
    }
}
