use super::*;

/// Eastern Kingdoms, where the fixtures stand unless a test moves them.
const MAP_0: InMap = InMap::continent(0);

#[test]
fn ghoststate_stationary_observer_changes_recheck_ghost_visibility() {
    use shared::death::DeathState;
    let mut app = setup_interest_app();
    let connection = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
    let player = app.world().get::<PlayerLink>(connection).unwrap().0;
    app.world_mut().entity_mut(player).insert(DeathState::Alive);
    let target = spawn_interest_target(&mut app, pos(10.0, 0.0, 0.0));
    app.world_mut().entity_mut(target).insert(DeathState::Ghost);
    run_fixed_ticks(&mut app, 1);
    assert_not_visible(
        app.world().get::<NetworkVisibility>(target).unwrap(),
        connection,
        "living observer cannot see ghost",
    );
    app.world_mut().entity_mut(player).insert(DeathState::Ghost);
    run_fixed_ticks(&mut app, 1);
    assert_visible(
        app.world().get::<NetworkVisibility>(target).unwrap(),
        connection,
        "ghost observer sees ghost without moving",
    );
    app.world_mut().entity_mut(player).insert(DeathState::Alive);
    run_fixed_ticks(&mut app, 1);
    assert_not_visible(
        app.world().get::<NetworkVisibility>(target).unwrap(),
        connection,
        "resurrection hides ghosts without moving",
    );
}

#[test]
fn typed_invisibility_detection_rechecks_stationary_and_new_observers() {
    use crate::auras::{
        AuraApplication, AuraSource, RemoveReason, apply_aura, aura_effect_mask, remove_auras,
    };
    let harness = crate::spell_cast::test_harness::harness_with_plugins(
        Vec::new(),
        &[89302, 82343, 100616, 82336],
        1,
        false,
        |app| {
            app.add_plugins(InterestPlugin);
        },
    )
    .expect("world data");
    let mut app = harness.app;
    let first = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
    let player = app.world().get::<PlayerLink>(first).unwrap().0;
    let jaga = spawn_interest_target(&mut app, pos(10.0, 0.0, 0.0));
    let rendan = spawn_interest_target(&mut app, pos(15.0, 0.0, 0.0));
    let apply = |app: &mut App, unit, spell_id| {
        let mask = aura_effect_mask(
            app.world()
                .resource::<crate::spell_info::SpellStore>()
                .get(spell_id)
                .unwrap(),
        );
        apply_aura(
            app.world_mut(),
            &AuraApplication {
                caster: unit,
                target: unit,
                spell_id,
                effect_mask: mask,
                source: AuraSource::Cast,
            },
        );
    };
    apply(&mut app, jaga, 89302); // type35, value100
    apply(&mut app, rendan, 82343); // type12, value100
    run_fixed_ticks(&mut app, 1);
    let visible = |app: &App, unit, connection| {
        app.world()
            .get::<NetworkVisibility>(unit)
            .unwrap()
            .is_visible(connection)
    };
    assert!(!visible(&app, jaga, first) && !visible(&app, rendan, first));
    apply(&mut app, player, 100616); // type35, value1000
    run_fixed_ticks(&mut app, 1);
    assert!(
        visible(&app, jaga, first),
        "eligible stationary observer detects Jaga"
    );
    assert!(
        !visible(&app, rendan, first),
        "wrong type does not detect Rendan"
    );
    let second = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
    run_fixed_ticks(&mut app, 1);
    assert!(!visible(&app, jaga, second));
    apply(&mut app, player, 82336);
    run_fixed_ticks(&mut app, 1);
    assert!(visible(&app, rendan, first));
    remove_auras(app.world_mut(), player, RemoveReason::Cancelled, |aura| {
        aura.spell_id == 100616
    });
    run_fixed_ticks(&mut app, 1);
    assert!(
        !visible(&app, jaga, first),
        "detector removal hides the target again"
    );
    assert!(visible(&app, rendan, first));
}

