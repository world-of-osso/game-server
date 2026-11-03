//! "Fail the test, not the follower": reports carry every APL fact error and reject a non-empty ledger.
use bevy::prelude::World;
use serde_json::Value;
use server::simc_apl::{FactError, collect_fact_errors};

/// Attach every actor's fact errors before gating, so passing reports show the empty ledger.
pub(super) fn attach_fact_errors(world: &mut World, report: &mut Value) -> Result<(), String> {
    let errors = collect_fact_errors(world);
    report["fact_errors"] = serde_json::to_value(&errors)
        .map_err(|error| format!("serialize APL fact errors: {error}"))?;
    check_fact_errors(&errors)
}

pub(super) fn check_fact_errors(errors: &[FactError]) -> Result<(), String> {
    if errors.is_empty() {
        return Ok(());
    }
    let entries: Vec<_> = errors.iter().map(describe_fact_error).collect();
    Err(format!(
        "{} APL fact error(s):\n{}",
        errors.len(),
        entries.join("\n")
    ))
}

fn describe_fact_error(error: &FactError) -> String {
    format!(
        "actor={} target={:?} fact={} count={} detail={}",
        error.actor, error.target, error.fact, error.count, error.detail
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use server::simc_apl::FactErrors;

    #[test]
    fn empty_ledger_passes_and_appears_in_report() {
        let mut world = World::new();
        world.spawn(FactErrors::default());
        let mut report = json!({"spec": 66});
        attach_fact_errors(&mut world, &mut report).unwrap();
        assert_eq!(report["fact_errors"], json!([]));
    }

    #[test]
    fn seeded_fact_errors_are_reported_and_fail() {
        let mut world = World::new();
        let ledger = FactErrors::default();
        ledger.record("target.health", 12, Some(34), "missing health");
        ledger.record("target.health", 12, Some(34), "missing health");
        world.spawn(ledger);
        let other = FactErrors::default();
        other.record("buff.bloodlust.up", 7, None, "unknown aura");
        world.spawn(other);
        let mut report = json!({"spec": 66});
        let failure = attach_fact_errors(&mut world, &mut report).unwrap_err();
        assert_eq!(
            report["fact_errors"],
            json!([
                {"fact":"buff.bloodlust.up","actor":7,"target":null,"detail":"unknown aura","count":1},
                {"fact":"target.health","actor":12,"target":34,"detail":"missing health","count":2},
            ])
        );
        assert_eq!(
            failure,
            "2 APL fact error(s):\n\
             actor=7 target=None fact=buff.bloodlust.up count=1 detail=unknown aura\n\
             actor=12 target=Some(34) fact=target.health count=2 detail=missing health"
        );
    }
}
