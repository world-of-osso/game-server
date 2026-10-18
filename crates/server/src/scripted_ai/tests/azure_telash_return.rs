//! Native388008 AfterCast, real714 motion/landing and observable source event clocks.
use super::*;
use crate::spell_cast::powers::power_value;
use shared::components::PowerType;

const DAMAGE: u32 = 388008;
const RETURN: u32 = 388082;
const ZERO: u32 = 387928;
const SHIELD: u32 = 388084;
const ICE: u32 = 389453;
const FROST: u32 = 386781;
const ICY: u32 = 387151;
const STEP: f32 = 0.05;
// Native fixture observations are quantized to50ms; allow two collection frames.
const CLOCK_WINDOW: f64 = 2.0 * STEP as f64;
const RETURN_POINTS: [Vec3; 3] = [
    Vec3::new(-5350.0835, 1089.8976, 340.5709),
    Vec3::new(-5310.4464, 1066.8134, 340.5967),
    Vec3::new(-5350.3934, 1043.9323, 340.5894),
];

fn engine_position(wow: Vec3) -> Position {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(wow.x, wow.y, wow.z);
    Position { x, y, z }
}

fn at(h: &Harness, unit: Entity) -> Vec3 {
    let p = h.app.world().get::<Position>(unit).unwrap();
    Vec3::new(p.x, p.y, p.z)
}

fn count_go(h: &Harness, telash: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == telash.to_bits() && go.spell_id == spell)
        .count()
}

fn cast_starts(h: &Harness, telash: Entity, spell: u32) -> usize {
    h.inbox(0)
        .log
        .iter()
        .filter(|event| {
            event.source == Some(telash.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::CastStart
        })
        .count()
}

fn center_cast_fixture() -> (Harness, Entity) {
    let (mut h, telash) = super::azure_frost_bomb::fixture();
    super::azure_absolute_zero::complete_ordinary_rune_activation_phase(&mut h, telash);
    let outbound = super::azure_jump_charge::read_source_jump_seconds(713);
    assert_eq!(super::azure_jump_charge::read_source_jump_seconds(714), 1.0);
    let effect = &h
        .app
        .world()
        .resource::<SpellStore>()
        .get(RETURN)
        .unwrap()
        .effects[0];
    assert_eq!((effect.effect, effect.misc_values[0]), (254, 714));
    h.advance(outbound + 0.1);
    let cast = h.app.world().get::<CastState>(telash).unwrap();
    assert_eq!(
        (cast.spell_id, cast.duration, cast.cast_type),
        (DAMAGE, 8.0, shared::casting::CastType::Normal)
    );
    assert_eq!(count_go(&h, telash, DAMAGE), 0);
    assert!(h.app.world().get::<CreatureAiState>(telash).is_some());
    assert!(h.has_aura(telash, SHIELD));
    assert!(!h.has_aura(telash, ICE));
    (h, telash)
}

fn complete_center_cast(h: &mut Harness, telash: Entity) -> f64 {
    assert_eq!(
        count_go(h, telash, RETURN),
        0,
        "no return before actual native completion"
    );
    let cast = h.app.world().get::<CastState>(telash).unwrap();
    let frames = ((cast.duration - cast.elapsed) / STEP).ceil() as usize + 2;
    for _ in 0..frames {
        h.advance(STEP);
        if count_go(h, telash, DAMAGE) == 1 {
            return crate::auras::aura_now(h.app.world());
        }
    }
    panic!("actual8s388008 must complete before AfterCast RED");
}

fn assert_clock(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= CLOCK_WINDOW,
        "{label}: observed{actual}, source{expected},50ms observation frames"
    );
}

fn record_starts(h: &mut Harness, telash: Entity, seconds: f32, observed: &mut Vec<(u32, f64)>) {
    for _ in 0..(seconds / STEP).round() as usize {
        let old = h.inbox(0).log.len();
        h.advance(STEP);
        let now = crate::auras::aura_now(h.app.world());
        observed.extend(h.inbox(0).log[old..].iter().filter_map(|event| {
            let spell = event.spell_id?;
            (event.source == Some(telash.to_bits())
                && event.kind == CombatLogKind::CastStart
                && [FROST, ICY].contains(&spell))
            .then_some((spell, now))
        }));
    }
}

