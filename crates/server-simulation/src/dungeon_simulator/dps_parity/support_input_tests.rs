//! Generated MID2 support profiles must load and spawn through native catalogs.
use super::*;

#[test]
#[ignore = "requires generated MID2 support inputs and canonical world DB"]
fn mid2_support_inputs_load_and_spawn_all_eight_specs() {
    for spec in [65, 104, 105, 256, 257, 264, 270, 1468] {
        let (profile, traits) =
            read_reference(spec).unwrap_or_else(|error| panic!("spec {spec}: {error}"));
        assert_eq!(profile.spec_id, spec);
        assert!(
            !traits.is_empty(),
            "spec {spec} must retain sourced talents"
        );
        let identity = inputs::read_actor(&profile).unwrap();
        let run = spawn_reference(profile, &traits, 1, identity)
            .unwrap_or_else(|error| panic!("spec {spec}: {error}"));
        assert_eq!(
            run.app
                .world()
                .get::<shared::components::ActiveSpec>(run.actor)
                .unwrap()
                .0,
            spec
        );
    }
}
