//! Exact74-row prepared SimC76f8118 profile; shared Program owns all interpretation.
use super::bindings::ACTIONS;
use server::simc_apl::Program;

pub(super) fn load() -> Result<Program, String> {
    let rows = include_str!("76f8118.simc")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(parse_row)
        .collect::<Result<Vec<_>, _>>()?;
    Program::parse(&rows)
}

fn parse_row(line: &str) -> Result<(&str, &str), String> {
    let (head, literal) = line
        .split_once('=')
        .ok_or("Feral source row lacks assignment")?;
    let head = head.strip_suffix('+').unwrap_or(head);
    let list = if head == "actions" {
        "default"
    } else {
        head.strip_prefix("actions.")
            .ok_or("Feral source row is not an action list")?
    };
    let literal = literal.strip_prefix('/').unwrap_or(literal);
    validate_options(literal)?;
    Ok((list, literal))
}

fn validate_options(literal: &str) -> Result<(), String> {
    let mut parts = literal.split(',');
    let action = parts.next().ok_or("Feral source action is empty")?;
    if matches!(action, "variable" | "call_action_list" | "pool_resource") {
        // These directives are validated and interpreted by shared Program.
        return Ok(());
    }
    if !ACTIONS.iter().any(|(name, _)| *name == action)
        && !matches!(
            action,
            "berserk"
                | "auto_attack"
                | "snapshot_stats"
                | "use_item"
                | "use_items"
                | "invoke_external_buff"
        )
    {
        return Err(format!("unknown literal Feral action {action}"));
    }
    for option in parts {
        let (name, value) = option
            .split_once('=')
            .ok_or_else(|| format!("invalid Feral option {option}"))?;
        if !supports_option(action, name, value) {
            return Err(format!(
                "unsupported literal Feral option {action}:{option}"
            ));
        }
    }
    Ok(())
}

fn supports_option(action: &str, name: &str, value: &str) -> bool {
    matches!(
        (action, name, value),
        (_, "if" | "target_if", _)
            | ("use_item", "slot", "trinket1" | "trinket2")
            | ("use_item", "name", "algethar_puzzle_box")
            | ("use_item", "use_off_gcd", "1")
            | ("invoke_external_buff", "name", "power_infusion")
            | ("potion", "pre_pot_time", "12")
            | ("ferocious_bite", "max_energy", "1")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn facts(rows: &[(&str, f64)]) -> BTreeMap<String, f64> {
        rows.iter()
            .map(|&(name, value)| (name.into(), value))
            .collect()
    }

    #[test]
    fn feral_literal_source_finisher_selects_rip_before_pooling() {
        let program = load().expect("entire exact profile must parse through shared Program");
        let facts = facts(&[
            ("combo_points", 5.0),
            ("refreshable", 1.0),
            ("buff.tigers_fury.up", 1.0),
            ("remains", 4.0),
            ("cooldown.tigers_fury.remains", 10.0),
        ]);
        let action = program
            .select("finisher", &facts, &mut program.state(), |_| Ok(true))
            .unwrap()
            .unwrap();
        assert_eq!(action.name, "rip");
    }

    #[test]
    fn feral_literal_source_builder_upgrades_rake_from_real_row_order() {
        let program = load().expect("entire exact profile must parse through shared Program");
        let facts = facts(&[
            ("buff.shadowmeld.up", 0.0),
            ("buff.prowl.up", 0.0),
            ("action.rake.pmultiplier", 1.6),
            ("dot.rake.refreshable", 0.0),
            ("buff.sudden_ambush.up", 0.0),
            ("hero_tree.druid_of_the_claw", 0.0),
            ("buff.tigers_fury.up", 1.0),
            ("cooldown.tigers_fury.remains", 10.0),
            ("remains", 4.0),
            ("refreshable", 1.0),
            ("persistent_multiplier", 1.6),
            ("pmultiplier", 1.0),
        ]);
        let action = program
            .select("builder", &facts, &mut program.state(), |_| Ok(true))
            .unwrap()
            .unwrap();
        assert_eq!(action.name, "rake");
    }
}
