//! SIMC_RET_DIVINE_PURPOSE_TRIGGER: real Holy Power funding, no proc chance override.
use super::ret_tier_tests::{cast_finisher, has, native_ret};
use super::*;
use crate::spell_cast::{CastFailure, CostPool, test_harness::ObservedCombatEvents};
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

const PURPOSE_TALENT: u32 = 408459;
const PURPOSE: u32 = 223819;
const FINAL_VERDICT: u32 = 383328;
const JUDGMENT: u32 = 20271;
const TEMPLARS_VERDICT: u32 = 85256;

#[derive(Resource, Default)]
struct GuillotineProcInputs(Vec<crate::auras::procs::AuraEvent>);

fn record_guillotine_input(
    world: &mut World,
    unit: Entity,
    aura: &crate::auras::AuraInstance,
    event: &crate::auras::procs::AuraEvent,
) {
    world
        .resource_mut::<GuillotineProcInputs>()
        .0
        .push(event.clone());
    crate::spell_cast::scripts::cast_guillotine(world, unit, aura, event);
}

#[derive(Resource)]
struct FixtureAdmission {
    owner: Entity,
    copy: crate::maps::InMap,
}

fn trace_follower_lifetime(run: &Run) -> Value {
    let world = run.app.world();
    let Some(admission) = world.get_resource::<FixtureAdmission>() else {
        return json!({"actor_is_player": true});
    };
    let events = world
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.target == Some(run.actor.to_bits())
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Death)
        })
        .collect::<Vec<_>>();
    json!({"actor_exists": world.get_entity(run.actor).is_ok(),
        "actor_health": world.get::<shared::components::Health>(run.actor),
        "actor_map": format!("{:?}", world.get::<crate::maps::InMap>(run.actor)),
        "owner_exists": world.get_entity(admission.owner).is_ok(),
        "owner_map": format!("{:?}", world.get::<crate::maps::InMap>(admission.owner)),
        "admitted_copy": format!("{:?}", admission.copy),
        "target_map": format!("{:?}", world.get::<crate::maps::InMap>(run.targets[0])),
        "damage_or_death_events": events,
        "guillotine_proc_inputs": world.resource::<GuillotineProcInputs>().0.iter()
            .map(|event| format!("{event:?}")).collect::<Vec<_>>()})
}

fn native_ret_in_melee() -> Run {
    let mut run = native_ret(2);
    let owner = run
        .app
        .world()
        .get::<crate::followers::Follower>(run.actor)
        .unwrap()
        .owner;
    let copy = *run
        .app
        .world()
        .get::<crate::maps::InMap>(run.actor)
        .unwrap();
    run.app.insert_resource(FixtureAdmission { owner, copy });
    run.app.init_resource::<GuillotineProcInputs>();
    run.app
        .world_mut()
        .resource_mut::<crate::auras::scripts::AuraScripts>()
        .0
        .get_mut(&crate::spell_cast::scripts::GUILLOTINE)
        .unwrap()
        .on_damage_proc = Some(record_guillotine_input);
    // Use the same ordinary startattack/chase path as the literal controller;
    // a manually casting follower must not keep following its distant owner.
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    run
}

fn purpose_applications(run: &Run) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(PURPOSE)
                && event.kind == CombatLogKind::AuraApplied
        })
        .count()
}

fn current_holy_power(run: &Run) -> f32 {
    crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::HolyPower)
        .unwrap()
        .0
}

fn cast_generator(run: &mut Run) -> Result<(), CastFailure> {
    if run
        .app
        .world()
        .get::<shared::components::Player>(run.actor)
        .is_some()
    {
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            run.actor,
            JUDGMENT,
            Some(run.targets[0]),
        )
    } else {
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            JUDGMENT,
            Some(run.targets[0]),
            false,
        )
    }
}

fn earn_finisher_power(run: &mut Run, finisher: u32) {
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(finisher)
        .unwrap();
    let required: f32 = crate::spell_cast::applicable_costs(run.app.world(), run.actor, spell)
        .unwrap()
        .into_iter()
        .filter(|(pool, _, _)| *pool == CostPool::Power(PowerType::HolyPower))
        .map(|(_, amount, _)| amount)
        .sum();
    for _ in 0..1000 {
        if current_holy_power(run) >= required {
            return;
        }
        match cast_generator(run) {
            Ok(()) => {}
            Err(
                CastFailure::OnCooldown
                | CastFailure::OnGlobalCooldown
                | CastFailure::NoCharges
                | CastFailure::InProgress,
            ) => {}
            Err(reason) => panic!(
                "native Purpose funding Judgment failed: {reason:?}; lifetime={}",
                trace_follower_lifetime(run)
            ),
        }
        run.app.update();
    }
    panic!(
        "native generators did not earn {required} HolyPower; actual{}",
        current_holy_power(run)
    );
}

