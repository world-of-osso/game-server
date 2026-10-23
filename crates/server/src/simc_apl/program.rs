//! Source-ordered variables/calls, SimC76f8118 variable.cpp and action.cpp4648–4689.
use super::{Action, ActionFacts, Expression, Facts, Variables};
use std::collections::BTreeMap;

#[derive(Debug)]
enum Assignment {
    Set(Expression),
    SetIf {
        condition: Expression,
        value: Expression,
        otherwise: Expression,
    },
    Reset,
}

#[derive(Debug)]
enum Body {
    Action,
    Variable {
        name: String,
        assignment: Assignment,
    },
    Call(String),
}

#[derive(Debug)]
struct Rule {
    action: Action,
    condition: Option<Expression>,
    body: Body,
}

#[derive(Debug)]
pub struct Program {
    lists: BTreeMap<String, Vec<Rule>>,
    defaults: BTreeMap<String, f64>,
}

impl Program {
    /// An empty literal explicitly declares an empty list. An unknown call is an error.
    pub fn parse(lines: &[(&str, &str)]) -> Result<Self, String> {
        let mut program = Self {
            lists: BTreeMap::new(),
            defaults: BTreeMap::new(),
        };
        for &(list, literal) in lines {
            if list.is_empty() {
                return Err("APL list name is empty".into());
            }
            program.lists.entry(list.to_owned()).or_default();
            if literal.is_empty() {
                continue;
            }
            let rule = parse_rule(literal, &mut program.defaults)
                .map_err(|error| format!("APL list {list} row {literal:?}: {error}"))?;
            program
                .lists
                .get_mut(list)
                .expect("declared above")
                .push(rule);
        }
        program.validate_calls()?;
        Ok(program)
    }

    fn validate_calls(&self) -> Result<(), String> {
        for rules in self.lists.values() {
            for rule in rules {
                if let Body::Call(name) = &rule.body
                    && !self.lists.contains_key(name)
                {
                    return Err(format!("unknown APL call_action_list {name}"));
                }
            }
        }
        Ok(())
    }

    pub fn state(&self) -> Variables {
        Variables {
            values: self.defaults.clone(),
        }
    }

    pub fn select(
        &self,
        list: &str,
        facts: &dyn ActionFacts,
        variables: &mut Variables,
        mut usable: impl FnMut(&Action) -> Result<bool, String>,
    ) -> Result<Option<Action>, String> {
        let mut selection = Selection {
            facts,
            variables,
            usable: &mut usable,
            stack: Vec::new(),
        };
        self.select_list(list, &mut selection)
    }

    fn select_list(
        &self,
        list: &str,
        selection: &mut Selection<'_>,
    ) -> Result<Option<Action>, String> {
        if selection.stack.iter().any(|active| active == list) {
            return Err(format!(
                "recursive APL call_action_list {:?} -> {list}",
                selection.stack
            ));
        }
        let rules = self
            .lists
            .get(list)
            .ok_or_else(|| format!("unknown APL list {list}"))?;
        selection.stack.push(list.to_owned());
        let result = self.select_rules(rules, selection);
        selection.stack.pop();
        result
    }

    fn select_rules(
        &self,
        rules: &[Rule],
        selection: &mut Selection<'_>,
    ) -> Result<Option<Action>, String> {
        for rule in rules {
            let bound = BoundFacts {
                facts: selection.facts,
                variables: selection.variables,
                action: &rule.action,
            };
            if let Some(condition) = &rule.condition
                && condition.evaluate(&bound)? == 0.0
            {
                continue;
            }
            match &rule.body {
                Body::Variable { name, assignment } => {
                    self.assign(name, assignment, &rule.action, selection)?
                }
                Body::Call(name) => {
                    if let Some(action) = self.select_list(name, selection)? {
                        return Ok(Some(action));
                    }
                }
                Body::Action if (selection.usable)(&rule.action)? => {
                    return Ok(Some(rule.action.clone()));
                }
                Body::Action => {}
            }
        }
        Ok(None)
    }

