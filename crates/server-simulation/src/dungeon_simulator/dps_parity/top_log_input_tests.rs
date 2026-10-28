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
            let equipped_stats = report::stats(run.app.world(), run.actor);
            let (mut bare_profile, bare_traits) = read_reference(spec)?;
            bare_profile.equipment.clear();
            let identity = inputs::read_actor(&bare_profile)?;
            let mut bare = spawn_reference(bare_profile, &bare_traits, 1, identity)?;
            bare.app.update();
            let bare_stats = report::stats(bare.app.world(), bare.actor);
            println!("spec {spec}: equipped {equipped_stats}; bare {bare_stats}");
            let equipped_stamina = equipped_stats["attribute"]["stamina"].as_f64().unwrap();
            let bare_stamina = bare_stats["attribute"]["stamina"].as_f64().unwrap();
            if equipped_stamina <= bare_stamina {
                return Err("equipped items did not increase stamina".into());
            }
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