#[test]
fn arena_shadow_sight_reveals_source_stealth_and_rechecks_stationary_facing_and_expiry() {
    use crate::auras::{AuraApplication, AuraSource, RemoveReason, apply_aura, remove_auras};
    use shared::components::{Health, Level, Player, Rotation, UnitFactionTemplate};
    let harness = crate::spell_cast::test_harness::harness_with_plugins(
        Vec::new(),
        &[5215, 34709],
        1,
        false,
        |app| {
            app.add_plugins(InterestPlugin);
        },
    )
    .unwrap();
    let mut app = harness.app;
    let connection = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
    let player = app.world().get::<PlayerLink>(connection).unwrap().0;
    app.world_mut().entity_mut(player).insert((
        Level(20),
        Health {
            current: 4240.0,
            max: 4240.0,
        },
        Player {
            name: "Observer".into(),
            race: 1,
            class: 11,
            appearance: Default::default(),
        },
        Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
        UnitFactionTemplate(1),
    ));
    let target = spawn_interest_target(&mut app, pos(50.0, 0.0, 0.0));
    app.world_mut().entity_mut(target).insert((
        Level(20),
        Health {
            current: 4240.0,
            max: 4240.0,
        },
        Player {
            name: "Prowler".into(),
            race: 2,
            class: 11,
            appearance: Default::default(),
        },
        UnitFactionTemplate(2),
    ));
    app.world_mut()
        .entity_mut(player)
        .insert(crate::duel::DuelOpponent(target));
    app.world_mut()
        .entity_mut(target)
        .insert(crate::duel::DuelOpponent(player));
    let apply = |app: &mut App, unit, spell_id| {
        apply_aura(
            app.world_mut(),
            &AuraApplication {
                caster: unit,
                target: unit,
                spell_id,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        )
    };
    let visible = |app: &App| {
        app.world()
            .get::<NetworkVisibility>(target)
            .unwrap()
            .is_visible(connection)
    };
    apply(&mut app, target, 5215);
    run_fixed_ticks(&mut app, 1);
    assert!(
        !visible(&app),
        "source Prowl hides a living player beyond detection range"
    );
    assert!(!crate::spell_cast::targets::relation_matches(
        app.world(),
        player,
        target,
        crate::spell_cast::targets::Relation::Enemy
    ));
    apply(&mut app, player, 34709);
    run_fixed_ticks(&mut app, 1);
    assert!(
        visible(&app),
        "physical ShadowSight aura228 reveals source type0 stealth"
    );
    assert!(crate::spell_cast::targets::relation_matches(
        app.world(),
        player,
        target,
        crate::spell_cast::targets::Relation::Enemy
    ));
    app.world_mut().get_mut::<Rotation>(player).unwrap().y = -std::f32::consts::FRAC_PI_2;
    run_fixed_ticks(&mut app, 1);
    assert!(!visible(&app), "TC checks rear arc before aura228");
    app.world_mut().get_mut::<Rotation>(player).unwrap().y = std::f32::consts::FRAC_PI_2;
    run_fixed_ticks(&mut app, 1);
    assert!(visible(&app));
    remove_auras(app.world_mut(), player, RemoveReason::Expired, |aura| {
        aura.spell_id == 34709
    });
    run_fixed_ticks(&mut app, 1);
    assert!(!visible(&app), "expiry hides stationary target again");
    // Level-20 Prowl: base 85 + (20 - BaseLevel 13) * RealPointsPerLevel 5 = 120 hides
    // against 30 + 19 * 5 = 125 detection, so 5 points * 0.3 + 1.5 reach = 3yd.
    for (distance, expected) in [(2.9, true), (3.2, false), (1.0, true)] {
        *app.world_mut().get_mut::<Position>(target).unwrap() = pos(distance, 0.0, 0.0);
        crate::interest::mark_interest_position_changed(
            app.world().get_resource::<InterestDirtyQueue>(),
            target,
        );
        run_fixed_ticks(&mut app, 1);
        assert_eq!(
            visible(&app),
            expected,
            "source level20 detection at {distance}yd"
        );
    }
    remove_auras(app.world_mut(), target, RemoveReason::Expired, |aura| {
        aura.spell_id == 5215
    });
    run_fixed_ticks(&mut app, 1);
    assert!(
        visible(&app),
        "no residual hidden state after Prowl removal"
    );
}

fn make_state_and_conn() -> (NetworkVisibility, Entity) {
    (NetworkVisibility::default(), Entity::from_bits(1))
}

#[path = "interest_group_tests.rs"]
mod group_private;

fn setup_interest_app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(InterestPlugin);
    app
}

fn spawn_interest_observer(app: &mut App, position: Position) -> Entity {
    let player = app.world_mut().spawn((position, MAP_0)).id();
    app.world_mut()
        .spawn((PlayerLink(player), ReplicationSender))
        .id()
}

fn spawn_interest_target(app: &mut App, position: Position) -> Entity {
    app.world_mut()
        .spawn((position, MAP_0, NetworkVisibility::default()))
        .id()
}

