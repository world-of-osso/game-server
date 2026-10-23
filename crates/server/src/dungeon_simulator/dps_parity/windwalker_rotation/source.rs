//! Literal generated profile data; the shared evaluator owns expressions/lists.
use super::bindings::ACTIONS;
use server::simc_apl::{Expression, Program};

pub(super) fn load() -> Result<Program, String> {
    let rows = include_str!("76f8118.simc")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(parse_line)
        .collect::<Result<Vec<_>, _>>()?;
    Program::parse(&rows)
}

fn parse_line(line: &str) -> Result<(&str, &str), String> {
    let (head, literal) = line
        .split_once('=')
        .ok_or("WW source row lacks assignment")?;
    let head = head.strip_suffix('+').unwrap_or(head);
    let list = if head == "actions" {
        "default"
    } else {
        head.strip_prefix("actions.")
            .ok_or("WW source row is not an action list")?
    };
    let literal = literal.strip_prefix('/').unwrap_or(literal);
    validate_options(literal)?;
    Ok((list, literal))
}

fn validate_options(literal: &str) -> Result<(), String> {
    let mut parts = literal.split(',');
    let action = parts.next().ok_or("WW source action empty")?;
    let control = matches!(action, "variable" | "call_action_list");
    if !control
        && !matches!(
            action,
            "auto_attack" | "snapshot_stats" | "use_item" | "invoke_external_buff"
        )
        && !ACTIONS.iter().any(|(name, _)| *name == action)
    {
        return Err(format!("unknown literal WW action {action}"));
    }
    for option in parts.filter(|value| !value.is_empty()) {
        let (name, value) = option
            .split_once('=')
            .ok_or_else(|| format!("invalid WW option {option}"))?;
        if control {
            continue;
        } // Shared Program validates variable/call options.
        match name {
            "if" => {}
            "target_if" if matches!(value, "min:target.time_to_die" | "max:target.time_to_die") => {
            }
            "cancel_if" => {
                Expression::parse(value)?;
            }
            "slot"
                if action == "use_item"
                    && matches!(value, "main_hand" | "trinket1" | "trinket2") => {}
            "name" if action == "use_item" && value == "algethar_puzzle_box" => {}
            "name" if action == "invoke_external_buff" && value == "power_infusion" => {}
            _ => return Err(format!("unsupported literal WW option {action}:{option}")),
        }
    }
    Ok(())
}
