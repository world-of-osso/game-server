//! Compile the complete preserved APL asset; never curate a replacement priority list.
use server::simc_apl::Program;
use std::collections::BTreeMap;

pub(super) fn load() -> Result<Program, String> {
    compile_source(include_str!("76f8118.simc"))
}

fn compile_source(source: &str) -> Result<Program, String> {
    let mut lists = BTreeMap::<String, String>::new();
    for (index, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        append_source_row(&mut lists, line)
            .map_err(|error| format!("Dev APL source line{}: {error}", index + 1))?;
    }
    let lists: Vec<_> = lists
        .iter()
        .map(|(name, body)| (name.as_str(), body.as_str()))
        .collect();
    Program::parse(&lists)
}

fn append_source_row(lists: &mut BTreeMap<String, String>, line: &str) -> Result<(), String> {
    let (assignment, row) = line.split_once('=').ok_or("action assignment absent")?;
    let append = assignment.ends_with('+');
    let key = assignment.strip_suffix('+').unwrap_or(assignment);
    let name = if key == "actions" {
        "default"
    } else {
        key.strip_prefix("actions.")
            .filter(|name| !name.is_empty())
            .ok_or_else(|| format!("unsupported action assignment {key}"))?
    };
    let body = lists.entry(name.into()).or_default();
    if !append {
        body.clear();
    } else if !body.is_empty() {
        body.push('/');
    }
    body.push_str(row.strip_prefix('/').unwrap_or(row));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_dev_source_compiles_all_lists_without_hiding_missing_native_facts() {
        let program = load().unwrap();
        let mut state = program.state();
        let facts = BTreeMap::<String, f64>::new();
        let snapshot = program
            .select("precombat", &facts, &mut state, |action| {
                Ok(action.name == "snapshot_stats")
            })
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.name, "snapshot_stats");
        let error = program
            .select("precombat", &facts, &mut state, |_| Ok(false))
            .unwrap_err();
        assert!(error.contains("trinket.1.has_buff.intellect"), "{error}");
    }

    #[test]
    fn literal_dev_source_assignment_replaces_and_append_preserves_action_order() {
        let program =
            compile_source("actions=azure_strike\nactions=dragonrage\nactions+=/living_flame\n")
                .unwrap();
        let facts = BTreeMap::<String, f64>::new();
        let mut state = program.state();
        let first = program
            .select("default", &facts, &mut state, |_| Ok(true))
            .unwrap()
            .unwrap();
        assert_eq!(first.name, "dragonrage");
        let next = program
            .select("default", &facts, &mut state, |action| {
                Ok(action.name == "living_flame")
            })
            .unwrap()
            .unwrap();
        assert_eq!(next.name, "living_flame");
    }
}
