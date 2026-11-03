//! Enhancement decisions return to the caller; `SpecRotation::execute` is their only dispatch.
use super::*;
use server_follower_apl::{SpecRotation, literal_rotation::PrecombatPoll};

fn load(run: &mut Run) -> enhancement_rotation::Rotation {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    enhancement_rotation::Rotation::load(run.app.world_mut(), run.actor, encounter, &run.profile)
        .unwrap()
}

fn event_count(run: &Run) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len()
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_precombat_poll_selects_and_only_trait_execute_casts() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    let mut rotation = load(&mut run);
    let mut executed = Vec::new();
    for _ in 0..200 {
        let before = event_count(&run);
        let poll = SpecRotation::poll_precombat(
            &mut rotation,
            run.app.world_mut(),
            run.actor,
            &run.targets,
        )
        .unwrap();
        let PrecombatPoll::Selected { action, execution } = poll else {
            if matches!(poll, PrecombatPoll::Finished) {
                break;
            }
            preparation::finish_pending_setup(&mut run).unwrap();
            continue;
        };
        assert_eq!(
            event_count(&run),
            before,
            "polling {} must not cast or apply anything",
            action.name
        );
        executed.push(action.name.clone());
        SpecRotation::execute(
            &mut rotation,
            run.app.world_mut(),
            run.actor,
            action,
            execution,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    assert_eq!(
        &executed[..3],
        ["windfury_weapon", "flametongue_weapon", "lightning_shield"],
        "pinned precombat rows in source order: {executed:?}"
    );
    for spell in [319773, 319778] {
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, spell),
            "trait execution must apply native weapon imbue {spell}"
        );
    }
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_trait_execute_switches_thorims_invocation_to_chain_lightning() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 3).unwrap();
    let mut rotation = load(&mut run);
    while rotation
        .step(run.app.world_mut(), run.actor, &run.targets, "precombat")
        .unwrap()
    {
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    engage(&mut run);
    assert!(!rotation.ti_chain_lightning);
    let (actor, targets) = (run.actor, run.targets.clone());
    let mut invoked = Vec::new();
    for _ in 0..6000 {
        literal_rotation::drive_tick(run.app.world_mut(), actor, |world| {
            if let Some((action, execution)) = rotation.select(world, actor, &targets, "default")? {
                let name = action.name.clone();
                SpecRotation::execute(&mut rotation, world, actor, action, execution)?;
                if matches!(name.as_str(), "chain_lightning" | "tempest") {
                    invoked.push((name, rotation.ti_chain_lightning));
                }
            }
            Ok(())
        })
        .unwrap();
        if !invoked.is_empty() {
            break;
        }
        run.app.update();
    }
    assert!(
        !invoked.is_empty(),
        "three-target pinned APL never chose Chain Lightning or Tempest"
    );
    assert!(
        invoked.iter().all(|(_, chain)| *chain),
        "trait execution must select Chain Lightning for Thorim's Invocation: {invoked:?}"
    );
}
