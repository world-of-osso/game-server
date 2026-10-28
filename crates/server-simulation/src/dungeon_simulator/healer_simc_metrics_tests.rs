use super::Recorder;
use bevy::prelude::{Real, Time, World};
use shared::casting::CastState;
use shared::components::Health;

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
