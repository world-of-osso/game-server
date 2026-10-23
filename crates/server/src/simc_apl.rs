//! Literal SimC76f8118 APL expressions and ordered lists; native facts are caller-owned.
use std::collections::BTreeMap;

pub trait Facts {
    fn value(&self, name: &str) -> Result<f64, String>;
}

impl Facts for BTreeMap<String, f64> {
    fn value(&self, name: &str) -> Result<f64, String> {
        self.get(name)
            .copied()
            .ok_or_else(|| format!("unknown APL operand {name}"))
    }
}

pub struct Expression {
    source: String,
}

impl Expression {
    pub fn parse(source: &str) -> Result<Self, String> {
        Ok(Self {
            source: source.to_owned(),
        })
    }

    pub fn evaluate(&self, _facts: &dyn Facts) -> Result<f64, String> {
        Err(format!("APL expression not implemented: {}", self.source))
    }
}

#[cfg(test)]
mod tests;
