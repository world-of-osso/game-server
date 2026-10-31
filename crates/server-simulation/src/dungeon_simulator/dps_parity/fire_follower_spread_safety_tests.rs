//! Follower admission for Fire Blast's native Ignite spread.
use super::*;
use shared::components::Position;
use shared::protocol::CombatLogKind;
use std::collections::BTreeMap;

const FIREBALL: u32 = 133;
const FIRE_BLAST: u32 = 108853;
const IGNITE: u32 = 12654;

#[test]
#[ignore = "requires native Fire profile, spell data and follower initialization"]
fn fire_follower_blast_refuses_idle_neighbour_and_spreads_onto_passive_one() {
    let (profile, traits) = read_reference(63).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        3,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    let actor = run.actor;
    let owner = fixture_player(&run);
    let [primary, second, neighbour] = [run.targets[0], run.targets[1], run.targets[2]];
    // The neighbour stands 9 yd from the engaged secondary and 14 yd from the
    // primary, outside both units' 10 yd assistance radius.
    let origin = *run.app.world().get::<Position>(primary).unwrap();
    for (unit, z) in [(second, 5.0), (neighbour, 14.0)] {
        run.app.world_mut().entity_mut(unit).insert(Position {
            z: origin.z + z,
            ..origin
        });
    }
    for target in [primary, second] {
        crate::combat_refs::set_in_combat_with(run.app.world_mut(), owner, target);
    }
    // A creature that can aggro idles with a CreatureState; a passive dummy has none.
    run.app
        .world_mut()
        .entity_mut(neighbour)
        .insert(crate::creature_ai::CreatureState::Idle);
    run.app.world_mut().entity_mut(actor).insert((
        crate::followers::AplControlled,
        crate::networking::CombatTarget(Some(primary)),
    ));
    face_actor_toward_primary(run.app.world_mut(), actor, primary);
    let encounter = crate::simc_apl_native::Encounter::Dungeon {
        started_at: crate::spell_cast::now(run.app.world()),
    };
    let rotation = server_follower_apl::fire_rotation::Rotation::load(
        run.app.world_mut(),
        actor,
        encounter,
        &server_follower_apl::profiles::profile(63).unwrap(),
    )
    .unwrap();
    let mut apl = server_follower_apl::FollowerApl::new(Box::new(rotation));
    let attempt = |apl: &mut server_follower_apl::FollowerApl, run: &mut Run, name: &str, spell| {
        let action = server::simc_apl::Action {
            source_row: None,
            name: name.into(),
            target: Some(primary.to_bits()),
            options: BTreeMap::new(),
        };
        apl.execute_plan(
            run.app.world_mut(),
            actor,
            action,
            server_follower_apl::literal_rotation::execution::Execution::Spell(
                spell, primary, None,
            ),
        )
        .unwrap();
    };
    let ignited =
        |run: &Run, target: Entity| crate::auras::has_aura(run.app.world(), target, IGNITE);
    let blasted = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(primary.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(FIRE_BLAST)
            })
    };
    // Fire Blast spreads an Ignite the primary already carries.
    attempt(&mut apl, &mut run, "fireball", FIREBALL);
    for _ in 0..600 {
        run.app.update();
        if ignited(&run, primary) {
            break;
        }
    }
    assert!(ignited(&run, primary), "Fireball must ignite the primary");

    attempt(&mut apl, &mut run, "fire_blast", FIRE_BLAST);
    for _ in 0..60 {
        run.app.update();
    }
    assert!(!blasted(&run), "the unsafe Fire Blast must not launch");
    assert!(
        !ignited(&run, neighbour),
        "Ignite must not spread onto the idle neighbour"
    );
    assert!(
        crate::combat_refs::active_opponents(run.app.world(), neighbour)
            .next()
            .is_none(),
        "the idle neighbour must remain out of combat"
    );

    // The same spot as a passive dummy, which cannot pull: the spread reaches it.
    // The spread copies one Ignite, so the secondary leaves Fire Blast's range.
    run.app
        .world_mut()
        .entity_mut(neighbour)
        .remove::<crate::creature_ai::CreatureState>();
    run.app.world_mut().get_mut::<Position>(second).unwrap().x += 100.0;
    attempt(&mut apl, &mut run, "fire_blast", FIRE_BLAST);
    for _ in 0..60 {
        run.app.update();
        if ignited(&run, neighbour) {
            break;
        }
    }
    assert!(blasted(&run), "the safe Fire Blast must hit its primary");
    assert!(
        ignited(&run, neighbour),
        "native Fire Blast spread must reach the neighbour's position"
    );
}
