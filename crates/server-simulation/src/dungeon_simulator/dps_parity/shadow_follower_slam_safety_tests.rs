//! Follower admission for native scripted Tentacle Slam secondary reach.
use super::*;
use shared::components::Position;
use shared::protocol::CombatLogKind;
use std::collections::BTreeMap;

#[test]
#[ignore = "requires native Shadow profile, spell data and follower initialization"]
fn shadow_follower_slam_refuses_idle_outsider_and_hits_engaged_secondary() {
    let (profile, traits) = read_reference(258).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        3,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    let actor = run.actor;
    let owner = fixture_player(&run);
    let [primary, second, outsider] = [run.targets[0], run.targets[1], run.targets[2]];
    for target in [primary, second] {
        crate::combat_refs::set_in_combat_with(run.app.world_mut(), owner, target);
    }
    run.app
        .world_mut()
        .entity_mut(outsider)
        .insert(crate::creature_ai::CreatureState::Idle);
    run.app.world_mut().entity_mut(actor).insert((
        crate::followers::AplControlled,
        crate::networking::CombatTarget(Some(primary)),
    ));
    face_actor_toward_primary(run.app.world_mut(), actor, primary);
    let encounter = crate::simc_apl_native::Encounter::Dungeon {
        started_at: crate::spell_cast::now(run.app.world()),
    };
    let rotation =
        server_follower_apl::shadow_rotation::Rotation::load(run.app.world_mut(), actor, encounter)
            .unwrap();
    let mut apl = server_follower_apl::FollowerApl::new(Box::new(rotation));
    let action = server::simc_apl::Action {
        source_row: None,
        name: "tentacle_slam".into(),
        target: Some(primary.to_bits()),
        options: BTreeMap::new(),
    };
    let attempt = |apl: &mut server_follower_apl::FollowerApl, run: &mut Run| {
        apl.execute_plan(
            run.app.world_mut(),
            actor,
            action.clone(),
            server_follower_apl::literal_rotation::execution::Execution::Spell(
                1227280, primary, None,
            ),
        )
        .unwrap();
    };
    attempt(&mut apl, &mut run);
    for _ in 0..20 {
        run.app.update();
    }
    let has_damage = |run: &Run, target: Entity| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && matches!(event.spell_id, Some(1227621 | 34914))
            })
    };
    assert!(
        !has_damage(&run, outsider),
        "the in-range idle outsider must not receive Slam or Vampiric Touch"
    );
    assert!(
        crate::combat_refs::active_opponents(run.app.world(), outsider)
            .next()
            .is_none(),
        "the outsider must remain out of combat"
    );
    assert!(
        !has_damage(&run, primary),
        "the unsafe Slam must not launch"
    );
    run.app.world_mut().get_mut::<Position>(outsider).unwrap().x += 100.0;
    run.app.update();
    attempt(&mut apl, &mut run);
    for _ in 0..60 {
        run.app.update();
        if has_damage(&run, primary) && has_damage(&run, second) {
            break;
        }
    }
    assert!(
        has_damage(&run, primary),
        "the safe Slam must hit its engaged primary"
    );
    assert!(
        has_damage(&run, second),
        "the safe Slam must still hit an engaged secondary"
    );
    assert!(!has_damage(&run, outsider));
}
