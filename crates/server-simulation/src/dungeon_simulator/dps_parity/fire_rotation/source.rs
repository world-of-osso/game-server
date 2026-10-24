//! Pinned rows are data; the shared Program owns list/variable/option semantics.
use server::simc_apl::Program;

pub(super) fn load() -> Result<Program, String> {
    let rows = include_str!("76f8118.simc")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(parse_line)
        .collect::<Result<Vec<_>, _>>()?;
    Program::parse(&rows)
}

fn parse_line(line: &str) -> Result<(&str, &str), String> {
    let (head, literal) = line.split_once('=').ok_or("Fire source lacks assignment")?;
    let head = head.strip_suffix('+').unwrap_or(head);
    let list = if head == "actions" {
        "default"
    } else {
        head.strip_prefix("actions.")
            .ok_or("Fire source is not an action list")?
    };
    Ok((list, literal.strip_prefix('/').unwrap_or(literal)))
}
