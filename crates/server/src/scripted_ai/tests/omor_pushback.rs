//! Native Omor damage and cast lifetime, without changing imported clocks or actor budgets.
use super::omor::orbital_fixture;
use super::*;
use shared::components::CastState;

const SHADOW_BOLT: u32 = 30686;
const NATIVE_CAST_SECONDS: f32 = 3.0;
const CLOCK_EPSILON: f64 = 0.0002;

fn cast_progress(h: &Harness, boss: Entity) -> (f32, f32, u8) {
    let cast = h
        .app
        .world()
        .get::<CastState>(boss)
        .expect("actual native Shadow Bolt bar");
    assert_eq!(cast.spell_id, SHADOW_BOLT);
    (cast.duration, cast.elapsed, cast.pushback_count)
}

fn remaining_seconds(h: &Harness, boss: Entity) -> f32 {
    let (duration, elapsed, _) = cast_progress(h, boss);
    duration - elapsed
}

fn observed_completion_deadline(h: &Harness, boss: Entity) -> f64 {
    h.app.world().resource::<Time<Fixed>>().elapsed_secs_f64()
        + f64::from(remaining_seconds(h, boss))
}

#[test]
fn omor_early_native_damage_caps_remaining_timer_and_keeps_two_hit_limit() {
    let (mut h, boss, _) = orbital_fixture();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: origin.x + 5.0,
        ..origin
    });
    h.engage(boss);
    h.advance(0.4);
    let (duration, issued_elapsed, count) = cast_progress(&h, boss);
    assert_eq!((duration, count), (NATIVE_CAST_SECONDS, 0));
    assert!(
        (0.30..=0.40).contains(&issued_elapsed),
        "early native hit elapsed={issued_elapsed}"
    );
    let before_first = observed_completion_deadline(&h, boss);
    // Actual Fireball damage, including the real below20% shield threshold.
    h.hit_pct(boss, 81.0);
    h.advance(0.05); // Existing deferred damage-event boundary; no clock override.
    let (duration, delivered_elapsed, count) = cast_progress(&h, boss);
    let remaining = remaining_seconds(&h, boss);
    eprintln!(
        "Omor early hit: issued_elapsed={issued_elapsed}, delivered_elapsed={delivered_elapsed}, duration={duration}, remaining={remaining}, count={count}"
    );
    assert_eq!(count, 1);
    assert!(
        (2.95..=NATIVE_CAST_SECONDS + 0.0001).contains(&remaining),
        "TC caps remaining timer at native3s, remaining={remaining}"
    );
    let first_delay = observed_completion_deadline(&h, boss) - before_first;
    assert!(first_delay >= f64::from(issued_elapsed) - CLOCK_EPSILON);
    assert!(
        first_delay <= f64::from(issued_elapsed) + 0.05 + CLOCK_EPSILON,
        "only elapsed progress plus one native event tick may be restored, delay={first_delay}"
    );

    // A second early hit may restore progress only up to the same native cap.
    h.advance(0.1);
    h.hit_pct(boss, 0.1);
    h.advance(0.05);
    assert_eq!(cast_progress(&h, boss).2, 2);
    assert!((2.95..=NATIVE_CAST_SECONDS + 0.0001).contains(&remaining_seconds(&h, boss)));

    // The third real hit must not move the absolute completion deadline.
    h.advance(0.1);
    let before_third = observed_completion_deadline(&h, boss);
    h.hit_pct(boss, 0.1);
    h.advance(0.05);
    assert_eq!(cast_progress(&h, boss).2, 2);
    assert!((observed_completion_deadline(&h, boss) - before_third).abs() < CLOCK_EPSILON);
    h.advance(3.2);
    assert!(h.app.world().get::<CastState>(boss).is_none());
    assert_eq!(
        h.inbox(0)
            .spell_gos
            .iter()
            .filter(|go| go.caster == boss.to_bits() && go.spell_id == SHADOW_BOLT)
            .count(),
        1,
        "normal cast must actually finish, not silently stop"
    );
    assert!(
        h.has_aura(boss, 31901),
        "actual threshold shield is released after completion"
    );
}
