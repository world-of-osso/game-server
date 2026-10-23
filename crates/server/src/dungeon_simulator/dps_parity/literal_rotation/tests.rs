//! Real Player dispatch and request-time spellbook validation, not class selection.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use execution::NativeAction;
use shared::protocol::CombatLogKind;
use std::collections::BTreeMap;

fn select_palm(
    run: &mut super::super::Run,
    driver: &mut Driver,
    equipment: &Equipment,
) -> (Action, Execution) {
    for _ in 0..200 {
        let actions = [NativeAction {
            name: "tiger_palm",
            spell: 100780,
            talent: None,
            self_target: false,
        }];
        let prepared = Prepared::read(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            equipment,
            actions,
            false,
        )
        .unwrap();
        if let Some(selection) = driver
            .select(
                "default",
                &BTreeMap::<String, f64>::new(),
                &prepared,
                equipment,
            )
            .unwrap()
        {
            return selection;
        }
        run.app.update();
    }
    panic!("native Player Tiger Palm never reached actual readiness");
}

#[test]
#[ignore = "real Player dispatch RED; requires frozen native reference inputs"]
fn literal_player_native_cast_and_request_time_spellbook_rejection() {
    let _scope = crate::random::seed_scope(70448);
    let (profile, traits) = super::super::read_reference(269).unwrap();
    assert_eq!(profile.race, "human");
    let mut run = super::super::prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 1 },
    )
    .unwrap();
    super::super::engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(run.actor)
            .is_some()
    );
    assert!(
        run.app
            .world()
            .get::<crate::followers::Follower>(run.actor)
            .is_none()
    );
    assert!(
        run.app
            .world()
            .get::<shared::components::Npc>(run.actor)
            .is_none()
    );
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    capture_fixture(run.app.world_mut(), run.actor, encounter).unwrap();
    let equipment = Equipment::load(
        run.app.world(),
        run.actor,
        &mut crate::simc_apl_native::Bindings::default(),
    )
    .unwrap();
    let mut driver = Driver::new(Program::parse(&[("default", "tiger_palm")]).unwrap());
    let (action, execution) = select_palm(&mut run, &mut driver, &equipment);
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    for _ in 0..20 {
        run.app.update();
    }
    let actor = run.actor.to_bits();
    let casts = |world: &World| {
        world
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor)
                    && event.kind == CombatLogKind::CastSuccess
                    && event.spell_id == Some(100780)
            })
            .count()
    };
    assert_eq!(
        casts(run.app.world()),
        1,
        "actual Player cast succeeds once"
    );
    let (action, execution) = select_palm(&mut run, &mut driver, &equipment);
    // Knowledge can change after readiness is sampled. A native Player request
    // must reject the no-longer-known action; creature admission would bypass it.
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::class_progression::KnownSpells::default());
    let result = driver.execute(run.app.world_mut(), run.actor, action, execution);
    assert!(
        result.is_err(),
        "native Player dispatch must revalidate its actual spellbook"
    );
    assert!(result.unwrap_err().contains("NotKnown"));
    assert_eq!(
        casts(run.app.world()),
        1,
        "refused Player request produces no successful cast"
    );
}
