use bevy::prelude::{Component, World};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FactError {
    pub fact: String,
    pub actor: u64,
    pub target: Option<u64>,
    pub detail: String,
    pub count: u64,
}

type ErrorKey = (u64, Option<u64>, String);

/// Per-actor APL fact failures, shared with immutable fact views.
#[derive(Component, Clone, Default)]
pub struct FactErrors(Arc<Mutex<BTreeMap<ErrorKey, FactError>>>);

impl FactErrors {
    pub fn record(&self, fact: &str, actor: u64, target: Option<u64>, detail: &str) {
        let mut errors = self.0.lock().expect("APL fact-error ledger poisoned");
        let key = (actor, target, fact.to_owned());
        let entry = errors.entry(key).or_insert_with(|| FactError {
            fact: fact.to_owned(),
            actor,
            target,
            detail: detail.to_owned(),
            count: 0,
        });
        if entry.count == 0 {
            bevy::log::error!(fact, actor, ?target, detail, "native APL fact failed");
        }
        entry.count += 1;
    }

    pub fn snapshot(&self) -> Vec<FactError> {
        self.0
            .lock()
            .expect("APL fact-error ledger poisoned")
            .values()
            .cloned()
            .collect()
    }
}

/// Read all actor ledgers in stable actor, target and fact order.
pub fn collect_fact_errors(world: &mut World) -> Vec<FactError> {
    let mut errors: Vec<_> = world
        .query::<&FactErrors>()
        .iter(world)
        .flat_map(FactErrors::snapshot)
        .collect();
    errors.sort_by(|left, right| {
        (left.actor, left.target, &left.fact).cmp(&(right.actor, right.target, &right.fact))
    });
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_fact_errors_are_counted_per_actor_and_target() {
        let ledger = FactErrors::default();
        ledger.record("target.health", 11, Some(22), "missing health");
        ledger.record("target.health", 11, Some(22), "missing health");
        ledger.record("target.health", 11, Some(33), "missing health");
        assert_eq!(
            ledger.snapshot(),
            vec![
                FactError {
                    fact: "target.health".into(),
                    actor: 11,
                    target: Some(22),
                    detail: "missing health".into(),
                    count: 2
                },
                FactError {
                    fact: "target.health".into(),
                    actor: 11,
                    target: Some(33),
                    detail: "missing health".into(),
                    count: 1
                },
            ]
        );
    }
}
