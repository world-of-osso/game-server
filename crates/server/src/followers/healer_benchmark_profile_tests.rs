//! Alternate benchmark actors must never silently use an unauthored healer policy.
use super::{Follower, FollowerCatalog, kit_tests::level32, simulator::spawn_party_with_healer};
use shared::components::Position;

#[test]
#[ignore = "RED: benchmark alternate specs have no explicit policy-admission boundary"]
fn healer_benchmark_alternate_specs_require_explicit_native_policy() {
    let (mut h, _) = level32(209072);
    let missing = [65, 256, 264, 270, 1468];
    let mut catalog = h.app.world_mut().resource_mut::<FollowerCatalog>();
    catalog.kits = catalog
        .kits
        .iter()
        .filter(|entry| !missing.contains(&entry.spec_id))
        .cloned()
        .collect::<Vec<_>>()
        .into();
    let roster: Vec<_> = catalog
        .profiles
        .iter()
        .map(|profile| {
            (
                profile.entry,
                profile.class_id,
                profile.spec_id,
                profile.role,
            )
        })
        .collect();
    let copy = *h.app.world().get::<crate::maps::InMap>(h.caster).unwrap();
    let position = *h.app.world().get::<Position>(h.caster).unwrap();
    let before = {
        let world = h.app.world_mut();
        world.query::<&Follower>().iter(world).count()
    };
    let errors: Vec<_> = missing
        .into_iter()
        .map(|spec| {
            spawn_party_with_healer(h.app.world_mut(), copy, position, 32, spec)
                .err()
                .expect("absent native decision policy must block before spawning")
        })
        .collect();
    assert_eq!(errors, vec!["NATIVE_HEALER_POLICY_ABSENT"; missing.len()]);
    assert_eq!(
        {
            let world = h.app.world_mut();
            world.query::<&Follower>().iter(world).count()
        },
        before,
        "policy refusal must not leave a partially spawned party"
    );
    let after: Vec<_> = h
        .app
        .world()
        .resource::<FollowerCatalog>()
        .profiles
        .iter()
        .map(|profile| {
            (
                profile.entry,
                profile.class_id,
                profile.spec_id,
                profile.role,
            )
        })
        .collect();
    assert_eq!(
        roster, after,
        "benchmark configuration must not rewrite the native roster"
    );
}
