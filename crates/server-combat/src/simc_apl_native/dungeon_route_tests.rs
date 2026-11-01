//! SimC76f8118 DungeonRoute: sim-level `fight_remains` and the per-wave add TTD.
use super::*;
use crate::simc_apl::Program;
use shared::components::Health;

/// A pull of `health` adds engaged at 0 s, read `now` seconds later, after a
/// 2 s precombat moved the combat clock.
fn pull(health: &[f32], now: u64) -> (World, Entity, Vec<Entity>, Encounter) {
    let mut world = World::new();
    let mut clock = Time::<()>::default();
    clock.advance_by(Duration::from_secs(now));
    world.insert_resource(clock);
    let actor = world.spawn_empty().id();
    let adds = health
        .iter()
        .map(|&current| {
            world
                .spawn(Health {
                    current,
                    max: 1000.0,
                })
                .id()
        })
        .collect();
    let encounter = Encounter::Dungeon {
        started_at: 2.0,
        pulled_at: 0.0,
    };
    (world, actor, adds, encounter)
}

fn target_ttd(world: &World, actor: Entity, adds: &[Entity], add: Entity, pull: Encounter) -> f64 {
    let bindings = Bindings::default();
    NativeFacts::new(world, actor, &bindings, pull, adds.len())
        .with_targets(Some(add), adds)
        .value("target.time_to_die")
        .unwrap()
}

#[test]
fn class_follower_dying_dungeon_pack_keeps_vengeance_cooldowns_gated() {
    // SimC76f8118 vengeance APL (profiles/581.profile.json) dungeon gating rows.
    let program = Program::parse(&[
        (
            "default",
            "variable,name=is_dungeon,value=fight_style.dungeonroute|fight_style.dungeonslice",
        ),
        ("default", "variable,name=execute,value=fight_remains<20"),
        ("default", "cycling_variable,name=dung_pull_ttd,op=reset"),
        (
            "default",
            "cycling_variable,name=dung_pull_ttd,op=max,value=target.time_to_die",
        ),
        (
            "default",
            "variable,name=dung_next_pull,value=variable.is_dungeon&raid_event.adds.exists&raid_event.pull.remains<12&(raid_event.adds.has_boss|raid_event.adds.count>=3)",
        ),
        (
            "default",
            "variable,name=dung_cd_ok,value=variable.execute|!variable.is_dungeon|(variable.dung_pull_ttd>12&!variable.dung_next_pull)",
        ),
        ("default", "soul_carver,if=variable.dung_cd_ok"),
    ])
    .unwrap();
    let bindings = Bindings::default();
    let select = |health: &[f32]| {
        let (world, actor, adds, encounter) = pull(health, 30);
        let facts = NativeFacts::new(&world, actor, &bindings, encounter, adds.len())
            .with_targets(Some(adds[0]), &adds);
        let mut state = program.state();
        let chosen = program
            .select("default", &facts, &mut state, |_| Ok(true))
            .unwrap()
            .map(|action| action.name);
        (
            facts.value("fight_remains").unwrap(),
            state.value("execute"),
            state.value("dung_pull_ttd").unwrap(),
            chosen,
        )
    };
    // Three adds at 50/1000 after 30 s: the pack dies in ~1.6 s, but the
    // 45-minute route has 2672 s left, so execute cannot open the gate.
    let (remains, execute, pull_ttd, chosen) = select(&[50.0, 50.0, 50.0]);
    assert_eq!(remains, 2700.0 - 28.0);
    assert_eq!(execute, Some(0.0));
    assert!((pull_ttd - 50.0 / (2850.0 / 30.0 / 3.0)).abs() < 1e-9);
    assert_eq!(chosen, None);
    // The same route with a fresh pack still spends the cooldown.
    let (_, execute, pull_ttd, chosen) = select(&[900.0, 900.0, 900.0]);
    assert_eq!(execute, Some(0.0));
    assert!((pull_ttd - 270.0).abs() < 1e-9);
    assert_eq!(chosen.as_deref(), Some("soul_carver"));
}

#[test]
fn class_follower_focus_fired_pack_splits_wave_damage_over_living_adds() {
    // After 20 s of the pull: one add focused to 200, two untouched, one dead.
    let (world, actor, adds, encounter) = pull(&[200.0, 1000.0, 1000.0, 0.0], 20);
    let ttd = |add| target_ttd(&world, actor, &adds, add, encounter);
    // SimC76f8118 raid_event.cpp415–430: 800 lost over the 20 s since the
    // pull, not the 18 s since combat started, split over three living adds.
    let per_add_dps = 800.0 / 20.0 / 3.0;
    assert!((ttd(adds[0]) - 200.0 / per_add_dps).abs() < 1e-9);
    assert!((ttd(adds[1]) - 1000.0 / per_add_dps).abs() < 1e-9);
    assert!((ttd(adds[2]) - 1000.0 / per_add_dps).abs() < 1e-9);
    assert_eq!(ttd(adds[3]), 0.0);
    // Without wave damage, player.cpp7754: route length minus elapsed.
    let (world, actor, adds, encounter) = pull(&[1000.0, 1000.0], 20);
    assert_eq!(
        target_ttd(&world, actor, &adds, adds[1], encounter),
        2700.0 - 18.0
    );
}
