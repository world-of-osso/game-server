use super::*;

fn wsg_instance() -> (BgInstance, BgTemplate) {
    let tmpl = warsong_gulch();
    let bg_match = BgMatch {
        bg_id: 1,
        team_a: vec![1, 2, 3, 4, 5],
        team_b: vec![6, 7, 8, 9, 10],
    };
    (BgInstance::from_match(&bg_match, &tmpl), tmpl)
}

#[path = "battleground_tests/integration.rs"]
mod integration;
#[path = "battleground_tests/rewards_and_deserter.rs"]
mod rewards_and_deserter;
#[path = "battleground_tests/state_and_objectives.rs"]
mod state_and_objectives;
#[path = "battleground_tests/templates_and_queue.rs"]
mod templates_and_queue;
