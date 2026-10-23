//! Alternate benchmark actors must never silently use an unauthored healer policy.
use super::{Follower, FollowerCatalog, kit_tests::level32, simulator::spawn_party_with_healer};
use shared::components::Position;

#[test]
#[ignore = "explicit alternate benchmark actor/policy admission"]
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

#[test]
#[ignore = "explicit source actor identity and requested native class/spec configuration"]
fn healer_benchmark_alternate_actor_configuration_preserves_source_identity() {
    let (h, _) = level32(209072);
    for (spec, entry, source_spec, class) in [
        (65, 209057, 66, 2),
        (256, 215011, 257, 5),
        (264, 209053, 262, 7),
        (270, 227734, 269, 10),
        (1468, 215240, 1467, 13),
    ] {
        let configured = super::simulator::healer_profiles::configure(h.app.world(), spec).unwrap();
        let source = h
            .app
            .world()
            .resource::<FollowerCatalog>()
            .profiles
            .iter()
            .find(|profile| profile.entry == entry)
            .unwrap();
        assert_eq!(
            (configured.entry, configured.class_id, configured.spec_id),
            (entry, class, spec)
        );
        assert_eq!(configured.name, source.name);
        assert_eq!(
            source.spec_id, source_spec,
            "authored identity remains unchanged"
        );
        assert_eq!(configured.role, shared::group::GroupRole::Healer);
        assert_eq!(
            (
                configured.attack_spell,
                configured.cooldown_spell,
                configured.taunt_spell,
                configured.form_spell
            ),
            (0, 0, 0, 0),
            "source DPS/tank actions are not healer policy"
        );
    }
    for spec in [105, 257] {
        let configured = super::simulator::healer_profiles::configure(h.app.world(), spec).unwrap();
        let source = h
            .app
            .world()
            .resource::<FollowerCatalog>()
            .profiles
            .iter()
            .find(|profile| profile.spec_id == spec)
            .unwrap();
        assert_eq!(
            (
                configured.entry,
                configured.heal_spell,
                configured.attack_spell
            ),
            (source.entry, source.heal_spell, source.attack_spell)
        );
    }
}
