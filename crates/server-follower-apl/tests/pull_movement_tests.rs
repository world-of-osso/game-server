//! Stockade pull movement with a production class tank casting through its real APL.
use bevy::prelude::*;
use server_combat::followers::stockade_scenario::{self, Scenario};
use shared::components::Position;

fn configure(app: &mut App) {
    app.add_plugins(server_spell_scripts::SpellScriptsPlugin);
    app.add_plugins(server_combat::stats::StatsPlugin);
    server_follower_apl::action_metadata::install_provider(app);
}

fn route() -> Scenario {
    stockade_scenario::route(configure, server_follower_apl::drive_followers)
}

fn engaged() -> (Scenario, Position) {
    stockade_scenario::engaged(configure, server_follower_apl::drive_followers)
}

#[test]
fn tank_pull_movement_regroup_cannot_replace_the_admitted_position() {
    stockade_scenario::assert_regroup_cannot_replace_the_admitted_position(route());
}

#[test]
fn tank_pull_movement_apl_pursuit_cannot_replace_the_admitted_position() {
    stockade_scenario::assert_pursuit_cannot_replace_the_admitted_position(engaged());
}

#[test]
fn production_tank_pending_admission_holds_native_chase_without_losing_victim() {
    stockade_scenario::assert_pending_admission_holds_native_chase(engaged());
}

#[test]
fn production_tank_incomplete_run_admission_keeps_pursuit_held() {
    stockade_scenario::assert_incomplete_run_admission_keeps_pursuit_held(engaged());
}

#[test]
fn production_tank_goal_allows_existing_fight_defense_without_a_healer() {
    stockade_scenario::assert_existing_fight_defense_without_a_healer(engaged());
}

#[test]
fn production_tank_goal_refuses_unbudgeted_pursuit_without_injected_position() {
    stockade_scenario::assert_unbudgeted_pursuit_refused_without_injected_position(engaged());
}

#[test]
fn tank_reaches_moving_owner_attacker_without_pulling_nearby_idle_pack() {
    stockade_scenario::assert_tank_reaches_moving_owner_attacker_without_pulling_nearby_idle_pack(
        configure,
    );
}
