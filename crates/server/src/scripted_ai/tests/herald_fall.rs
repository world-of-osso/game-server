//! TC224–249/328–335: natural point4 arrival, real falling spline, DESCEND3.
use super::herald_call::{damage_to_percent, first_west};
use super::herald_intro::{actors_in_copy, spawn_second_copy};
use super::herald_intro2::received_casts;
use super::*;
use shared::components::ThreatTable;

const DESCEND: Position = Position {
    x: -1429.1720,
    y: 84.34760,
    z: -1749.1290,
};
// MovementTypedefs.h exact f32 gravity.
const GRAVITY: f32 = 19.291_103;

fn source_floor(h: &mut Harness, map: InMap) -> f32 {
    let probe = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, Vec3::new(DESCEND.x, DESCEND.y, DESCEND.z));
    let crate::ground::GroundProbe::Supported(floor) = probe else {
        panic!("native Ramparts fall ray needs supported geometry: {probe:?}");
    };
    assert!(DESCEND.y - floor > 0.1);
    floor
}

fn qualify_spell(h: &Harness) {
    let spell = h.app.world().resource::<SpellStore>().get(20538).unwrap();
    assert_eq!(spell.effects.len(), 1);
    let effect = &spell.effects[0];
    assert_eq!((effect.index, effect.effect, effect.aura), (0, 3, 0));
    assert_eq!(effect.implicit_targets, [22, 15]);
    assert!(effect.radius[0].unwrap().radius > 0.0);
    assert!(spell.power_costs.is_empty());
    assert_eq!(spell.misc.as_ref().unwrap().cast_time.unwrap().base_ms, 0);
    println!("native20538 {}", serde_json::to_string(spell).unwrap());
}

#[test]
fn encounter_coverage_herald_fall_source_qualification() {
    let (mut h, map, nazan, _) = first_west(1);
    qualify_spell(&h);
    let floor = source_floor(&mut h, map);
    // TC Unit::GetHoverOffset only applies while HOVER is set; DESCEND1 removes18430.
    println!(
        "source point4={DESCEND:?}, floor={floor:?}, drop={}, nativeNazan={:?}",
        DESCEND.y - floor,
        h.app.world().get::<shared::components::Npc>(nazan)
    );
}

