use serde_json::Value;

/// Attach the observed errors before rejecting a report so failure output retains them.
pub(super) fn attach_and_check(report: &mut Value, errors: Value) -> Result<(), String> {
    let is_empty = errors
        .as_array()
        .ok_or("fact error snapshot is not an array")?
        .is_empty();
    report["fact_errors"] = errors;
    if is_empty {
        Ok(())
    } else {
        Err(format!("APL fact errors in report: {report}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_snapshot_passes_and_appears_in_report() {
        let mut report = json!({"spec": 66});
        attach_and_check(&mut report, json!([])).unwrap();
        assert_eq!(report["fact_errors"], json!([]));
    }

    #[test]
    fn seeded_snapshot_is_reported_and_fails() {
        let entry = json!({"fact":"target.health","actor":12,"target":34,"detail":"missing health","count":1});
        let mut report = json!({"spec": 66});
        let failure = attach_and_check(&mut report, json!([entry])).unwrap_err();
        assert_eq!(report["fact_errors"][0], entry);
        assert!(failure.contains("target.health"));
        assert!(failure.contains("missing health"));
    }
}
