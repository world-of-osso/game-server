//! Fixture adapter over the production APL.
use super::literal_rotation::Driver;
use crate::simc_apl_native::Encounter;
use bevy::prelude::*;
use facts::DevFacts;
use serde_json::Value;
pub(super) use server_follower_apl::devastation_rotation::*;

#[cfg(test)]
mod channel_tests;
#[cfg(test)]
mod fact_tests;
pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    install_bombardments_simulation(run)?;
    super::prepare_production_driver(run)
}

fn install_bombardments_simulation(run: &mut super::Run) -> Result<(), String> {
    let read = |key: &str, default: &str| -> Result<String, String> {
        match run.profile.options.get(key) {
            None => Ok(default.into()),
            Some(Value::String(value)) => Ok(value.clone()),
            Some(Value::Bool(value)) => Ok(value.to_string()),
            Some(Value::Number(value)) => Ok(value.to_string()),
            Some(_) => Err(format!("SimC {key} must be a scalar option")),
        }
    };
    // sc_evoker.cpp1333–1335/10884–10888: these are simulation options, not native values.
    let enabled = read("evoker.simulate_bombardments", "1")?;
    match enabled.as_str() {
        "0" | "false" => return Ok(()),
        "1" | "true" => {}
        _ => {
            return Err(format!(
                "invalid SimC evoker.simulate_bombardments={enabled}"
            ));
        }
    }
    let mean = read(
        "evoker.simulate_bombardments_time_between_procs_mean",
        "0.2",
    )?
    .parse::<f64>()
    .map_err(|error| format!("SimC Bombardments mean: {error}"))?;
    let deviation = read(
        "evoker.simulate_bombardments_time_between_procs_stddev",
        "0.1",
    )?
    .parse::<f64>()
    .map_err(|error| format!("SimC Bombardments deviation: {error}"))?;
    server_spell_scripts::bombardments::install_simulation(&mut run.app, run.actor, mean, deviation)
}

pub(super) const REPORT_KEY: Option<&str> = Some("devastation_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_DEVASTATION_76f8118"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.drive(world, actor, targets)
    }

    fn report(&self) -> serde_json::Value {
        self.report()
    }
}
