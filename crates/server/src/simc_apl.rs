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

#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    pub name: String,
    pub options: BTreeMap<String, String>,
}

#[derive(Default)]
pub struct Variables {
    values: BTreeMap<String, f64>,
}

impl Variables {
    pub fn value(&self, name: &str) -> Option<f64> {
        self.values.get(name).copied()
    }
}

pub struct Program;

impl Program {
    pub fn parse(_lines: &[(&str, &str)]) -> Result<Self, String> {
        Err("APL program not implemented".into())
    }

    pub fn state(&self) -> Variables {
        Variables::default()
    }

    pub fn select(
        &self,
        _list: &str,
        _facts: &dyn Facts,
        _variables: &mut Variables,
        _usable: impl FnMut(&Action) -> Result<bool, String>,
    ) -> Result<Option<Action>, String> {
        Err("APL selection not implemented".into())
    }
}

#[cfg(test)]
mod program_tests;
#[cfg(test)]
mod tests;