fn natural_fall(difficulty: u32) {
    let (mut h, map, nazan, vazruden) = first_west(difficulty);
    qualify_spell(&h);
    let floor = source_floor(&mut h, map);
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: DESCEND.x + 3.0,
        y: floor,
        z: DESCEND.z,
    });
    let other_map = spawn_second_copy(&mut h);
    let other = actors_in_copy(&mut h, 17307, other_map)[0];
    let other_health = h.health(other).current;
    damage_to_percent(&mut h, nazan, 29.0);
    let start = *h.app.world().get::<Position>(nazan).unwrap();
    let speed = h
        .app
        .world()
        .get::<crate::creature_ai::SpawnPoint>(nazan)
        .unwrap()
        .speed_run
        * 7.0;
    let distance = Vec3::new(
        start.x - DESCEND.x,
        start.y - DESCEND.y,
        start.z - DESCEND.z,
    )
    .length();
    let budget = (distance / (speed * 0.05)).ceil() as usize + 2;
    for _ in 0..budget {
        h.advance(0.05);
        assert_eq!(
            received_casts(&h, nazan, 20538),
            0,
            "no hate-to-zero before fall"
        );
        if h.app.world().get::<Position>(nazan) == Some(&DESCEND) {
            break;
        }
    }
    assert_eq!(
        h.app.world().get::<Position>(nazan),
        Some(&DESCEND),
        "actual point4 arrival"
    );
    assert!(!h.has_aura(nazan, 18430));
    assert!(
        h.app
            .world()
            .get::<crate::scripted_ai::waypoint_path::GravityDisabled>(nazan)
            .is_some()
    );
    let native_duration_ms = ((2.0 * (DESCEND.y - floor) / GRAVITY).sqrt() * 1000.0) as u32;
    assert!(native_duration_ms > 50 && native_duration_ms < 2_000);
    // Launch is on point4's real arrival tick; no scripted teleport/fake callback.
    for tick in 1..=native_duration_ms.div_ceil(50) {
        let pre_threat = h
            .app
            .world()
            .get::<ThreatTable>(nazan)
            .unwrap()
            .threat_for(player.to_bits());
        assert!(pre_threat > 0.0, "positive native threat before20538");
        h.advance(0.05);
        let position = *h.app.world().get::<Position>(nazan).unwrap();
        assert_eq!((position.x, position.z), (DESCEND.x, DESCEND.z));
        let elapsed = tick * 50;
        if elapsed < native_duration_ms {
            let seconds = elapsed as f32 / 1000.0;
            let expected = (DESCEND.y - seconds * (seconds * GRAVITY * 0.5)).max(floor);
            assert_eq!(
                position.y.to_bits(),
                expected.to_bits(),
                "native falling spline at{elapsed}ms"
            );
            assert!(
                h.app
                    .world()
                    .get::<crate::scripted_ai::waypoint_path::GravityDisabled>(nazan)
                    .is_some()
            );
            assert_eq!(received_casts(&h, nazan, 20538), 0);
        } else {
            assert_eq!(
                position.y.to_bits(),
                floor.to_bits(),
                "exact floor from original fall ray"
            );
            assert!(
                h.app
                    .world()
                    .get::<crate::scripted_ai::waypoint_path::GravityDisabled>(nazan)
                    .is_none(),
                "DESCEND3 enables gravity after EFFECT completion"
            );
            assert_eq!(
                received_casts(&h, nazan, 20538),
                1,
                "ordinary20538 follows actual landing once"
            );
            let threat = h.app.world().get::<ThreatTable>(nazan).unwrap();
            assert_eq!(
                threat.threat_for(player.to_bits()),
                0.0,
                "native20538 actual hit removes100% threat"
            );
            assert!(
                threat
                    .entries()
                    .iter()
                    .any(|entry| entry.entity == player.to_bits()),
                "zero reference survives"
            );
        }
        assert!(
            h.app
                .world()
                .get::<crate::scripted_ai::hellfire_ramparts::HeraldUnkillable>(nazan)
                .is_some()
        );
        assert_eq!(
            h.app.world().get::<ReactState>(nazan),
            Some(&ReactState::Passive)
        );
        assert!(
            h.app
                .world()
                .get::<crate::combat::DeathState>(nazan)
                .is_none()
        );
        assert_eq!(h.health(other).current, other_health);
        assert_eq!(received_casts(&h, other, 20538), 0);
        assert!(h.inbox(0).ends.iter().all(|end| !end.success));
        assert_eq!(
            received_casts(&h, vazruden, 30693),
            0,
            "own30 ingress needs no fabricated Call"
        );
    }
}

#[test]
#[ignore = "HERALD_POINT4_ARRIVAL_MOVE_FALL_UNIMPLEMENTED: genuine RED before production"]
fn encounter_coverage_herald_fall_normal() {
    natural_fall(1);
}

#[test]
#[ignore = "HERALD_POINT4_ARRIVAL_MOVE_FALL_UNIMPLEMENTED: genuine RED before production"]
fn encounter_coverage_herald_fall_heroic() {
    natural_fall(2);
}

#[test]
#[ignore = "HERALD_HATE_TO_ZERO_HIT_UNIMPLEMENTED: native20538 DUMMY hook"]
fn encounter_coverage_herald_fall_hate_to_zero_actual_hit() {
    let (mut h, _, nazan, _) = first_west(1);
    qualify_spell(&h);
    let player = h.player();
    let origin = *h.app.world().get::<Position>(nazan).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: origin.x + 3.0,
        ..origin
    });
    crate::combat::deal_direct_damage(h.app.world_mut(), player, nazan, 1.0);
    assert!(
        h.app
            .world()
            .get::<ThreatTable>(nazan)
            .unwrap()
            .threat_for(player.to_bits())
            > 0.0
    );
    assert!(
        crate::spell_cast::creature_cast(h.app.world_mut(), nazan, 20538, Some(nazan), false)
            .is_ok()
    );
    h.advance(0.05);
    assert_eq!(received_casts(&h, nazan, 20538), 1);
    let threat = h.app.world().get::<ThreatTable>(nazan).unwrap();
    assert_eq!(
        threat.threat_for(player.to_bits()),
        0.0,
        "ordinary native DUMMY hit must zero caster threat"
    );
    assert!(
        threat
            .entries()
            .iter()
            .any(|entry| entry.entity == player.to_bits())
    );
}
