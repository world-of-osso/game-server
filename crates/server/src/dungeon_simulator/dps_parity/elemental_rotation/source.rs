//! Exact generated76f8118 Elemental rows; shared evaluator owns expressions/lists.
use super::bindings::ACTIONS;
use server::simc_apl::Program;

pub(super) fn load() -> Result<Program, String> {
    let rows = include_str!("76f8118.simc")
        .lines()
        .map(parse_line)
        .collect::<Result<Vec<_>, _>>()?;
    Program::parse(&rows)
}

fn parse_line(line: &str) -> Result<(&str, &str), String> {
    let (head, literal) = line
        .split_once('=')
        .ok_or("Elemental source row lacks assignment")?;
    let head = head.strip_suffix('+').unwrap_or(head);
    let list = if head == "actions" {
        "default"
    } else {
        head.strip_prefix("actions.")
            .ok_or("Elemental source row is not an action list")?
    };
    let literal = literal.strip_prefix('/').unwrap_or(literal);
    validate_options(literal)?;
    Ok((list, literal))
}

fn validate_options(literal: &str) -> Result<(), String> {
    let mut parts = literal.split(',');
    let action = parts.next().ok_or("Elemental source action empty")?;
    let control = matches!(action, "variable" | "run_action_list");
    if !control
        && !matches!(
            action,
            "snapshot_stats" | "use_item" | "invoke_external_buff"
        )
        && !ACTIONS.iter().any(|(name, _)| *name == action)
    {
        return Err(format!("unknown literal Elemental action {action}"));
    }
    for option in parts {
        let (name, value) = option
            .split_once('=')
            .ok_or_else(|| format!("invalid Elemental option {option}"))?;
        if control {
            continue; // Shared Program validates variable/list options.
        }
        match name {
            "if" => {}
            "moving" | "use_off_gcd" if value == "1" => {}
            "target_if"
                if matches!(
                    value,
                    "min:dot.flame_shock.remains" | "min:debuff.lightning_rod.remains"
                ) => {}
            "slot"
                if action == "use_item"
                    && matches!(value, "main_hand" | "trinket1" | "trinket2") => {}
            "name"
                if action == "use_item"
                    && matches!(
                        value,
                        "stormbound_emblem_of_dazar"
                            | "hex_lords_dooming_idol"
                            | "font_of_venomous_rage"
                    ) => {}
            "name" if action == "invoke_external_buff" && value == "power_infusion" => {}
            _ => {
                return Err(format!(
                    "unsupported literal Elemental option {action}:{option}"
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_ele_source_rejects_unknown_inactive_actions_and_options() {
        assert!(parse_line("actions=unknown_elemental_action,if=0").is_err());
        assert!(parse_line("actions=lightning_bolt,unsupported=1").is_err());
        assert!(parse_line("actions=elemental_blast,target_if=min:dot.unknown.remains").is_err());
        assert!(load().is_ok());
    }
}
