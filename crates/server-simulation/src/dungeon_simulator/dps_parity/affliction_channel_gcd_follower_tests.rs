//! Native channel chains must replace an active GCD at the final paid tick.
use super::*;
use shared::casting::{CastState, CastType};

#[test]
#[ignore = "requires prepared265 native profile and world data"]
fn affliction_follower_chains_fast_channel_while_gcd_is_active() {
    let _seed = crate::random::seed_scope(2);
    let mut run = prepare_measurement(265, 3).unwrap();
    engage(&mut run);
    let start = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, start);
    drive_player(&mut run).unwrap();
    let mut retired = Vec::new();
    let mut chained_during_gcd = false;
    for tick in 1..=6000 {
        let world = run.app.world();
        let old = world.get::<CastState>(run.actor).cloned();
        let gcd = world
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .is_some_and(|history| history.has_active_gcd(crate::spell_cast::history_now(world)));
        advance_fixture_frame(
            &mut run,
            start + Duration::from_millis(50 * tick),
            start,
            &mut retired,
            &mut |_| {},
        )
        .expect("a selected channel chain must not fail native GCD admission");
        let new = run.app.world().get::<CastState>(run.actor);
        if let (Some(old), Some(new)) = (old, new) {
            if gcd
                && old.spell_id == 1261153
                && old.cast_type == CastType::Channel
                && new.spell_id == old.spell_id
                && new.elapsed < old.elapsed
            {
                chained_during_gcd = true;
            }
        }
    }
    assert!(
        chained_during_gcd,
        "the real seed must exercise a channel restart during its prior GCD"
    );
}
