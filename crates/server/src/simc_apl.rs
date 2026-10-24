//! Literal SimC76f8118 APL decisions; native numeric facts/castability stay caller-owned.
mod expression;
mod program;
pub use program::Program;
use std::collections::{BTreeMap, BTreeSet};

pub trait Facts {
    fn value(&self, name: &str) -> Result<f64, String>;
}

/// Program operands such as plain gcd/combo_strike belong to the current action.
/// Native adapters must supply this explicitly; no actor-wide fallback is assumed.
pub trait ActionFacts: Facts {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String>;

    /// Explicit native candidate order; unsupported adapters cannot invent a target.
    fn target_ids(&self, _action: &Action) -> Result<Vec<u64>, String> {
        Err("target_if requires explicit native target candidates".into())
    }
}

impl Facts for BTreeMap<String, f64> {
    fn value(&self, name: &str) -> Result<f64, String> {
        self.get(name)
            .copied()
            .ok_or_else(|| format!("unknown APL operand {name}"))
    }
}

impl ActionFacts for BTreeMap<String, f64> {
    fn value_for(&self, _action: &Action, name: &str) -> Result<f64, String> {
        Facts::value(self, name)
    }
}

#[derive(Debug)]
pub struct Expression {
    root: expression::Node,
    operands: Vec<String>,
}

impl Expression {
    pub fn parse(source: &str) -> Result<Self, String> {
        let root = expression::parse(source)
            .map_err(|error| format!("APL expression {source:?}: {error}"))?;
        let mut operands = BTreeSet::new();
        root.collect_operands(&mut operands);
        let operands = operands.into_iter().map(str::to_owned).collect();
        Ok(Self { root, operands })
    }

    /// Bind every operand before evaluation: unsupported facts cannot disappear
    /// behind a false branch. Valid numeric operators retain native short-circuiting.
    pub fn evaluate(&self, facts: &dyn Facts) -> Result<f64, String> {
        let values = self
            .operands
            .iter()
            .map(|name| facts.value(name).map(|value| (name.as_str(), value)))
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(self.root.evaluate(&values))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    pub name: String,
    /// Selected native target identity; None leaves the caller's current target unchanged.
    pub target: Option<u64>,
    /// Non-if options are preserved for native action/target adapters, not discarded.
    pub options: BTreeMap<String, String>,
}

#[derive(Default)]
pub struct Variables {
    values: BTreeMap<String, f64>,
}

impl Variables {
    pub fn value(&self, name: &str) -> Option<f64> {
        self.values.get(&name.to_ascii_lowercase()).copied()
    }
}

#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod minmax_tests;
#[cfg(test)]
mod program_tests;
#[cfg(test)]
mod run_tests;
#[cfg(test)]
mod target_tests;
#[cfg(test)]
mod tests;