fn observed_times(observed: &[(u32, f64)], spell: u32, origin: f64) -> Vec<f64> {
    observed
        .iter()
        .filter(|(id, _)| *id == spell)
        .map(|(_, time)| time - origin)
        .collect()
}

#[test]
fn azure_telash_native_aftercast_returns_to_current_max_threat_nearest_point_and_resumes() {
    for (index, point) in RETURN_POINTS.into_iter().enumerate() {
        let (mut h, telash) = center_cast_fixture();
        let player = h.player();
        h.app
            .world_mut()
            .entity_mut(player)
            .insert(engine_position(point));
        let other = h.players[1].0;
        let map = *h.app.world().get::<InMap>(telash).unwrap();
        h.app
            .world_mut()
            .entity_mut(other)
            .insert((map, engine_position(RETURN_POINTS[(index + 1) % 3])));
        let threat = h
            .app
            .world()
            .get::<shared::threat::ThreatTable>(telash)
            .unwrap()
            .entries()
            .iter()
            .find(|entry| entry.entity == player.to_bits())
            .unwrap()
            .threat;
        crate::creature_aggro::add_threat(h.app.world_mut(), telash, other, threat * 1.2);
        assert_eq!(
            (Ctx {
                world: h.app.world_mut(),
                me: telash
            })
            .select_target_max_threat(),
            Some(player)
        );
        let center = at(&h, telash);
        complete_center_cast(&mut h, telash);
        assert_eq!(
            (Ctx {
                world: h.app.world_mut(),
                me: telash
            })
            .select_target_max_threat(),
            Some(player),
            "MaxThreat reference retains incumbent below ranged130%, not numeric top"
        );
        let entries = h
            .app
            .world()
            .get::<shared::threat::ThreatTable>(telash)
            .unwrap()
            .entries();
        let incumbent = entries
            .iter()
            .find(|entry| entry.entity == player.to_bits())
            .unwrap()
            .threat;
        let numeric_top = entries
            .iter()
            .find(|entry| entry.entity == other.to_bits())
            .unwrap()
            .threat;
        assert!(
            numeric_top > incumbent && numeric_top < incumbent * 1.3,
            "actual higher numeric threat does not replace current MaxThreat reference"
        );
        assert_eq!(
            count_go(&h, telash, RETURN),
            1,
            "actual388008 AfterCast must cast388082"
        );
        let destination = engine_position(point);
        let destination = Vec3::new(destination.x, destination.y, destination.z);
        h.advance(0.25);
        let flying = at(&h, telash);
        let horizontal = Vec2::new(flying.x - center.x, flying.z - center.z);
        let total = Vec2::new(destination.x - center.x, destination.z - center.z);
        let fraction = horizontal.dot(total) / total.length_squared();
        assert!(
            fraction > 0.0 && fraction < 1.0,
            "real durational714, not teleport"
        );
        let linear_y = center.y + (destination.y - center.y) * fraction;
        assert!(flying.y > linear_y, "actual source parabolic return motion");
        assert!(h.has_aura(telash, SHIELD));
        assert!(!h.has_aura(telash, ICE));
        assert_eq!(
            h.app.world().get::<ReactState>(telash),
            Some(&ReactState::Passive)
        );
        h.advance(super::azure_jump_charge::read_source_jump_seconds(714));
        assert!(
            at(&h, telash).distance(destination) < 0.001,
            "nearest authored landing{index}"
        );
        assert!(
            h.has_aura(telash, ICE),
            "actual714 landing resumes native energize"
        );
        assert!(!h.has_aura(telash, SHIELD));
        assert_eq!(
            h.app.world().get::<ReactState>(telash),
            Some(&ReactState::Aggressive)
        );
        let energy = power_value(h.app.world(), telash, PowerType::Energy)
            .unwrap()
            .0;
        h.advance(1.0);
        assert_eq!(
            power_value(h.app.world(), telash, PowerType::Energy)
                .unwrap()
                .0,
            energy + 2.0,
            "resumption has a real native periodic consequence, no resource priming"
        );
    }
}

