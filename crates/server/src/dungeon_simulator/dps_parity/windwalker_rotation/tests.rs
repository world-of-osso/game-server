use super::*;
use crate::simc_apl_native::FightStyle;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn fixture(world: &World) -> Encounter {
    Encounter::Fixture {
        started_at: crate::spell_cast::now(world),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    }
}

fn action(name: &str) -> Action {
    Action {
        name: name.into(),
        options: Default::default(),
    }
}

fn cast(run: &mut super::super::Run, spell: u32) {
    use crate::spell_cast::CastFailure;
    for _ in 0..1200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.targets[0]),
            false,
        ) {
            Ok(()) => {
                run.app.update();
                return;
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::InProgress
                | CastFailure::NotEnoughPower(_),
            ) => run.app.update(),
            Err(reason) => panic!("native spell{spell} failed: {reason:?}"),
        }
    }
    panic!("native spell{spell} never became ready");
}

#[test]
#[ignore = "literal WW native RED: action-specific combo facts after actual TP and utility"]
fn literal_ww_native_combo_facts_follow_the_proposed_action_and_ignore_utility() {
    let mut run = super::super::prepare(269, 1).unwrap();
    let clock = fixture(run.app.world());
    let rotation = Rotation::load(run.app.world_mut(), run.actor, clock).unwrap();
    for (name, expected) in [
        ("cooldown.whirling_dragon.remains", 0.0),
        (
            "cooldown.invoke_power_infusion_0.duration",
            i64::MIN as f64 / 1000.0,
        ),
    ] {
        assert_eq!(
            rotation
                .value_for(
                    run.app.world_mut(),
                    run.actor,
                    &run.targets,
                    &action("tiger_palm"),
                    name
                )
                .unwrap(),
            expected
        );
    }
    assert!(
        rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("tiger_palm"),
                "cooldown.unknown_whirling_dragon.remains"
            )
            .is_err()
    );
    assert_eq!(
        rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("tiger_palm"),
                "combo_strike"
            )
            .unwrap(),
        1.0
    );
    cast(&mut run, 100780);
    assert_eq!(
        rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("tiger_palm"),
                "combo_strike"
            )
            .unwrap(),
        0.0
    );
    assert_eq!(
        rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("blackout_kick"),
                "combo_strike"
            )
            .unwrap(),
        1.0
    );
    cast(&mut run, 115546);
    assert_eq!(
        rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("tiger_palm"),
                "combo_strike"
            )
            .unwrap(),
        0.0
    );
}

fn run_literal(count: usize) {
    let _scope = crate::random::seed_scope(70448);
    let mut run = super::super::prepare(269, count).unwrap();
    super::super::engage(&mut run);
    let clock = fixture(run.app.world());
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, clock).unwrap();
    for _ in 0..600 {
        rotation
            .drive(run.app.world_mut(), run.actor, &run.targets)
            .unwrap();
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let casts = |spell| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::CastSuccess
            })
            .count()
    };
    assert!(
        casts(100780) > 0,
        "literal opener must naturally build Chi with native Tiger Palm"
    );
    assert!(
        casts(113656) > 0,
        "literal priority must reach actual FoF, not generic BOK starvation"
    );
    assert!(
        casts(107428) > 0,
        "actual native Rising Sun Kick must execute"
    );
    assert_eq!(
        casts(392983),
        0,
        "unselected Strike of the Windlord must not leak from the old generic kit"
    );
    if count == 3 {
        assert!(casts(101546) > 0, "literal multitarget SCK must execute");
        for target in &run.targets {
            assert!(
                events.iter().any(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(107270)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                }),
                "each real target must receive actual native SCK107270 damage"
            );
        }
    }
    assert!(
        run.app
            .world()
            .get::<crate::followers::Follower>(run.actor)
            .is_some(),
        "native follower identity stays intact"
    );
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(run.actor)
            .is_none(),
        "no synthetic measured player"
    );
}

#[test]
#[ignore = "literal WW native RED: 30s real ST casts under pinned source ordering"]
fn literal_ww_native_st_executes_its_real_selected_kit_without_generic_starvation() {
    run_literal(1);
}

#[test]
#[ignore = "literal WW native RED: 30s real three-target casts and recipients"]
fn literal_ww_native_three_target_executes_sck_and_damages_all_native_targets() {
    run_literal(3);
}

#[test]
#[ignore = "explicit native non-Fixture control; requires the prepared parity inputs"]
fn literal_ww_native_non_fixture_follower_keeps_its_native_kit() {
    let mut run = super::super::prepare(269, 1).unwrap();
    super::super::engage(&mut run);
    for _ in 0..80 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<super::super::literal_rotation::LiteralParityFixture>(run.actor)
            .is_none()
    );
    assert!(
        run.app
            .world()
            .get::<crate::followers::Follower>(run.actor)
            .is_some()
    );
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
        "non-Fixture follower still runs its independent native kit"
    );
}

#[test]
#[ignore = "literal WW native RED: dungeon clock unavailable instead of hidden rotation fallback"]
fn literal_ww_native_driver_rejects_dungeon_ttd_without_touching_the_native_kit() {
    let mut run = super::super::prepare(269, 1).unwrap();
    let result = Rotation::load(
        run.app.world_mut(),
        run.actor,
        Encounter::Dungeon { started_at: 0.0 },
    );
    assert!(result.err().unwrap().contains("DUNGEON_TTD_UNAVAILABLE"));
    super::super::engage(&mut run);
    for _ in 0..80 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
        "separate native kit still works; rejected literal loading cannot capture its controller"
    );
}
