//! Top-ranked healer and Guardian snapshots must pass native item and trait admission.
use super::*;

#[test]
#[ignore = "requires top-character profile snapshots and canonical world database"]
fn top_log_support_inputs_spawn_and_grant_natures_cure() {
    let mut failures = Vec::new();
    for spec in [65, 104, 105, 256, 257, 264, 270, 1468] {
        let result = (|| -> Result<(), String> {
            let (profile, traits) = read_reference(spec)?;
            let identity = inputs::read_actor(&profile)?;
            let mut run = spawn_reference(profile, &traits, 1, identity)?;
            // Run native equipment synchronization, including on-use enchant 4897.
            run.app.update();
            if spec == 105 {
                let world = run.app.world();
                let known = world
                    .get::<crate::class_progression::KnownSpells>(run.actor)
                    .ok_or("known spells absent")?;
                if !known.contains(88423) {
                    return Err("Restoration does not know Nature's Cure 88423".into());
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            failures.push((spec, error));
        }
    }
    assert!(
        failures.is_empty(),
        "top-character native input failures: {failures:?}"
    );
}