#[test]
fn azure_telash_native_aftercast_counters_follow_source_dispatch_clocks_and_next_center_landing() {
    let (mut h, telash) = center_cast_fixture();
    let player = h.player();
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(engine_position(RETURN_POINTS[0]));
    let misc = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(ICY)
        .unwrap()
        .misc
        .as_ref()
        .unwrap();
    let icy_busy =
        f64::from(misc.cast_time.unwrap().base_ms + misc.duration.unwrap().duration_ms) / 1000.0;
    let origin = complete_center_cast(&mut h, telash);
    let mut observed = Vec::new();
    record_starts(&mut h, telash, 4.1, &mut observed);
    assert!(observed_times(&observed, FROST, origin).is_empty());
    record_starts(&mut h, telash, 0.3, &mut observed);
    assert_eq!(
        observed_times(&observed, FROST, origin).len(),
        1,
        "AfterCast action1 must produce first native Frost at4.3s"
    );
    record_starts(&mut h, telash, 45.6, &mut observed);
    let frost = observed_times(&observed, FROST, origin);
    let icy = observed_times(&observed, ICY, origin);
    assert_eq!(
        (frost.len(), icy.len()),
        (3, 2),
        "source post-phase3Frost/2Icy, not loops"
    );
    assert_clock(frost[0], 4.3, "first Frost");
    assert_clock(icy[0], 15.2, "first Icy");
    assert_clock(
        frost[1],
        (frost[0] + 18.2).max(icy[0] + icy_busy),
        "second Frost waits for casting",
    );
    if icy[1] < frost[2] {
        assert_clock(icy[1], icy[0] + 23.1, "second Icy");
        assert_clock(
            frost[2],
            (icy[1] + 7.3).max(icy[1] + icy_busy),
            "last Icy cross-reschedules final Frost",
        );
    } else {
        assert_clock(frost[2], frost[1] + 15.8, "third Frost");
        assert_clock(
            icy[1],
            frost[2] + 10.8,
            "last Frost cross-reschedules final Icy",
        );
    }
    assert_eq!(
        count_go(&h, telash, ZERO),
        1,
        "no second phase before actual energy"
    );
    let (energy, _) = power_value(h.app.world(), telash, PowerType::Energy).unwrap();
    let casting = h.app.world().get::<CastState>(telash).map_or(0.0, |cast| {
        let remaining = cast.duration - cast.elapsed;
        if cast.spell_id == ICY && cast.cast_type == shared::casting::CastType::Normal {
            remaining + icy_busy as f32
        } else {
            remaining
        }
    });
    let budget = (100.0 - energy) / 2.0 + casting + 0.6;
    for _ in 0..(budget / STEP).ceil() as usize {
        if count_go(&h, telash, ZERO) == 2 {
            break;
        }
        h.advance(STEP);
    }
    assert_eq!(
        count_go(&h, telash, ZERO),
        2,
        "native landing resumed energy into next real phase"
    );
    h.advance(super::azure_jump_charge::read_source_jump_seconds(713) + 0.1);
    let cast = h
        .app
        .world()
        .get::<CastState>(telash)
        .expect("next real center landing starts388008");
    assert_eq!(
        cast.spell_id, DAMAGE,
        "return landing cleared center state, no synthetic MovementInform"
    );
}

