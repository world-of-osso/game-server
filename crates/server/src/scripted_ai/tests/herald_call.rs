//! Real below40 damage and ordinary30693 through the first physical descent step.
use super::herald_intro::{actors_in_copy, spawn_second_copy};
use super::herald_intro2::{WEST, prepared_ramparts, reach_center, received_casts};
use super::*;

const DESCEND: Position = Position {
    x: -1429.1720,
    y: 84.34760,
    z: -1749.1290,
};

pub(super) fn first_west(difficulty: u32) -> (Harness, InMap, Entity, Entity) {
    let (mut h, map, nazan, sentries) = prepared_ramparts(difficulty);
    reach_center(&mut h, nazan, &sentries);
    h.advance(1.0);
    let start = *h.app.world().get::<Position>(nazan).unwrap();
    let speed = h
        .app
        .world()
        .get::<crate::creature_ai::SpawnPoint>(nazan)
        .unwrap()
        .speed_run
        * 7.0;
    let ticks = (distance(start, WEST) / (speed * 0.05)).ceil() as usize;
    for _ in 0..=ticks {
        if h.app.world().get::<Position>(nazan) == Some(&WEST) {
            break;
        }
        h.advance(0.05);
    }
    assert_eq!(h.app.world().get::<Position>(nazan), Some(&WEST));
    assert!(h.has_aura(nazan, 18430));
    let vazruden = actors_in_copy(&mut h, 17537, map);
    assert_eq!(vazruden.len(), 1);
    (h, map, nazan, vazruden[0])
}

fn distance(left: Position, right: Position) -> f32 {
    Vec3::new(left.x, left.y, left.z).distance(Vec3::new(right.x, right.y, right.z))
}

pub(super) fn damage_to_percent(h: &mut Harness, actor: Entity, percent: f32) {
    let health = h.health(actor);
    let player = h.player();
    let multiplier = crate::level_scaling::UnitScaling::of(h.app.world(), actor).health_multiplier(
        &crate::level_scaling::UnitScaling::of(h.app.world(), player),
    );
    let target_health = (health.max * percent / 100.0).round();
    let damage = (health.current - target_health) * multiplier;
    assert!(damage > 0.0);
    let expected_loss = (damage / multiplier).trunc();
    crate::combat::deal_direct_damage(h.app.world_mut(), player, actor, damage);
    assert_eq!(h.health(actor).current, health.current - expected_loss);
    assert_eq!(
        h.health(actor).current < health.max * 0.4,
        percent < 40.0,
        "real native integer damage must land on the intended side of40%"
    );
}

pub(super) fn descent_emotes(h: &Harness, nazan: Entity) -> usize {
    h.inbox(0)
        .chat
        .iter()
        .filter(|line| {
            line.channel == shared::protocol::ChatType::RaidBossEmote(nazan.to_bits())
                && line.content == "Nazan descends from the sky."
        })
        .count()
}

fn assert_call_ingress(difficulty: u32) {
    let (mut h, map, nazan, vazruden) = first_west(difficulty);
    // Authored Type41/TextRange0 has the normal25yd listening radius.
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: WEST.x + 3.0,
        ..WEST
    });
    let other_map = spawn_second_copy(&mut h);
    let other_herald = actors_in_copy(&mut h, 17307, other_map)[0];
    damage_to_percent(&mut h, vazruden, 45.0);
    h.advance(0.05);
    assert_eq!(
        received_casts(&h, vazruden, 30693),
        0,
        "above40 cannot call"
    );
    assert!(h.has_aura(nazan, 18430));
    assert_eq!(descent_emotes(&h, nazan), 0);
    let fireball = if difficulty == 2 { 32491 } else { 30691 };
    // The victim repeat remains3.2s; wait for it, never shorten the source clock.
    for _ in 0..65 {
        if h.app
            .world()
            .get::<shared::casting::CastState>(nazan)
            .is_some()
        {
            break;
        }
        h.advance(0.05);
        assert_eq!(received_casts(&h, vazruden, 30693), 0);
    }
    let active = h
        .app
        .world()
        .get::<shared::casting::CastState>(nazan)
        .expect("actual scheduled ordinary Fireball is active before descent");
    assert_eq!(active.spell_id, fireball);
    assert_eq!(h.app.world().get::<Position>(nazan), Some(&WEST));
    damage_to_percent(&mut h, vazruden, 39.0);
    // CALL0 and special DESCEND0 may execute on successive real AI ticks.
    for _ in 0..3 {
        h.advance(0.05);
        if h.app.world().get::<Position>(nazan) != Some(&WEST) {
            break;
        }
    }
    assert_eq!(
        received_casts(&h, vazruden, 30693),
        1,
        "below40 must produce exactly one actual ordinary30693 SpellGo"
    );
    let call = h
        .inbox(0)
        .spell_gos
        .iter()
        .find(|go| go.caster == vazruden.to_bits() && go.spell_id == 30693)
        .unwrap();
    assert_eq!(
        call.target,
        Some(vazruden.to_bits()),
        "ordinary source self-cast"
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
            .is_none(),
        "DESCEND1 interrupts the real non-melee cast"
    );
    assert_eq!(descent_emotes(&h, nazan), 1);
    let step = *h.app.world().get::<Position>(nazan).unwrap();
    assert_ne!(step, WEST, "real point4 movement must advance");
    assert_ne!(
        step, DESCEND,
        "endpoint is strictly before point4 arrival/fall"
    );
    assert!(distance(step, DESCEND) < distance(WEST, DESCEND));
    let speed = h
        .app
        .world()
        .get::<crate::creature_ai::SpawnPoint>(nazan)
        .unwrap()
        .speed_run
        * 7.0;
    let delta = Vec3::new(DESCEND.x - WEST.x, DESCEND.y - WEST.y, DESCEND.z - WEST.z);
    let travelled = delta / delta.length() * (speed * 0.05);
    assert_eq!(
        step,
        Position {
            x: WEST.x + travelled.x,
            y: WEST.y + travelled.y,
            z: WEST.z + travelled.z,
        },
        "first step is exactly one50ms source-speed straight-line step, not teleportation"
    );
    assert!(
        h.app
            .world()
            .get::<crate::scripted_ai::waypoint_path::GravityDisabled>(nazan)
            .is_some()
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Npc>(other_herald)
            .unwrap()
            .template_id,
        17307
    );
    assert_eq!(descent_emotes(&h, other_herald), 0);
    assert!(actors_in_copy(&mut h, 17537, other_map).is_empty());
    damage_to_percent(&mut h, vazruden, 38.0);
    dispatch(h.app.world_mut(), nazan, Hook::DoAction(0));
    h.advance(0.05);
    assert_eq!(
        received_casts(&h, vazruden, 30693),
        1,
        "further damage cannot replay Call"
    );
    assert_eq!(
        descent_emotes(&h, nazan),
        1,
        "duplicate action cannot replay descent"
    );
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
fn encounter_coverage_herald_call_nazan_normal_first_physical_descent_step() {
    assert_call_ingress(1);
}

#[test]
fn encounter_coverage_herald_call_nazan_heroic_first_physical_descent_step() {
    assert_call_ingress(2);
}
