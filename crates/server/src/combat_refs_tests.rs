use std::time::Duration;

use shared::components::{Health, Player, ThreatTable};

use super::*;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, expire_combat_refs_system);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(100),
    ));
    app.update();
    app
}

fn health() -> Health {
    Health {
        current: 100.0,
        max: 100.0,
    }
}

fn player(app: &mut App, name: &str) -> Entity {
    app.world_mut()
        .spawn((
            Player {
                name: name.into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            health(),
        ))
        .id()
}

/// A creature with a threat list: its AI ends its fights.
fn creature(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((health(), ThreatTable::default()))
        .id()
}

/// A creature without a threat list (a training dummy, a critter).
fn passive_creature(app: &mut App) -> Entity {
    app.world_mut().spawn(health()).id()
}

fn in_combat(app: &App, unit: Entity) -> bool {
    app.world()
        .get::<CombatStatus>(unit)
        .is_some_and(|status| status.0)
}

fn advance_secs(app: &mut App, secs: f64) {
    for _ in 0..(secs * 10.0).round() as usize {
        app.update();
    }
}

#[test]
fn a_fight_with_a_creature_lasts_until_it_stops_combat_however_long_the_lull() {
    let mut app = app();
    let hero = player(&mut app, "Hero");
    let wolf = creature(&mut app);
    set_in_combat_with(app.world_mut(), hero, wolf);
    assert!(in_combat(&app, hero) && in_combat(&app, wolf));

    advance_secs(&mut app, 30.0);
    assert!(in_combat(&app, hero) && in_combat(&app, wolf));

    // Death or evade: `Unit::CombatStop` ends the fight on both sides at once.
    crate::combat::combat_stop(app.world_mut(), wolf);
    assert!(!in_combat(&app, hero) && !in_combat(&app, wolf));
}

#[test]
fn a_unit_fighting_two_creatures_stays_in_combat_until_both_fights_end() {
    let mut app = app();
    let hero = player(&mut app, "Hero");
    let wolf = creature(&mut app);
    let bear = creature(&mut app);
    set_in_combat_with(app.world_mut(), hero, wolf);
    set_in_combat_with(app.world_mut(), bear, hero);

    crate::combat::combat_stop(app.world_mut(), wolf);
    assert!(in_combat(&app, hero) && in_combat(&app, bear));
    crate::combat::combat_stop(app.world_mut(), bear);
    assert!(!in_combat(&app, hero));
}

#[test]
fn player_versus_player_combat_ends_five_seconds_after_its_last_refresh() {
    let mut app = app();
    let alice = player(&mut app, "Alice");
    let bob = player(&mut app, "Bob");
    set_in_combat_with(app.world_mut(), alice, bob);
    advance_secs(&mut app, 4.0);
    set_in_combat_with(app.world_mut(), bob, alice);
    advance_secs(&mut app, 4.0);
    assert!(in_combat(&app, alice) && in_combat(&app, bob));

    advance_secs(&mut app, 1.5);
    assert!(!in_combat(&app, alice) && !in_combat(&app, bob));
}

#[test]
fn a_fight_with_a_creature_without_a_threat_list_times_out_after_five_seconds() {
    let mut app = app();
    let hero = player(&mut app, "Hero");
    let dummy = passive_creature(&mut app);
    set_in_combat_with(app.world_mut(), hero, dummy);
    advance_secs(&mut app, 4.5);
    assert!(in_combat(&app, hero) && in_combat(&app, dummy));

    advance_secs(&mut app, 1.0);
    assert!(!in_combat(&app, hero) && !in_combat(&app, dummy));
}

#[test]
fn a_despawned_opponent_ends_the_fight_and_the_dead_cannot_begin_one() {
    let mut app = app();
    let hero = player(&mut app, "Hero");
    let wolf = creature(&mut app);
    set_in_combat_with(app.world_mut(), hero, wolf);
    app.world_mut().despawn(wolf);
    app.update();
    assert!(!in_combat(&app, hero));

    let corpse = creature(&mut app);
    app.world_mut().get_mut::<Health>(corpse).unwrap().current = 0.0;
    set_in_combat_with(app.world_mut(), hero, corpse);
    set_in_combat_with(app.world_mut(), hero, hero);
    assert!(!in_combat(&app, hero));
}