#[test]
fn azure_telash_native_aftercast_no_target_icy_does_not_retry_or_consume_a_counter() {
    let (mut h, telash) = center_cast_fixture();
    let player = h.player();
    let near = engine_position(RETURN_POINTS[0]);
    h.app.world_mut().entity_mut(player).insert(near);
    let origin = complete_center_cast(&mut h, telash);
    let mut observed = Vec::new();
    record_starts(&mut h, telash, 4.4, &mut observed);
    assert_eq!(
        observed_times(&observed, FROST, origin).len(),
        1,
        "action1 scheduling prerequisite"
    );
    record_starts(&mut h, telash, 10.6, &mut observed);
    let map = *h.app.world().get::<InMap>(telash).unwrap();
    let mut far = near;
    far.x += 90.0;
    let probe = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .ground_in(map, Vec3::new(far.x, far.y, far.z));
    let crate::ground::GroundProbe::Supported(height) = probe else {
        panic!("native floor for source75yd negative control: {probe:?}");
    };
    far.y = height;
    h.app.world_mut().entity_mut(player).insert(far);
    assert!(
        crate::scripted_ai::model_reach::distance(h.app.world(), telash, player).unwrap() > 75.0
    );
    assert!(h.health(player).current > 0.0);
    record_starts(&mut h, telash, 0.3, &mut observed);
    h.app.world_mut().entity_mut(player).insert(near);
    assert!(
        observed_times(&observed, ICY, origin).is_empty(),
        "no eligible player at first Icy execution"
    );
    record_starts(&mut h, telash, 34.7, &mut observed);
    assert_eq!(observed_times(&observed, FROST, origin).len(), 3);
    assert!(
        observed_times(&observed, ICY, origin).is_empty(),
        "no retry23.1s and no later10.8s cross-reschedule from a falsely decremented Icy counter"
    );
    assert_eq!(count_go(&h, telash, ZERO), 1);
}

#[test]
fn azure_telash_native_aftercast_no_victim_causes_neither_return_nor_action_timers() {
    let (mut h, telash) = center_cast_fixture();
    let player = h.player();
    let original = *h.app.world().get::<Position>(player).unwrap();
    let cast = h.app.world().get::<CastState>(telash).unwrap();
    let remaining = cast.duration - cast.elapsed;
    h.advance(remaining - 0.1);
    assert_eq!(
        h.app.world().get::<CastState>(telash).unwrap().spell_id,
        DAMAGE
    );
    // Authored Leymor sprout origin, same map but outside native388008 radius300.
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(engine_position(Vec3::new(-5141.42, 1276.70, 555.89)));
    let entries = h
        .app
        .world()
        .get::<shared::threat::ThreatTable>(telash)
        .unwrap()
        .entries()
        .to_vec();
    for entry in &entries {
        h.app
            .world_mut()
            .get_mut::<shared::threat::ThreatTable>(telash)
            .unwrap()
            .remove(entry.entity);
    }
    assert_eq!(
        (Ctx {
            world: h.app.world_mut(),
            me: telash
        })
        .select_target_max_threat(),
        None
    );
    complete_center_cast(&mut h, telash);
    assert_eq!(
        (Ctx {
            world: h.app.world_mut(),
            me: telash
        })
        .select_target_max_threat(),
        None,
        "native damage did not recreate a victim before guard observation"
    );
    assert_eq!(count_go(&h, telash, RETURN), 0);
    h.app.world_mut().entity_mut(player).insert(original);
    for entry in entries {
        h.app
            .world_mut()
            .get_mut::<shared::threat::ThreatTable>(telash)
            .unwrap()
            .add_damage_threat(entry.entity, entry.threat, 1.0);
    }
    assert!(
        (Ctx {
            world: h.app.world_mut(),
            me: telash
        })
        .is_engaged()
    );
    let frost = cast_starts(&h, telash, FROST);
    let icy = cast_starts(&h, telash, ICY);
    h.advance(16.0);
    assert_eq!(
        (cast_starts(&h, telash, FROST), cast_starts(&h, telash, ICY)),
        (frost, icy),
        "restored victim exposes any action1 timers wrongly queued before the no-victim guard"
    );
    assert_eq!(count_go(&h, telash, RETURN), 0);
    assert!(!h.has_aura(telash, ICE));
}
