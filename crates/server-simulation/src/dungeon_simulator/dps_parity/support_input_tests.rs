//! Generated MID2 support profiles must load and spawn through native catalogs.
use super::*;

fn check_support_profile(spec: u32) -> Result<(), String> {
    let (profile, traits) = read_reference(spec)?;
    if profile.spec_id != spec || traits.is_empty() {
        return Err("wrong spec or empty talent allocation".into());
    }
    let identity = inputs::read_actor(&profile)?;
    let run = spawn_reference(profile, &traits, 1, identity)?;
    if run
        .app
        .world()
        .get::<shared::components::ActiveSpec>(run.actor)
        .map(|spec| spec.0)
        != Some(spec)
    {
        return Err("spawned actor has wrong spec".into());
    }
    Ok(())
}

#[test]
#[ignore = "requires generated MID2 support inputs and canonical world DB"]
fn mid2_support_inputs_load_and_spawn_all_eight_specs() {
    let failures: Vec<_> = [65, 104, 105, 256, 257, 264, 270, 1468]
        .into_iter()
        .filter_map(|spec| check_support_profile(spec).err().map(|error| (spec, error)))
        .collect();
    assert!(
        failures.is_empty(),
        "support profile failures: {failures:?}"
    );
}
