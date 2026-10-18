//! USER project policy, not Retail coefficients. Source estimates are supplied by the
//! shared follower planner; this module never substitutes nominal/prior-run rates.
const OBSERVATION_SECONDS: f64 = 10.0;
const TANK_HP_RESERVE_FRACTION: f64 = 0.30;

pub(super) struct BudgetInput {
    pub observation_seconds: f64,
    pub completed_fights: u32,
    pub pack_count: usize,
    pub overlapping: bool,
    pub party_stable: bool,
    pub healer_available: bool,
    pub tank_health: f64,
    pub tank_max_health: f64,
    pub hostile_health: f64,
    pub group_dps: Option<f64>,
    pub healer_hps: Option<f64>,
    pub incoming_dps: f64,
    pub opening_burst: f64,
    pub confirmed_cooldown_credit: f64,
}

#[derive(Debug, PartialEq)]
pub(super) enum Decision {
    Allow {
        bootstrap: bool,
        forecast_health: Option<f64>,
    },
    Wait(Reason),
}

#[derive(Debug, PartialEq)]
pub(super) enum Reason {
    Invalid,
    Healer,
    FirstSinglePack,
    HealthStability,
    Measurements,
    Reserve,
}

fn valid_estimates(input: &BudgetInput) -> bool {
    let nonnegative = [
        input.incoming_dps,
        input.opening_burst,
        input.confirmed_cooldown_credit,
        input.observation_seconds,
    ];
    if input.pack_count == 0 || input.tank_health > input.tank_max_health {
        return false;
    }
    let positive = [
        input.tank_health,
        input.tank_max_health,
        input.hostile_health,
    ];
    let valid_positive = positive
        .iter()
        .all(|value| value.is_finite() && *value > 0.0);
    valid_positive
        && nonnegative
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
}

fn measured_rates(input: &BudgetInput) -> Option<(f64, f64)> {
    let damage = input.group_dps?;
    let healing = input.healer_hps?;
    let complete = input.observation_seconds >= OBSERVATION_SECONDS;
    let valid_damage = damage.is_finite() && damage > 0.0;
    let valid_healing = healing.is_finite() && healing >= 0.0;
    (complete && valid_damage && valid_healing).then_some((damage, healing))
}

pub(super) fn evaluate(input: &BudgetInput) -> Decision {
    if !valid_estimates(input) {
        return Decision::Wait(Reason::Invalid);
    }
    if !input.healer_available {
        return Decision::Wait(Reason::Healer);
    }
    if input.completed_fights == 0 {
        return if input.pack_count == 1 && !input.overlapping {
            Decision::Allow {
                bootstrap: true,
                forecast_health: None,
            }
        } else {
            Decision::Wait(Reason::FirstSinglePack)
        };
    }
    if input.overlapping && !input.party_stable {
        return Decision::Wait(Reason::HealthStability);
    }
    let Some((damage, healing)) = measured_rates(input) else {
        return Decision::Wait(Reason::Measurements);
    };
    forecast_decision(input, damage, healing)
}

fn forecast_decision(input: &BudgetInput, damage: f64, healing: f64) -> Decision {
    let duration = input.hostile_health / damage;
    let sustained_debt = (input.incoming_dps - healing).max(0.0) * duration;
    let damage_debt = sustained_debt.max(input.opening_burst);
    let prevented = input.confirmed_cooldown_credit.min(damage_debt);
    let forecast = input.tank_health - damage_debt + prevented;
    if forecast >= input.tank_max_health * TANK_HP_RESERVE_FRACTION {
        Decision::Allow {
            bootstrap: false,
            forecast_health: Some(forecast),
        }
    } else {
        Decision::Wait(Reason::Reserve)
    }
}