fn run_fixed_ticks(app: &mut App, ticks: usize) {
    for _ in 0..ticks {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn pos(x: f32, y: f32, z: f32) -> Position {
    Position { x, y, z }
}

fn assert_visible(state: &NetworkVisibility, conn: Entity, msg: &str) {
    assert!(state.is_visible(conn), "{msg}");
}

fn assert_not_visible(state: &NetworkVisibility, conn: Entity, msg: &str) {
    assert!(!state.is_visible(conn), "{msg}");
}

#[test]
fn new_nearby_entity_is_visible_on_next_fixed_tick() {
    let mut app = setup_interest_app();
    let connection = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
    let target = spawn_interest_target(&mut app, pos(50.0, 0.0, 0.0));

    run_fixed_ticks(&mut app, 1);

    assert!(
        app.world()
            .entity(target)
            .get::<NetworkVisibility>()
            .unwrap()
            .is_visible(connection),
        "a newly visible entity must not wait for a recurring global scan"
    );
}

#[test]
fn unchanged_interest_ticks_do_not_dirty_visibility() {
    let mut app = setup_interest_app();
    let connection = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
    let target = spawn_interest_target(&mut app, pos(50.0, 0.0, 0.0));
    run_fixed_ticks(&mut app, 1);
    assert!(
        app.world()
            .entity(target)
            .get::<NetworkVisibility>()
            .unwrap()
            .is_visible(connection)
    );
    let visibility_tick = app
        .world()
        .entity(target)
        .get_ref::<NetworkVisibility>()
        .unwrap()
        .last_changed();

    run_fixed_ticks(&mut app, 10);

    assert_eq!(
        app.world()
            .entity(target)
            .get_ref::<NetworkVisibility>()
            .unwrap()
            .last_changed(),
        visibility_tick,
        "unchanged fixed ticks must not re-evaluate and dirty replication visibility"
    );
}

#[test]
fn dirty_target_crosses_exact_radius_within_same_bucket() {
    let mut app = setup_interest_app();
    let connection = spawn_interest_observer(&mut app, pos(50.0, 0.0, 0.0));
    let target = spawn_interest_target(&mut app, pos(150.01, 0.0, 0.0));
    run_fixed_ticks(&mut app, 1);
    assert!(
        !app.world()
            .entity(target)
            .get::<NetworkVisibility>()
            .unwrap()
            .is_visible(connection)
    );

    app.world_mut()
        .entity_mut(target)
        .insert(pos(149.99, 0.0, 0.0));
    app.world().resource::<InterestDirtyQueue>().mark(target);
    run_fixed_ticks(&mut app, 1);
    assert!(
        app.world()
            .entity(target)
            .get::<NetworkVisibility>()
            .unwrap()
            .is_visible(connection)
    );

    app.world_mut()
        .entity_mut(target)
        .insert(pos(150.01, 0.0, 0.0));
    app.world().resource::<InterestDirtyQueue>().mark(target);
    run_fixed_ticks(&mut app, 1);
    assert!(
        !app.world()
            .entity(target)
            .get::<NetworkVisibility>()
            .unwrap()
            .is_visible(connection)
    );
}

#[test]
fn dirty_observer_movement_updates_only_its_visibility() {
    let mut app = setup_interest_app();
    let player_a = app.world_mut().spawn((pos(0.0, 0.0, 0.0), MAP_0)).id();
    let connection_a = app
        .world_mut()
        .spawn((PlayerLink(player_a), ReplicationSender))
        .id();
    let player_b = app.world_mut().spawn((pos(300.0, 0.0, 0.0), MAP_0)).id();
    let connection_b = app
        .world_mut()
        .spawn((PlayerLink(player_b), ReplicationSender))
        .id();
    let target = spawn_interest_target(&mut app, pos(50.0, 0.0, 0.0));
    run_fixed_ticks(&mut app, 1);
    let state = app
        .world()
        .entity(target)
        .get::<NetworkVisibility>()
        .unwrap();
    assert!(state.is_visible(connection_a));
    assert!(!state.is_visible(connection_b));

    app.world_mut()
        .entity_mut(player_b)
        .insert(pos(100.0, 0.0, 0.0));
    app.world().resource::<InterestDirtyQueue>().mark(player_b);
    run_fixed_ticks(&mut app, 1);
    let state = app
        .world()
        .entity(target)
        .get::<NetworkVisibility>()
        .unwrap();
    assert!(state.is_visible(connection_a));
    assert!(state.is_visible(connection_b));

    app.world_mut()
        .entity_mut(player_a)
        .insert(pos(200.0, 0.0, 0.0));
    app.world().resource::<InterestDirtyQueue>().mark(player_a);
    run_fixed_ticks(&mut app, 1);
    let state = app
        .world()
        .entity(target)
        .get::<NetworkVisibility>()
        .unwrap();
    assert!(!state.is_visible(connection_a));
    assert!(state.is_visible(connection_b));
}

#[test]
fn visibility_removal_and_readd_do_not_leave_stale_range_state() {
    let mut app = setup_interest_app();
    let connection = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
    let target = spawn_interest_target(&mut app, pos(50.0, 0.0, 0.0));
    run_fixed_ticks(&mut app, 1);
    assert!(
        app.world()
            .entity(target)
            .get::<NetworkVisibility>()
            .unwrap()
            .is_visible(connection)
    );

    app.world_mut()
        .entity_mut(target)
        .remove::<NetworkVisibility>();
    app.world_mut()
        .entity_mut(target)
        .insert(pos(500.0, 0.0, 0.0));
    app.world_mut()
        .entity_mut(target)
        .insert(NetworkVisibility::default());
    run_fixed_ticks(&mut app, 1);

    assert!(
        !app.world()
            .entity(target)
            .get::<NetworkVisibility>()
            .unwrap()
            .is_visible(connection)
    );
}

#[test]
fn reconnect_uses_fresh_observer_visibility() {
    let mut app = setup_interest_app();
    let player = app.world_mut().spawn((pos(0.0, 0.0, 0.0), MAP_0)).id();
    let first_connection = app
        .world_mut()
        .spawn((PlayerLink(player), ReplicationSender))
        .id();
    let target = spawn_interest_target(&mut app, pos(50.0, 0.0, 0.0));
    run_fixed_ticks(&mut app, 1);
    assert!(
        app.world()
            .entity(target)
            .get::<NetworkVisibility>()
            .unwrap()
            .is_visible(first_connection)
    );

    app.world_mut()
        .entity_mut(first_connection)
        .remove::<PlayerLink>();
    let second_connection = app
        .world_mut()
        .spawn((PlayerLink(player), ReplicationSender))
        .id();
    run_fixed_ticks(&mut app, 1);

    assert!(
        app.world()
            .entity(target)
            .get::<NetworkVisibility>()
            .unwrap()
            .is_visible(second_connection)
    );
}

#[test]
fn test_distance_squared_basic() {
    let a = pos(0.0, 0.0, 0.0);
    let b = pos(3.0, 4.0, 0.0);
    assert!((distance_squared(&a, &b) - 25.0).abs() < 0.001);
}

#[test]
fn test_entity_within_range_is_visible() {
    let (mut state, conn) = make_state_and_conn();
    let players = vec![(conn, pos(0.0, 0.0, 0.0))];
    let entity_pos = pos(50.0, 0.0, 0.0);

    update_entity_visibility(&entity_pos, &mut state, &players);
    assert_visible(&state, conn, "entity at 50yd should be visible");
}

#[test]
fn test_entity_beyond_range_is_not_visible() {
    let (mut state, conn) = make_state_and_conn();
    let players = vec![(conn, pos(0.0, 0.0, 0.0))];
    let entity_pos = pos(150.0, 0.0, 0.0);

    state.gain_visibility(conn);
    update_entity_visibility(&entity_pos, &mut state, &players);
    assert_not_visible(&state, conn, "entity at 150yd should not be visible");
}

#[test]
fn test_interest_updates_when_player_moves() {
    let (mut state, conn) = make_state_and_conn();
    let entity_pos = pos(80.0, 0.0, 0.0);

    // Player far away (180yd) — not visible
    update_entity_visibility(&entity_pos, &mut state, &[(conn, pos(-100.0, 0.0, 0.0))]);
    assert_not_visible(&state, conn, "should not be visible when player is far");

    // Player moves close (50yd) — becomes visible
    update_entity_visibility(&entity_pos, &mut state, &[(conn, pos(30.0, 0.0, 0.0))]);
    assert_visible(&state, conn, "should be visible when player moves close");
}

#[test]
fn many_entities_only_nearby_replicated() {
    let conn = Entity::from_bits(1);
    let players = vec![(conn, pos(0.0, 0.0, 0.0))];

    // 20 entities at varied distances
    let distances: Vec<f32> = (0..20).map(|i| i as f32 * 15.0).collect(); // 0, 15, 30, ..., 285
    let mut visible_count = 0;
    let mut culled_count = 0;

    for &dist in &distances {
        let mut state = NetworkVisibility::default();
        let entity_pos = pos(dist, 0.0, 0.0);
        update_entity_visibility(&entity_pos, &mut state, &players);
        if state.is_visible(conn) {
            visible_count += 1;
        } else {
            culled_count += 1;
        }
    }

    // Entities at 0..=100yd visible (0,15,30,45,60,75,90 = 7), at 105+ culled (13)
    assert_eq!(visible_count, 7, "7 entities within 100yd");
    assert_eq!(culled_count, 13, "13 entities beyond 100yd");
}

#[test]
fn entity_at_exactly_100yd_boundary() {
    let conn = Entity::from_bits(1);
    let players = vec![(conn, pos(0.0, 0.0, 0.0))];

    // Exactly 100yd — visible (<=)
    let mut state = NetworkVisibility::default();
    update_entity_visibility(&pos(100.0, 0.0, 0.0), &mut state, &players);
    assert_visible(&state, conn, "exactly 100yd should be visible");

    // Just over 100yd — culled
    let mut state = NetworkVisibility::default();
    update_entity_visibility(&pos(100.01, 0.0, 0.0), &mut state, &players);
    assert_not_visible(&state, conn, "100.01yd should be culled");

    // Just under 100yd — visible
    let mut state = NetworkVisibility::default();
    update_entity_visibility(&pos(99.99, 0.0, 0.0), &mut state, &players);
    assert_visible(&state, conn, "99.99yd should be visible");
}

#[test]
fn teleport_removes_old_area_adds_new_area() {
    let conn = Entity::from_bits(1);

    // Entity near old position (0,0,0)
    let mut old_entity_state = NetworkVisibility::default();
    // Entity near new position (500,0,0)
    let mut new_entity_state = NetworkVisibility::default();

    let old_entity_pos = pos(50.0, 0.0, 0.0);
    let new_entity_pos = pos(520.0, 0.0, 0.0);

    // Player at old position — old entity visible, new entity culled
    let players_old = vec![(conn, pos(0.0, 0.0, 0.0))];
    update_entity_visibility(&old_entity_pos, &mut old_entity_state, &players_old);
    update_entity_visibility(&new_entity_pos, &mut new_entity_state, &players_old);
    assert_visible(
        &old_entity_state,
        conn,
        "old entity visible before teleport",
    );
    assert_not_visible(&new_entity_state, conn, "new entity culled before teleport");

    // Player teleports to (500,0,0) — old entity culled, new entity visible
    let players_new = vec![(conn, pos(500.0, 0.0, 0.0))];
    update_entity_visibility(&old_entity_pos, &mut old_entity_state, &players_new);
    update_entity_visibility(&new_entity_pos, &mut new_entity_state, &players_new);
    assert_not_visible(&old_entity_state, conn, "old entity culled after teleport");
    assert_visible(&new_entity_state, conn, "new entity visible after teleport");
}

#[test]
fn npc_spawn_appears_to_all_nearby_players() {
    let player_a = Entity::from_bits(1);
    let player_b = Entity::from_bits(2);
    let player_c = Entity::from_bits(3);

    let players = vec![
        (player_a, pos(10.0, 0.0, 0.0)),  // 40yd from NPC
        (player_b, pos(80.0, 0.0, 0.0)),  // 30yd from NPC
        (player_c, pos(300.0, 0.0, 0.0)), // 250yd from NPC — out of range
    ];

    // NPC spawns at (50, 0, 0)
    let mut state = NetworkVisibility::default();
    update_entity_visibility(&pos(50.0, 0.0, 0.0), &mut state, &players);

    assert_visible(&state, player_a, "player A within range");
    assert_visible(&state, player_b, "player B within range");
    assert_not_visible(&state, player_c, "player C out of range");
}

#[test]
fn npc_despawn_removed_from_all_observers() {
    let player_a = Entity::from_bits(1);
    let player_b = Entity::from_bits(2);
    let players = vec![
        (player_a, pos(0.0, 0.0, 0.0)),
        (player_b, pos(30.0, 0.0, 0.0)),
    ];

    // NPC visible to both players
    let mut state = NetworkVisibility::default();
    update_entity_visibility(&pos(10.0, 0.0, 0.0), &mut state, &players);
    assert_visible(&state, player_a, "A sees NPC");
    assert_visible(&state, player_b, "B sees NPC");

    // NPC despawns (simulated by moving far away)
    update_entity_visibility(&pos(9999.0, 0.0, 0.0), &mut state, &players);
    assert_not_visible(&state, player_a, "A lost NPC after despawn");
    assert_not_visible(&state, player_b, "B lost NPC after despawn");
}

#[test]
fn two_players_move_toward_each_other_both_gain_visibility() {
    let conn_a = Entity::from_bits(1);
    let conn_b = Entity::from_bits(2);

    // Player A's entity state (visible to B?) and B's entity state (visible to A?)
    let mut state_a = NetworkVisibility::default();
    let mut state_b = NetworkVisibility::default();

    // Players start 200yd apart — neither visible to the other
    let players_far = vec![(conn_a, pos(0.0, 0.0, 0.0)), (conn_b, pos(200.0, 0.0, 0.0))];
    update_entity_visibility(&pos(0.0, 0.0, 0.0), &mut state_a, &players_far);
    update_entity_visibility(&pos(200.0, 0.0, 0.0), &mut state_b, &players_far);
    assert_not_visible(&state_a, conn_b, "A not visible to B when far");
    assert_not_visible(&state_b, conn_a, "B not visible to A when far");

    // Both move toward center — now 50yd apart
    let players_close = vec![
        (conn_a, pos(75.0, 0.0, 0.0)),
        (conn_b, pos(125.0, 0.0, 0.0)),
    ];
    update_entity_visibility(&pos(75.0, 0.0, 0.0), &mut state_a, &players_close);
    update_entity_visibility(&pos(125.0, 0.0, 0.0), &mut state_b, &players_close);
    assert_visible(&state_a, conn_b, "A visible to B after approach");
    assert_visible(&state_b, conn_a, "B visible to A after approach");
}

#[test]
fn two_players_move_apart_both_lose_visibility() {
    let conn_a = Entity::from_bits(1);
    let conn_b = Entity::from_bits(2);
    let mut state_a = NetworkVisibility::default();
    let mut state_b = NetworkVisibility::default();

    // Start 20yd apart — both visible
    let players_close = vec![(conn_a, pos(0.0, 0.0, 0.0)), (conn_b, pos(20.0, 0.0, 0.0))];
    update_entity_visibility(&pos(0.0, 0.0, 0.0), &mut state_a, &players_close);
    update_entity_visibility(&pos(20.0, 0.0, 0.0), &mut state_b, &players_close);
    assert_visible(&state_a, conn_b, "A visible to B when close");
    assert_visible(&state_b, conn_a, "B visible to A when close");

    // Move 200yd apart — both lose visibility
    let players_far = vec![
        (conn_a, pos(-100.0, 0.0, 0.0)),
        (conn_b, pos(100.0, 0.0, 0.0)),
    ];
    update_entity_visibility(&pos(-100.0, 0.0, 0.0), &mut state_a, &players_far);
    update_entity_visibility(&pos(100.0, 0.0, 0.0), &mut state_b, &players_far);
    assert_not_visible(&state_a, conn_b, "A lost B after moving apart");
    assert_not_visible(&state_b, conn_a, "B lost A after moving apart");
}

#[test]
fn stealthed_entity_hidden_beyond_detect_range() {
    // Normal entity at 50yd: visible
    assert!(should_be_visible(50.0 * 50.0, false));

    // Stealthed entity at 50yd: hidden
    assert!(!should_be_visible(50.0 * 50.0, true));

    // Stealthed entity at 8yd: detected (within 10yd)
    assert!(should_be_visible(8.0 * 8.0, true));

    // Stealthed entity at exactly 10yd: detected
    assert!(should_be_visible(10.0 * 10.0, true));

    // Stealthed entity at 11yd: hidden
    assert!(!should_be_visible(11.0 * 11.0, true));
}

#[test]
fn dead_entity_still_visible_for_corpse_loot() {
    // Visibility is distance-only — death state doesn't affect it.
    // A dead NPC at 30yd should still be visible for looting.
    let (mut state, conn) = make_state_and_conn();
    let players = vec![(conn, pos(0.0, 0.0, 0.0))];
    let dead_npc_pos = pos(30.0, 0.0, 0.0);

    // Entity is within range → visible regardless of alive/dead
    update_entity_visibility(&dead_npc_pos, &mut state, &players);
    assert_visible(&state, conn, "dead NPC at 30yd still visible");

    // Re-check after another update (corpse stays visible)
    update_entity_visibility(&dead_npc_pos, &mut state, &players);
    assert_visible(&state, conn, "dead NPC remains visible on re-check");
}

#[test]
fn instance_boundary_entities_in_different_instances_invisible() {
    // Same instance: visible
    assert!(same_instance(0, 0), "both open world");
    assert!(same_instance(42, 42), "same dungeon instance");

    // Different instances: invisible
    assert!(!same_instance(1, 2), "different dungeon instances");
    assert!(!same_instance(0, 1), "open world vs dungeon");
    assert!(!same_instance(100, 200), "different raid instances");
}

#[test]
fn phase_system_different_phase_invisible() {
    // Phase 0 = all phases (always visible)
    assert!(phases_overlap(0, 1), "phase 0 entity visible to all");
    assert!(phases_overlap(1, 0), "all-phase player sees everything");

    // Same phase: visible
    assert!(phases_overlap(1, 1));
    assert!(phases_overlap(0b1010, 0b1010));

    // Overlapping bits: visible
    assert!(phases_overlap(0b1100, 0b0110), "share bit 2");

    // No overlap: invisible
    assert!(!phases_overlap(0b1000, 0b0001), "no shared bits");
    assert!(!phases_overlap(1, 2), "phase 1 vs phase 2");
    assert!(!phases_overlap(0b1100, 0b0011), "disjoint masks");
}

#[test]
fn large_entity_count_performance_bounded() {
    let players: Vec<(Entity, Position)> = (0..10)
        .map(|i| (Entity::from_bits(i + 1), pos(i as f32 * 200.0, 0.0, 0.0)))
        .collect();

    let start = std::time::Instant::now();
    let mut visible_count = 0;

    for i in 0..500 {
        let mut state = NetworkVisibility::default();
        let entity_pos = pos(i as f32 * 2.0, 0.0, 0.0);
        update_entity_visibility(&entity_pos, &mut state, &players);
        for &(conn, _) in &players {
            if state.is_visible(conn) {
                visible_count += 1;
            }
        }
    }

    let elapsed = start.elapsed();
    assert!(visible_count > 0, "some entities visible");
    assert!(
        elapsed.as_millis() < 500,
        "500 entities x 10 players took {}ms (should be <500ms)",
        elapsed.as_millis(),
    );
}

#[test]
fn repeated_update_with_unchanged_positions_is_idempotent() {
    let (mut state, conn) = make_state_and_conn();
    let players = vec![(conn, pos(0.0, 0.0, 0.0))];
    let entity_pos = pos(50.0, 0.0, 0.0);

    // First update: gains visibility
    update_entity_visibility(&entity_pos, &mut state, &players);
    assert_visible(&state, conn, "visible after first update");

    // Repeated updates with same positions: stays visible, no flip-flop
    for _ in 0..10 {
        update_entity_visibility(&entity_pos, &mut state, &players);
        assert_visible(&state, conn, "still visible on repeated update");
    }

    // Same for an out-of-range entity: stays culled
    let mut far_state = NetworkVisibility::default();
    let far_pos = pos(200.0, 0.0, 0.0);
    for _ in 0..10 {
        update_entity_visibility(&far_pos, &mut far_state, &players);
        assert_not_visible(&far_state, conn, "still culled on repeated update");
    }
}

#[test]
fn enemy_faction_corpse_still_visible_for_looting() {
    // Visibility is distance-only — faction doesn't affect it.
    // An enemy faction corpse at 20yd should be visible for looting.
    let (mut state, conn) = make_state_and_conn();
    let players = vec![(conn, pos(0.0, 0.0, 0.0))];
    let enemy_corpse_pos = pos(20.0, 0.0, 0.0);

    update_entity_visibility(&enemy_corpse_pos, &mut state, &players);
    assert_visible(&state, conn, "enemy corpse within range is visible");
}

/// A corpse past its decay is hidden from the players that saw it (AzerothCore
/// `RemoveCorpse`) and shown again when the NPC respawns.
#[test]
fn a_decayed_corpse_leaves_its_observers_and_the_respawn_returns() {
    use crate::combat::{Corpse, DeathState};
    use crate::creature_ai::SpawnPoint;
    use bevy::time::TimeUpdateStrategy;
    use shared::components::Health;

    let mut app = setup_interest_app();
    app.add_plugins(crate::combat::CombatPlugin);
    app.init_resource::<crate::faction::FactionTemplates>();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(50),
    ));
    app.finish();
    app.cleanup();
    let connection = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
    let wolf = app
        .world_mut()
        .spawn((
            pos(5.0, 0.0, 0.0),
            MAP_0,
            NetworkVisibility::default(),
            Replicate::to_clients(NetworkTarget::All),
            Health {
                current: 0.0,
                max: 100.0,
            },
            DeathState { respawn_timer: 3.0 },
            Corpse { remaining: 1.0 },
            SpawnPoint {
                x: 5.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
        ))
        .id();
    app.update();
    run_fixed_ticks(&mut app, 1);
    let visible = |app: &App| {
        app.world()
            .get::<NetworkVisibility>(wolf)
            .is_some_and(|state| state.is_visible(connection))
    };
    assert!(visible(&app), "the fresh corpse is seen");

    // 1.5 s: past the 1 s decay.
    for _ in 0..30 {
        app.update();
    }
    let corpse = app.world().entity(wolf);
    assert!(corpse.get::<Corpse>().is_none());
    assert!(
        !visible(&app),
        "the decayed corpse is hidden from its observer"
    );
    assert!(
        corpse.get::<Replicate>().is_some(),
        "replication stays for the respawn"
    );

    // 3.5 s: respawned at full health and visible again.
    for _ in 0..40 {
        app.update();
    }
    run_fixed_ticks(&mut app, 1);
    assert!(app.world().get::<DeathState>(wolf).is_none());
    assert!(visible(&app), "the respawned NPC is shown again");
}

#[test]
fn a_target_on_another_map_at_the_same_coordinates_stays_hidden() {
    let mut app = setup_interest_app();
    let connection = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
    // Kalimdor (map 1) and a Stockade copy (map 34) at the observer's coordinates.
    let kalimdor = app
        .world_mut()
        .spawn((
            pos(5.0, 0.0, 0.0),
            InMap::continent(1),
            NetworkVisibility::default(),
        ))
        .id();
    let stockade = app
        .world_mut()
        .spawn((
            pos(5.0, 0.0, 0.0),
            InMap {
                map_id: 34,
                instance_id: 1,
            },
            NetworkVisibility::default(),
        ))
        .id();
    let neighbour = spawn_interest_target(&mut app, pos(5.0, 0.0, 0.0));

    run_fixed_ticks(&mut app, 1);

    let visible = |entity: Entity| {
        app.world()
            .entity(entity)
            .get::<NetworkVisibility>()
            .unwrap()
            .is_visible(connection)
    };
    assert!(visible(neighbour));
    assert!(!visible(kalimdor));
    assert!(!visible(stockade));
}

#[test]
fn a_player_leaving_its_map_keeps_only_itself_until_it_arrives() {
    let mut app = setup_interest_app();
    let player = app
        .world_mut()
        .spawn((pos(0.0, 0.0, 0.0), MAP_0, NetworkVisibility::default()))
        .id();
    let connection = app
        .world_mut()
        .spawn((PlayerLink(player), ReplicationSender))
        .id();
    let neighbour = spawn_interest_target(&mut app, pos(5.0, 0.0, 0.0));
    run_fixed_ticks(&mut app, 1);

    // Map transfer: out of map 0, the neighbour no longer sees it either.
    app.world_mut().entity_mut(player).remove::<InMap>();
    app.world().resource::<InterestDirtyQueue>().mark(player);
    run_fixed_ticks(&mut app, 1);
    let visible = |app: &App, entity: Entity| {
        app.world()
            .entity(entity)
            .get::<NetworkVisibility>()
            .unwrap()
            .is_visible(connection)
    };
    assert!(visible(&app, player), "a player always sees itself");
    assert!(!visible(&app, neighbour));

    // Arrival in a Stockade copy at the same coordinates.
    app.world_mut().entity_mut(player).insert(InMap {
        map_id: 34,
        instance_id: 1,
    });
    app.world().resource::<InterestDirtyQueue>().mark(player);
    run_fixed_ticks(&mut app, 1);
    assert!(visible(&app, player));
    assert!(!visible(&app, neighbour));
}

#[test]
fn a_hidden_target_leaves_its_observers_is_not_shown_to_new_ones_and_returns_when_shown() {
    let mut app = setup_interest_app();
    let first = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
    let target = spawn_interest_target(&mut app, pos(10.0, 0.0, 0.0));
    run_fixed_ticks(&mut app, 1);
    let visible_to = |app: &App, connection| {
        app.world()
            .entity(target)
            .get::<NetworkVisibility>()
            .unwrap()
            .is_visible(connection)
    };
    assert!(visible_to(&app, first));
    // Vanish: MOD_INVISIBILITY hides the unit from everyone.
    set_hidden(app.world_mut(), target, true);
    run_fixed_ticks(&mut app, 1);
    assert!(!visible_to(&app, first));
    let second = spawn_interest_observer(&mut app, pos(5.0, 0.0, 0.0));
    run_fixed_ticks(&mut app, 2);
    assert!(
        !visible_to(&app, second),
        "a player arriving later does not see it"
    );
    set_hidden(app.world_mut(), target, false);
    run_fixed_ticks(&mut app, 1);
    assert!(visible_to(&app, first) && visible_to(&app, second));
}

#[test]
fn a_decayed_corpse_is_hidden_from_players_who_arrive_later() {
    let mut app = setup_interest_app();
    let first = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
    let corpse = spawn_interest_target(&mut app, pos(10.0, 0.0, 0.0));
    run_fixed_ticks(&mut app, 1);
    let visible_to = |app: &App, connection| {
        app.world()
            .entity(corpse)
            .get::<NetworkVisibility>()
            .expect("a decayed corpse keeps its interest state")
            .is_visible(connection)
    };
    assert!(visible_to(&app, first));

    // `Creature::RemoveCorpse`: the decay timer ran out.
    app.world_mut()
        .entity_mut(corpse)
        .insert(crate::combat::Corpse { remaining: 0.0 });
    bevy::ecs::system::RunSystemOnce::run_system_once(
        app.world_mut(),
        crate::combat::corpse_decay_system,
    )
    .unwrap();
    run_fixed_ticks(&mut app, 1);
    assert!(!visible_to(&app, first), "the corpse leaves its observers");

    // Replicon shows every entity to a new client unless interest hides it.
    let second = spawn_interest_observer(&mut app, pos(5.0, 0.0, 0.0));
    run_fixed_ticks(&mut app, 2);
    assert!(
        !visible_to(&app, second),
        "a player arriving later does not see it"
    );
}

#[test]
fn dormant_detection_geometry_population_fits_one_catch_up_frame() {
    let mut app = App::new();
    app.init_resource::<InterestIndex>();
    app.add_systems(Update, sync_detection_geometry);
    app.world_mut().spawn_batch((0..250_000).map(|_| {
        (
            Npc {
                template_id: 30,
                name: "Forest Spider".into(),
            },
            crate::grid_activity::GridDormant,
            Level(30),
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            UnitScale(1.0),
        )
    }));
    app.update();
    let fastest = (0..3)
        .map(|_| {
            let start = std::time::Instant::now();
            for _ in 0..5 {
                app.update();
            }
            start.elapsed()
        })
        .min()
        .unwrap();
    eprintln!("250000 dormant detection units/five steps: {fastest:?}");
    assert!(
        fastest < std::time::Duration::from_millis(50),
        "detection scans exhaust frame: {fastest:?}"
    );
}

#[test]
fn waking_npc_refreshes_reciprocal_invisibility_before_visibility_update() {
    let harness = crate::spell_cast::test_harness::harness_with_plugins(
        Vec::new(),
        &[89302],
        1,
        false,
        |app| {
            app.add_plugins(InterestPlugin);
        },
    )
    .expect("native invisibility source");
    let mut app = harness.app;
    let connection = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
    let player = app.world().get::<PlayerLink>(connection).unwrap().0;
    let target = spawn_interest_target(&mut app, pos(10.0, 0.0, 0.0));
    app.world_mut()
        .entity_mut(target)
        .insert(crate::grid_activity::GridDormant);
    run_fixed_ticks(&mut app, 1);
    // A unit added while dormant must not keep the old non-unit geometry on wake.
    app.world_mut().entity_mut(target).insert(Npc {
        template_id: 30,
        name: "Forest Spider".into(),
    });
    let spell = app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(89302)
        .unwrap();
    let effect_mask = crate::auras::aura_effect_mask(spell);
    crate::auras::apply_aura(
        app.world_mut(),
        &crate::auras::AuraApplication {
            caster: player,
            target: player,
            spell_id: 89302,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
    run_fixed_ticks(&mut app, 2);
    app.world_mut()
        .entity_mut(target)
        .remove::<crate::grid_activity::GridDormant>();
    run_fixed_ticks(&mut app, 1);
    assert_not_visible(
        app.world().get::<NetworkVisibility>(target).unwrap(),
        connection,
        "an invisible viewer cannot see a unit that lacks reciprocal detection after wake",
    );
}
