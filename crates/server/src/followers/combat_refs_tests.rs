//! Native31146 uses CombatManager references, not a fabricated threat list.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use super::{Follower, FollowerCatalog, ai, kit_area};
use crate::spell_cast::test_harness::{Harness, load_native_creature_world};
use bevy::prelude::*;
use shared::components::{CombatStatus, ThreatTable};

const TRAINING_DUMMY: u32 = 31_146;
const MEREDY: u32 = 209_059;
const METEOR: u32 = 153_561;

fn native_dummies() -> (Harness, Entity, [Entity; 3]) {
    let (mut h, unit, _) = spawn_profile_fixture_with_plugins(MEREDY, |_| {});
    h.app.world_mut().despawn(h.npc);
    let native = load_native_creature_world(&[TRAINING_DUMMY]);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    let targets = std::array::from_fn(|index| spawn_dummy(&mut h, unit, index));
    (h, unit, targets)
}

fn spawn_dummy(h: &mut Harness, unit: Entity, index: usize) -> Entity {
    // Same real template/summon path as dungeon_simulator::dps_parity.
    let target = crate::zones::summon_creature(
        h.app.world_mut(),
        unit,
        TRAINING_DUMMY,
        [1.5, index as f32, 0.0, std::f32::consts::PI],
    )
    .expect("native training dummy31146");
    assert!(
        h.app
            .world()
            .get::<crate::creature_scripts::TrainingDummy>(target)
            .is_some()
    );
    assert!(h.app.world().get::<ThreatTable>(target).is_none());
    assert!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(target)
            .is_none_or(|t| t.0.is_none())
    );
    target
}

#[test]
fn follower_combatrefs_native_three_targets_reach_area_policy() {
    let (mut h, unit, targets) = native_dummies();
    let owner = h.caster;
    h.app.world_mut().entity_mut(owner).insert((
        crate::combat::AutoAttack::new(targets[0]),
        crate::networking::CombatTarget(Some(targets[0])),
    ));
    for (index, target) in targets.into_iter().enumerate() {
        let member = if index == 2 { unit } else { owner };
        crate::combat_refs::set_in_combat_with(h.app.world_mut(), target, member);
        assert_eq!(
            h.app.world().get::<CombatStatus>(target),
            Some(&CombatStatus(true))
        );
    }
    let follower = h.app.world().get::<Follower>(unit).unwrap().clone();
    let party = ai::party_units(h.app.world_mut(), owner);
    let enemies = ai::engaged_enemies(h.app.world_mut(), &follower, unit, &party);
    assert_eq!(
        enemies.len(),
        3,
        "all three real31146 are in party combat; owner victim alone previously admitted only one"
    );
    assert_eq!(enemies[0].0, targets[0], "selected target retains priority");
    let meteor = h
        .app
        .world()
        .resource::<FollowerCatalog>()
        .kits
        .iter()
        .find(|entry| entry.spell_id == METEOR && entry.available(63, 32))
        .unwrap()
        .clone();
    assert_eq!(meteor.conditions.min_enemies, 3);
    assert!(kit_area::allows(
        h.app.world_mut(),
        unit,
        targets[0],
        &enemies,
        &meteor
    ));

    // Nearby but unengaged enemies still veto an implicit area. Merely holding
    // combat with an outsider must not recruit that outsider's pack either.
    let fresh = spawn_dummy(&mut h, unit, 3);
    assert!(
        h.app
            .world()
            .get::<CombatStatus>(fresh)
            .is_none_or(|status| !status.0)
    );
    let outsider = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    crate::combat_refs::set_in_combat_with(h.app.world_mut(), fresh, outsider);
    let enemies = ai::engaged_enemies(h.app.world_mut(), &follower, unit, &party);
    assert_eq!(enemies.len(), 3, "nonparty combat must not be recruited");
    assert!(!enemies.iter().any(|(enemy, _)| *enemy == fresh));
    assert!(
        !kit_area::allows(h.app.world_mut(), unit, targets[0], &enemies, &meteor),
        "area must not pull unengaged packs"
    );

    h.app
        .world_mut()
        .entity_mut(owner)
        .remove::<crate::combat::AutoAttack>();
    let enemies = ai::engaged_enemies(h.app.world_mut(), &follower, unit, &party);
    assert_eq!(
        enemies.len(),
        3,
        "CombatRefs suffice without any owner swing"
    );
    for target in targets {
        crate::combat_refs::end_all_combat(h.app.world_mut(), target);
    }
    assert!(
        ai::engaged_enemies(h.app.world_mut(), &follower, unit, &party).is_empty(),
        "selection alone cannot keep a stopped fight engaged"
    );
}
