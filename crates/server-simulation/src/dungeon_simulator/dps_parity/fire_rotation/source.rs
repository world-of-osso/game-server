//! Preserve exact exported source rows; the shared Program owns their semantics.
use server::simc_apl::Program;

pub(super) fn load(profile: &super::super::inputs::Profile) -> Result<Program, String> {
    // SimC76f8118 engine/class_modules/apl/mage.cpp148–250: unchanged Fire lists.
    let source = profile
        .options
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .ok_or("Fire profile lacks literal action rows")?;
    let rows = source
        .iter()
        .map(|row| {
            let line = row.as_str().ok_or("Fire action row must be a string")?;
            let (head, literal) = line.split_once('=').ok_or("Fire source lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("Fire source is not an action list")?
            };
            Ok((list, literal.strip_prefix('/').unwrap_or(literal)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Program::parse(&rows)
}
