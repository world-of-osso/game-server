//! Regroup stops only the follower's own swing, not its attackers.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use crate::combat::AutoAttack;

#[test]
fn regroup_does_not_stop_an_enemy_attacking_the_follower() {
    let (mut h, follower, _) = spawn_profile_fixture_with_plugins(209059, |_| {});
    let enemy = h.npc;
    let owner = h.caster;
    h.app
        .world_mut()
        .entity_mut(enemy)
        .insert(AutoAttack::new(follower));
    h.app
        .world_mut()
        .entity_mut(follower)
        .insert(AutoAttack::new(enemy));
    crate::combat_refs::set_in_combat_with(h.app.world_mut(), enemy, follower);

    super::ai::regroup(h.app.world_mut(), follower, owner);

    assert!(h.app.world().get::<AutoAttack>(follower).is_none());
    assert_eq!(
        h.app.world().get::<AutoAttack>(enemy).unwrap().victim,
        follower
    );
    assert!(crate::combat_refs::has_active_reference_to(
        h.app.world(),
        enemy,
        follower
    ));
}
