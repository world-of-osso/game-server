//! Pinned Retribution action lists, shared by parity and live actors.
use super::Program;

pub fn load() -> Result<Program, String> {
    let rows = include_str!("ret_76f8118.simc")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(parse_row)
        .collect::<Result<Vec<_>, _>>()?;
    Program::parse(&rows)
}

fn parse_row(line: &str) -> Result<(&str, &str), String> {
    let (head, literal) = line
        .split_once("+=")
        .ok_or("Ret source lacks += assignment")?;
    let list = if head == "actions" {
        "default"
    } else {
        head.strip_prefix("actions.")
            .ok_or("Ret source is not an action list")?
    };
    let literal = literal
        .strip_prefix('/')
        .ok_or("Ret source action lacks slash")?;
    Ok((list, literal))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn class_follower_ret_program_selects_native_precombat_snapshot() {
        let program = load().unwrap();
        let mut state = program.state();
        let facts = BTreeMap::<String, f64>::new();
        let action = program
            .select("precombat", &facts, &mut state, |_| Ok(true))
            .unwrap()
            .unwrap();
        assert_eq!(action.name, "snapshot_stats");
    }
}