    fn assign(
        &self,
        name: &str,
        assignment: &Assignment,
        action: &Action,
        selection: &mut Selection<'_>,
    ) -> Result<(), String> {
        let bound = BoundFacts {
            facts: selection.facts,
            variables: selection.variables,
            action,
        };
        let value = match assignment {
            Assignment::Set(expression) => expression.evaluate(&bound)?,
            Assignment::SetIf {
                condition,
                value,
                otherwise,
            } => {
                let selected = if condition.evaluate(&bound)? != 0.0 {
                    value
                } else {
                    otherwise
                };
                selected.evaluate(&bound)?
            }
            Assignment::Reset => *self.defaults.get(name).expect("declared variable default"),
        };
        let current = selection.variables.values.get_mut(name).ok_or_else(|| {
            format!("unknown APL operand variable.{name}; state must come from this program")
        })?;
        *current = value;
        Ok(())
    }
}

struct Selection<'a> {
    facts: &'a dyn ActionFacts,
    variables: &'a mut Variables,
    usable: &'a mut dyn FnMut(&Action) -> Result<bool, String>,
    stack: Vec<String>,
}

struct BoundFacts<'a> {
    facts: &'a dyn ActionFacts,
    variables: &'a Variables,
    action: &'a Action,
}

impl Facts for BoundFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if let Some(variable) = name.strip_prefix("variable.") {
            return self
                .variables
                .value(variable)
                .ok_or_else(|| format!("unknown APL operand {name}"));
        }
        self.facts.value_for(self.action, name)
    }
}

fn split_options(literal: &str) -> Result<(&str, BTreeMap<String, String>), String> {
    let mut segments = literal.split(',');
    let name = segments.next().expect("split always has one segment");
    if name.is_empty() {
        return Err("empty APL action name".into());
    }
    let mut options = BTreeMap::new();
    for segment in segments.filter(|segment| !segment.is_empty()) {
        let (key, value) = segment
            .split_once('=')
            .ok_or_else(|| format!("invalid APL option {segment:?}"))?;
        if key.is_empty() || value.is_empty() {
            return Err(format!("empty APL option {segment:?}"));
        }
        if options.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(format!("duplicate APL option {key}"));
        }
    }
    Ok((name, options))
}

fn required_option(options: &BTreeMap<String, String>, key: &str) -> Result<String, String> {
    options
        .get(key)
        .cloned()
        .ok_or_else(|| format!("missing APL option {key}"))
}

fn option_expression(options: &BTreeMap<String, String>, key: &str) -> Result<Expression, String> {
    Expression::parse(&required_option(options, key)?)
}

fn parse_rule(literal: &str, defaults: &mut BTreeMap<String, f64>) -> Result<Rule, String> {
    let (name, mut options) = split_options(literal)?;
    let condition = options
        .remove("if")
        .map(|source| Expression::parse(&source))
        .transpose()?;
    let action = Action {
        name: name.to_owned(),
        options,
    };
    let body = match name {
        "variable" => parse_variable(&action.options, defaults)?,
        "call_action_list" => {
            if action.options.keys().any(|key| key != "name") {
                return Err("unsupported call_action_list option".into());
            }
            Body::Call(required_option(&action.options, "name")?)
        }
        _ => Body::Action,
    };
    Ok(Rule {
        action,
        condition,
        body,
    })
}

fn parse_variable(
    options: &BTreeMap<String, String>,
    defaults: &mut BTreeMap<String, f64>,
) -> Result<Body, String> {
    const OPTIONS: &[&str] = &["name", "op", "value", "condition", "value_else", "default"];
    if let Some(key) = options.keys().find(|key| !OPTIONS.contains(&key.as_str())) {
        return Err(format!("unsupported APL variable option {key}"));
    }
    let name = required_option(options, "name")?.to_ascii_lowercase();
    let default = options
        .get("default")
        .map(|value| value.parse::<f64>())
        .transpose()
        .map_err(|error| format!("invalid APL variable default: {error}"))?
        .unwrap_or(0.0);
    // variable.cpp118–133: first declaration owns the default, initially zero.
    defaults.entry(name.clone()).or_insert(default);
    let assignment = parse_assignment(options)?;
    Ok(Body::Variable { name, assignment })
}

fn parse_assignment(options: &BTreeMap<String, String>) -> Result<Assignment, String> {
    let operation = options.get("op").map_or("set", String::as_str);
    match operation {
        "set" => Ok(Assignment::Set(option_expression(options, "value")?)),
        "setif" => Ok(Assignment::SetIf {
            condition: option_expression(options, "condition")?,
            value: option_expression(options, "value")?,
            otherwise: option_expression(options, "value_else")?,
        }),
        "reset" => Ok(Assignment::Reset),
        _ => Err(format!("unsupported APL variable operation {operation}")),
    }
}
