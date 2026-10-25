//! SimC76f8118 player.cpp10644–10818: readonly next-action pooling policy.
use super::*;

pub(super) fn parse(action: &Action) -> Result<Body, String> {
    if action
        .options
        .keys()
        .any(|key| !matches!(key.as_str(), "for_next" | "resource"))
    {
        return Err(
            "unsupported pool_resource option; qualified surface is for_next=1 and resource".into(),
        );
    }
    if action.options.get("for_next").map(String::as_str) != Some("1") {
        return Err("pool_resource requires qualified for_next=1".into());
    }
    Ok(Body::PoolNext {
        resource: action.options.get("resource").cloned(),
    })
}

pub(super) fn validate_next_actions(lists: &BTreeMap<String, Vec<Rule>>) -> Result<(), String> {
    for (list, rules) in lists {
        for (index, rule) in rules.iter().enumerate() {
            if matches!(rule.body, Body::PoolNext { .. })
                && !rules
                    .get(index + 1)
                    .is_some_and(|next| matches!(next.body, Body::Action))
            {
                return Err(format!(
                    "pool_resource in {list} requires a following ordinary action"
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn options_ready(action: &Action, selection: &Selection<'_>) -> Result<bool, String> {
    match action.options.get("max_energy").map(String::as_str) {
        None | Some("0") => Ok(true),
        Some("1") => {
            let current = selection.facts.value_for(action, "energy")?;
            let maximum_energy = selection.facts.value_for(action, "maximum_energy")?;
            Ok(current >= maximum_energy)
        }
        Some(value) => Err(format!("unsupported max_energy={value}; expected0|1")),
    }
}

pub(super) fn should_wait(
    next: &Rule,
    selection: &mut Selection<'_>,
    resource: Option<&str>,
) -> Result<bool, String> {
    if bind_ready_target(next, selection)?.is_some() {
        return Ok(false);
    }
    let resource = match resource {
        Some(resource) => resource.to_owned(),
        None => selection.facts.primary_resource()?,
    };
    let facts = MaximumFacts {
        native: selection.facts,
        resource: &resource,
    };
    let mut admitted = |action: &Action| {
        Ok(selection.facts.target_ready(action)?
            && selection.facts.ready_at_resource_max(action, &resource)?)
    };
    let mut maximum = Selection {
        facts: &facts,
        variables: &mut *selection.variables,
        usable: &mut admitted,
        stack: Vec::new(),
        pooling: false,
        stop_before: None,
    };
    Ok(bind_ready_target(next, &mut maximum)?.is_some())
}

struct MaximumFacts<'a> {
    native: &'a dyn ActionFacts,
    resource: &'a str,
}

impl MaximumFacts<'_> {
    fn resource_value(&self, name: &str, action: Option<&Action>) -> Option<Result<f64, String>> {
        let maximum = || match action {
            Some(action) => self
                .native
                .value_for(action, &format!("{}.max", self.resource)),
            None => self.native.value(&format!("{}.max", self.resource)),
        };
        if name == self.resource {
            return Some(maximum());
        }
        let field = name.strip_prefix(self.resource)?.strip_prefix('.')?;
        Some(match field {
            "current" | "max" => maximum(),
            "deficit" | "time_to_max" => Ok(0.0),
            _ => Err(format!("unqualified maximum-resource operand {name}")),
        })
    }
}

impl Facts for MaximumFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        self.resource_value(name, None)
            .unwrap_or_else(|| self.native.value(name))
    }
}

impl ActionFacts for MaximumFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        self.resource_value(name, Some(action))
            .unwrap_or_else(|| self.native.value_for(action, name))
    }

    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }

    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.native.target_ready(action)
    }
}
