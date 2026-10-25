//! Compile the complete preserved APL asset; never curate a replacement priority list.
use server::simc_apl::Program;
use std::collections::BTreeMap;

type SourceLists = BTreeMap<String, Vec<(usize, String)>>;

pub(super) fn load() -> Result<Program, String> {
    compile_source(include_str!("76f8118.simc"))
}

fn compile_source(source: &str) -> Result<Program, String> {
    let mut lists = SourceLists::new();
    for (index, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        append_source_row(&mut lists, line, index)
            .map_err(|error| format!("Dev APL source line{}: {error}", index + 1))?;
    }
    let mut rows: Vec<_> = lists
        .iter()
        .flat_map(|(name, rows)| {
            rows.iter()
                .map(move |(position, row)| (*position, name.as_str(), row.as_str()))
        })
        .collect();
    // Program's first declaration owns a variable's default, even across lists.
    rows.sort_by_key(|(position, _, _)| *position);
    let rows: Vec<_> = rows.into_iter().map(|(_, name, row)| (name, row)).collect();
    Program::parse(&rows)
}

fn append_source_row(lists: &mut SourceLists, line: &str, position: usize) -> Result<(), String> {
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
    }
    body.push((position, row.strip_prefix('/').unwrap_or(row).into()));
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
        assert!(
            error.contains("unknown APL operand trinket.1.has_buff."),
            "{error}"
        );
    }

    #[test]
    fn literal_dev_source_preserves_first_declaration_default_across_lists() {
        let program = compile_source(
            "actions.precombat=variable,name=gate,default=1,value=2\n\
             actions=variable,name=gate,default=0,value=3\n\
             actions.inspect=ready,if=variable.gate=1\n",
        )
        .unwrap();
        let facts = BTreeMap::<String, f64>::new();
        let selected = program
            .select("inspect", &facts, &mut program.state(), |_| Ok(true))
            .unwrap()
            .expect("first source declaration must own the initial variable default");
        assert_eq!(selected.name, "ready");
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
