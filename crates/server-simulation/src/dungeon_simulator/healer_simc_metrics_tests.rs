use super::Recorder;
use bevy::prelude::{Real, Time, World};
use shared::casting::CastState;
use shared::components::Health;

#[test]
#[ignore = "requires the MID2 Holy player and canonical world database"]
fn idle_gcd_excludes_native_global_cooldown_after_an_instant_cast() {
    let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let healer = super::super::dps_parity::spawn_second_reference_player(&mut run, 65).unwrap();
    let mut recorder = Recorder::start(run.app.world(), healer, run.actor, vec![healer]);
    recorder.observe_idle_gcd(run.app.world());
    crate::spell_cast::creature_cast(run.app.world_mut(), healer, 20473, Some(healer), false)
        .unwrap();
    assert!(run.app.world().get::<CastState>(healer).is_none());
    for _ in 0..19 {
        recorder.observe_idle_gcd(run.app.world());
    }
    assert_eq!(recorder.report(1)["idle_gcd_pct"], 5.0);
}

#[test]
fn idle_gcd_counts_only_living_frames_without_cast_or_channel() {
    let mut world = World::new();
    world.insert_resource(Time::<Real>::default());
    world.insert_resource(crate::spell_cast::test_harness::ObservedCombatEvents::default());
    let healer = world
        .spawn(Health {
            current: 100.0,
            max: 100.0,
        })
        .id();
    let mut recorder = Recorder::start(&world, healer, healer, vec![healer]);
    for _ in 0..5 {
        recorder.observe_idle_gcd(&world);
    }
    world
        .entity_mut(healer)
        .insert(CastState::normal(1, 0, 2.0, false));
    for _ in 0..5 {
        recorder.observe_idle_gcd(&world);
    }
    world
        .entity_mut(healer)
        .insert(CastState::channel(2, 0, 2.0, 1.0, false));
    for _ in 0..5 {
        recorder.observe_idle_gcd(&world);
    }
    world.entity_mut(healer).remove::<CastState>();
    world.get_mut::<Health>(healer).unwrap().current = 0.0;
    for _ in 0..5 {
        recorder.observe_idle_gcd(&world);
    }
    assert_eq!(recorder.report(1)["idle_gcd_pct"], 25.0);
}
