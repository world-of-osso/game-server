//! Selected Surging Shots and Focus Fire through real native casts.
use super::marksmanship_mechanics_tests::{cast, owned_buff, player};
use super::*;

fn rapid_remaining(run: &Run) -> u64 {
    let world = run.app.world();
    let rapid = world
        .resource::<crate::spell_info::SpellStore>()
        .get(257044)
        .unwrap();
    world
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap()
        .remaining_cooldown_ms(rapid, crate::spell_cast::history_now(world))
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_mm_surging_shots_resets_rapid_and_focus_fire_ends_with_channel() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = player(1);
    assert!(owned_buff(&run, 391559), "selected native Surging Shots");
    assert!(owned_buff(&run, 1277546), "selected native Focus Fire");
    for _ in 0..50 {
        cast(&mut run, 257044, false);
        assert!(rapid_remaining(&run) > 0, "real Rapid Fire starts cooldown");
        cast(&mut run, 19434, false);
        if owned_buff(&run, 1277549) {
            assert_eq!(
                rapid_remaining(&run),
                0,
                "earned Surging Shots resets the running cooldown"
            );
            let world = run.app.world();
            let rapid_tick = world
                .resource::<crate::spell_info::SpellStore>()
                .get(257045)
                .unwrap();
            let boosted = crate::auras::spell_mods::modify(
                world,
                run.actor,
                rapid_tick,
                crate::auras::spell_mods::SpellModOp::HealingAndDamage,
                1.0,
            );
            assert!(
                boosted > 1.0,
                "earned Focus Fire modifies real Rapid Fire ticks"
            );
            cast(&mut run, 257044, false);
            assert!(
                !owned_buff(&run, 1277549),
                "the actual Rapid Fire channel consumes Focus Fire"
            );
            return;
        }
    }
    panic!("selected native Surging Shots never procced on actual Aimed Shots");
}
