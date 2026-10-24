//! Literal lists/targets: SimC76f8118 variable.cpp and action.cpp4648–4763,5362–5453.
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
    Run(String),
}

#[derive(Clone, Copy, Debug)]
enum TargetMode {
    First,
    Min,
    Max,
}

#[derive(Debug)]
struct TargetSelector {
    mode: TargetMode,
    expression: Expression,
}

#[derive(Debug)]
struct Rule {
    action: Action,
    condition: Option<Expression>,
    target: Option<TargetSelector>,
    cycle_limit: Option<usize>,
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
                if let Body::Call(name) | Body::Run(name) = &rule.body
                    && !self.lists.contains_key(name)
                {
                    return Err(format!("unknown APL {} {name}", rule.action.name));
                }
            }
        }
        Ok(())
    }

    /// Source declarations for native adapters whose policy depends on other rows.
    pub fn actions(&self) -> impl Iterator<Item = (&str, &Action)> {
        self.lists
            .iter()
            .flat_map(|(list, rules)| rules.iter().map(move |rule| (list.as_str(), &rule.action)))
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

    /// Consume top-level rows once; combat selection remains a priority scan.
    pub fn select_next(
        &self,
        list: &str,
        cursor: &mut usize,
        facts: &dyn ActionFacts,
        variables: &mut Variables,
        mut usable: impl FnMut(&Action) -> Result<bool, String>,
    ) -> Result<Option<Action>, String> {
        let rules = self
            .lists
            .get(list)
            .ok_or_else(|| format!("unknown APL list {list}"))?;
        let mut selection = Selection {
            facts,
            variables,
            usable: &mut usable,
            stack: vec![list.to_owned()],
        };
        while let Some(rule) = rules.get(*cursor) {
            if matches!(&rule.body, Body::Call(_) | Body::Run(_)) {
                return Err(format!(
                    "ordered precombat does not support nested {}",
                    rule.action.name
                ));
            }
            *cursor += 1;
            let action = self.select_rules(std::slice::from_ref(rule), &mut selection)?;
            if action.is_some() {
                return Ok(action);
            }
        }
        Ok(None)
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
            let Some(action) = bind_ready_target(rule, selection)? else {
                continue;
            };
            match &rule.body {
                Body::Variable { name, assignment } => {
                    self.assign(name, assignment, &action, selection)?
                }
                Body::Call(name) => {
                    if let Some(action) = self.select_list(name, selection)? {
                        return Ok(Some(action));
                    }
                }
                // action.cpp4749–4763 switches lists without resuming this caller.
                // player.cpp2467–2476 changes no child targets: the run's selector
                // belongs only to that control row, not its child actions.
                Body::Run(name) => return self.select_list(name, selection),
                Body::Action => return Ok(Some(action)),
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

fn bind_ready_target(rule: &Rule, selection: &mut Selection<'_>) -> Result<Option<Action>, String> {
    // action.cpp2435–2508: target_if takes precedence over cycle_targets.
    if let Some(limit) = rule.cycle_limit
        && rule.target.is_none()
        && selection.facts.value_for(&rule.action, "active_enemies")? > 1.0
    {
        return bind_cycle_target(rule, selection, limit);
    }
    let Some(action) = bind_target(rule, selection)? else {
        return Ok(None);
    };
    Ok(rule_ready(rule, &action, selection)?.then_some(action))
}

fn bind_cycle_target(
    rule: &Rule,
    selection: &mut Selection<'_>,
    limit: usize,
) -> Result<Option<Action>, String> {
    let mut action = rule.action.clone();
    let candidates = selection.facts.target_ids(&action)?;
    let limit = if limit > 0 { limit } else { candidates.len() };
    for target in candidates.into_iter().take(limit) {
        action.target = Some(target);
        if rule_ready(rule, &action, selection)? {
            return Ok(Some(action));
        }
    }
    Ok(None)
}

fn rule_ready(rule: &Rule, action: &Action, selection: &mut Selection<'_>) -> Result<bool, String> {
    if !condition_passes(rule, action, selection)? {
        return Ok(false);
    }
    if matches!(&rule.body, Body::Action) {
        return (selection.usable)(action);
    }
    Ok(true)
}

fn condition_passes(
    rule: &Rule,
    action: &Action,
    selection: &Selection<'_>,
) -> Result<bool, String> {
    let Some(condition) = &rule.condition else {
        return Ok(true);
    };
    evaluate_for(condition, action, selection).map(|value| value != 0.0)
}

fn evaluate_for(
    expression: &Expression,
    action: &Action,
    selection: &Selection<'_>,
) -> Result<f64, String> {
    expression.evaluate(&BoundFacts {
        facts: selection.facts,
        variables: selection.variables,
        action,
    })
}

/// action.cpp2435–2468,2619: target_if chooses before the ordinary if expression.
/// Candidate order/eligibility belongs to native adapters, never the interpreter.
fn bind_target(rule: &Rule, selection: &Selection<'_>) -> Result<Option<Action>, String> {
    let mut action = rule.action.clone();
    let Some(selector) = &rule.target else {
        return Ok(Some(action));
    };
    let target = choose_target(selector, &mut action, selection)?;
    Ok(target.map(|target| {
        action.target = Some(target);
        action
    }))
}

fn choose_target(
    selector: &TargetSelector,
    action: &mut Action,
    selection: &Selection<'_>,
) -> Result<Option<u64>, String> {
    let candidates = selection.facts.target_ids(action)?;
    let single = candidates.len() == 1;
    let mut best_value = match selector.mode {
        TargetMode::First => 0.0,
        TargetMode::Min => f64::INFINITY,
        TargetMode::Max => f64::NEG_INFINITY,
    };
    let mut best_target = None;
    for target in candidates {
        action.target = Some(target);
        let value = evaluate_for(&selector.expression, action, selection)?;
        if !target_preferred(selector.mode, single, value, best_value) {
            continue;
        }
        if matches!(selector.mode, TargetMode::First) {
            return Ok(Some(target));
        }
        best_value = value;
        best_target = Some(target);
    }
    Ok(best_target)
}

fn target_preferred(mode: TargetMode, single: bool, value: f64, best: f64) -> bool {
    match mode {
        // Pinned action.cpp5367–5379 versus5417–5426; actual binary probes agree.
        TargetMode::First if single => value > 0.0,
        TargetMode::First => value != 0.0,
        // Single min/max ignores score; evaluation still rejects unknown facts.
        TargetMode::Min => single || value < best,
        TargetMode::Max => single || value > best,
    }
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
        target: None,
        options,
    };
    let target = action
        .options
        .get("target_if")
        .map(|literal| parse_target_selector(literal))
        .transpose()?;
    let body = match name {
        "variable" => parse_variable(&action.options, defaults)?,
        "call_action_list" => {
            if action.options.keys().any(|key| key != "name") {
                return Err("unsupported call_action_list option".into());
            }
            Body::Call(required_option(&action.options, "name")?)
        }
        "run_action_list" => {
            if action
                .options
                .keys()
                .any(|key| !matches!(key.as_str(), "name" | "target_if"))
            {
                return Err("unsupported run_action_list option".into());
            }
            Body::Run(required_option(&action.options, "name")?)
        }
        _ => Body::Action,
    };
    let cycle_limit = parse_cycle_limit(&action.options)?;
    Ok(Rule {
        action,
        condition,
        target,
        cycle_limit,
        body,
    })
}

fn parse_target_selector(literal: &str) -> Result<TargetSelector, String> {
    let (mode, expression) = match literal.split_once(':') {
        None => (TargetMode::First, literal),
        Some(("first", expression)) => (TargetMode::First, expression),
        Some(("min", expression)) => (TargetMode::Min, expression),
        Some(("max", expression)) => (TargetMode::Max, expression),
        Some((mode, _)) => return Err(format!("unsupported target_if mode {mode}")),
    };
    Ok(TargetSelector {
        mode,
        expression: Expression::parse(expression)?,
    })
}

fn parse_cycle_limit(options: &BTreeMap<String, String>) -> Result<Option<usize>, String> {
    let enabled = match options.get("cycle_targets").map(String::as_str) {
        None | Some("0") => false,
        Some("1") => true,
        Some(value) => {
            return Err(format!(
                "invalid cycle_targets boolean {value:?}; expected0/1"
            ));
        }
    };
    let limit = options
        .get("max_cycle_targets")
        .map(|value| value.parse::<i32>())
        .transpose()
        .map_err(|error| format!("invalid max_cycle_targets integer: {error}"))?
        .unwrap_or(0);
    Ok(enabled.then_some(limit.max(0) as usize))
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