#[test]
#[ignore = "selected native408459 must produce223819 from naturally funded ordinary spenders"]
fn dps_parity_ret_purpose_selected_native_holy_power_casts_produce_real_proc() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret_in_melee();
    assert!(has(&run, PURPOSE_TALENT));
    assert_eq!(purpose_applications(&run), 0);
    for _ in 0..32 {
        earn_finisher_power(&mut run, FINAL_VERDICT);
        cast_finisher(&mut run, FINAL_VERDICT);
    }
    assert!(
        purpose_applications(&run) > 0,
        "32 real ordinary HolyPower spenders must exercise the selected native chance"
    );
}

#[test]
#[ignore = "actual Player70 native request path independently qualifies the selected Purpose producer"]
fn dps_parity_ret_purpose_player_native_holy_power_casts_produce_real_proc() {
    let _seed = crate::random::seed_scope(70448);
    let (profile, selected) = read_reference(70).unwrap();
    let mut run = prepare_reference(
        profile,
        &selected,
        1,
        crate::followers::simulator::ParityActor::Player { race: 3 },
    )
    .unwrap();
    engage(&mut run);
    assert!(has(&run, PURPOSE_TALENT));
    for _ in 0..32 {
        earn_finisher_power(&mut run, FINAL_VERDICT);
        let mut cast = false;
        for _ in 0..200 {
            match crate::spell_cast::cast_for_player_with_target(
                run.app.world_mut(),
                run.actor,
                FINAL_VERDICT,
                Some(run.targets[0]),
            ) {
                Ok(()) => {
                    cast = true;
                    break;
                }
                Err(CastFailure::OnGlobalCooldown | CastFailure::InProgress) => run.app.update(),
                Err(reason) => panic!("native Player Final Verdict failed: {reason:?}"),
            }
        }
        assert!(cast, "native Player spender reaches actual cast admission");
        for _ in 0..4 {
            run.app.update();
        }
    }
    assert!(
        purpose_applications(&run) > 0,
        "32 native Player HolyPower spenders must exercise the selected native chance"
    );
}

#[test]
#[ignore = "native408459 aura alone must not impersonate a selected talent"]
fn dps_parity_ret_purpose_unselected_aura_cannot_produce_the_selected_policy_proc() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret_in_melee();
    // Removing only DP invalidates a native23-point earlier-row gate. Use
    // the valid granted-only loadout and its actually learned base spender.
    inputs::install_traits(run.app.world_mut(), run.actor, &[]).unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert!(
        !run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(PURPOSE_TALENT)
    );
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        PURPOSE_TALENT,
        Some(run.actor),
        0,
    );
    assert!(
        has(&run, PURPOSE_TALENT),
        "control uses the actual native chance aura"
    );
    for _ in 0..32 {
        earn_finisher_power(&mut run, TEMPLARS_VERDICT);
        cast_finisher(&mut run, TEMPLARS_VERDICT);
    }
    assert_eq!(purpose_applications(&run), 0);
}

#[test]
#[ignore = "selected native408459 must not proc from full-mask background spenders"]
fn dps_parity_ret_purpose_triggered_spenders_cannot_produce_the_foreground_proc() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret_in_melee();
    for _ in 0..32 {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            FINAL_VERDICT,
            Some(run.targets[0]),
            0,
        );
        run.app.update();
    }
    assert_eq!(purpose_applications(&run), 0);
}

#[test]
#[ignore = "selected native408459 must not roll on ordinary non-HolyPower generators"]
fn dps_parity_ret_purpose_judgment_casts_cannot_produce_the_holy_power_proc() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret_in_melee();
    let mut casts = 0;
    for _ in 0..2000 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            JUDGMENT,
            Some(run.targets[0]),
            false,
        ) {
            Ok(()) => casts += 1,
            Err(
                CastFailure::OnCooldown
                | CastFailure::OnGlobalCooldown
                | CastFailure::NoCharges
                | CastFailure::InProgress,
            ) => {}
            Err(reason) => panic!("native Judgment control failed: {reason:?}"),
        }
        run.app.update();
        if casts == 8 {
            break;
        }
    }
    assert_eq!(casts, 8, "control actually casts eight native generators");
    assert_eq!(purpose_applications(&run), 0);
}
