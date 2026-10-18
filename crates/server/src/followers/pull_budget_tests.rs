//! Concrete USER policy decisions; synthetic estimates here, real spell/AI integration separately.
use super::pull_budget::*;

fn measured() -> BudgetInput {
    BudgetInput {
        observation_seconds: 10.0,
        completed_fights: 1,
        pack_count: 2,
        overlapping: true,
        party_stable: true,
        healer_available: true,
        tank_health: 900.0,
        tank_max_health: 1000.0,
        hostile_health: 4000.0,
        group_dps: Some(400.0),
        healer_hps: Some(100.0),
        incoming_dps: 140.0,
        opening_burst: 100.0,
        confirmed_cooldown_credit: 0.0,
    }
}

#[test]
fn tank_pull_budget_first_fight_is_one_pack_without_prior_run_rates() {
    let mut input = measured();
    input.completed_fights = 0;
    input.overlapping = false;
    input.group_dps = None;
    input.healer_hps = None;
    input.observation_seconds = 0.0;
    input.pack_count = 1;
    assert!(matches!(
        evaluate(&input),
        Decision::Allow {
            bootstrap: true,
            ..
        }
    ));
    input.pack_count = 2;
    assert_eq!(evaluate(&input), Decision::Wait(Reason::FirstSinglePack));
    input.group_dps = Some(99999.0); // prior-run numbers cannot bypass first-fight policy.
    input.healer_hps = Some(99999.0);
    assert_eq!(evaluate(&input), Decision::Wait(Reason::FirstSinglePack));
    input.pack_count = 1;
    input.overlapping = true;
    assert_eq!(evaluate(&input), Decision::Wait(Reason::FirstSinglePack));
}

#[test]
fn tank_pull_budget_allows_stable_overlap_with_real_ten_second_capacity() {
    assert!(matches!(
        evaluate(&measured()),
        Decision::Allow {
            bootstrap: false,
            ..
        }
    ));
    let mut input = measured();
    input.observation_seconds = 9.99;
    assert_eq!(evaluate(&input), Decision::Wait(Reason::Measurements));
    input.observation_seconds = 10.0;
    input.party_stable = false;
    assert_eq!(evaluate(&input), Decision::Wait(Reason::HealthStability));
}

#[test]
fn tank_pull_budget_forecast_leaves_at_least_thirty_percent_max_health() {
    let mut input = measured();
    input.incoming_dps = 160.0; // 10s forecast:900-(160-100)*10=300, inclusive boundary.
    assert!(matches!(
        evaluate(&input),
        Decision::Allow {
            forecast_health: Some(300.0),
            ..
        }
    ));
    input.incoming_dps = 160.1;
    assert_eq!(evaluate(&input), Decision::Wait(Reason::Reserve));
    input.confirmed_cooldown_credit = 2.0;
    assert!(matches!(evaluate(&input), Decision::Allow { .. }));
    input.confirmed_cooldown_credit = 0.0;
    input.incoming_dps = 80.0;
    input.opening_burst = 601.0;
    assert_eq!(evaluate(&input), Decision::Wait(Reason::Reserve));
}

#[test]
fn tank_pull_budget_healer_incapacity_and_unknown_rates_never_get_nominal_capacity() {
    let mut input = measured();
    input.healer_available = false;
    assert_eq!(evaluate(&input), Decision::Wait(Reason::Healer));
    input.healer_available = true;
    input.healer_hps = None;
    assert_eq!(evaluate(&input), Decision::Wait(Reason::Measurements));
    input.healer_hps = Some(0.0);
    input.group_dps = Some(0.0);
    assert_eq!(evaluate(&input), Decision::Wait(Reason::Measurements));
    input.group_dps = Some(400.0);
    input.incoming_dps = f64::NAN;
    assert_eq!(evaluate(&input), Decision::Wait(Reason::Invalid));
}

#[test]
fn tank_pull_budget_next_unaccounted_burst_is_added_to_sustained_debt() {
    let mut input = measured();
    input.incoming_dps = 160.0;
    input.opening_burst = 100.0;
    // 600 sustained debt plus the next100 damage not already in that rate leaves200,
    // below the unchanged300 reserve. Taking max(600,100) incorrectly admits it.
    assert_eq!(evaluate(&input), Decision::Wait(Reason::Reserve));
}
