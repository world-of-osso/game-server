//! Native MM action boundaries; spell/aura records supply every magnitude.
use super::*;
use literal_rotation::{
    equipment::Equipment,
    execution::{NativeAction, Prepared, execute},
};
use server::simc_apl::Action;
use shared::protocol::CombatLogKind;

pub(super) fn player(count: usize) -> Run {
    let (profile, traits) = read_reference(254).unwrap();
    let identity = inputs::read_actor(&profile).unwrap();
    let mut run = prepare_literal_reference(profile, &traits, count, identity).unwrap();
    engage(&mut run);
    run
}

/// Returns Focus paid immediately by an instant request, before regen advances.
pub(super) fn cast(run: &mut Run, spell: u32, self_target: bool) -> f32 {
    let equipment = Equipment::load(run.app.world(), run.actor, &mut Default::default()).unwrap();
    let action = Action {
        source_row: None,
        name: "probe".into(),
        target: None,
        options: Default::default(),
    };
    for _ in 0..600 {
        let prepared = Prepared::read(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            &equipment,
            [NativeAction {
                name: "probe",
                spell,
                self_target,
                talent: None,
            }],
            false,
        )
        .unwrap();
        if let Some(execution) = prepared.execution(&action, &equipment).unwrap() {
            let before = current_focus(run);
            execute(run.app.world_mut(), run.actor, execution).unwrap();
            let spent = before - current_focus(run);
            preparation::finish_pending_setup(run).unwrap();
            return spent;
        }
        run.app.update();
    }
    panic!("native spell{spell} did not become ready");
}

fn current_focus(run: &Run) -> f32 {
    crate::spell_cast::powers::power_value(
        run.app.world(),
        run.actor,
        shared::components::PowerType::Focus,
    )
    .unwrap()
    .0
}

pub(super) fn owned_buff(run: &Run, spell: u32) -> bool {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .any(|aura| aura.caster == run.actor && aura.spell_id == spell)
}

fn arcane_damage_after_precise_launch_expiry(precise: bool) -> (f64, f64) {
    let mut run = player(1);
    let arcane = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(185358)
        .unwrap()
        .clone();
    let live_modifier = |run: &Run| {
        f64::from(crate::auras::spell_mods::modify(
            run.app.world(),
            run.actor,
            &arcane,
            crate::auras::spell_mods::SpellModOp::HealingAndDamage,
            1.0,
        ))
    };
    let baseline = live_modifier(&run);
    if precise {
        let spell = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(260242)
            .unwrap();
        let effect_mask = crate::auras::aura_effect_mask(spell);
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: 260242,
                effect_mask,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    // SimC76f8118 sc_hunter.cpp4121/4127–4130: snapshot the earned
    // Precise multiplier before the successful cast consumes it.
    let expected = live_modifier(&run) / baseline;
    let target = run.targets[0];
    run.app.world_mut().insert_resource(
        crate::spell_cast::effects::factor_capture::FactorCapture::new(
            run.actor,
            target,
            185358,
            "arcane_shot",
            1,
        ),
    );
    let _seed = crate::random::seed_scope(70_448);
    execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(185358, target, Some(target)),
    )
    .unwrap();
    // Reproduce the production boundary: launch first, consume before projectile arrival.
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 260242,
    );
    for _ in 0..40 {
        run.app.update();
        let capture = run
            .app
            .world()
            .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
            .export();
        if capture["captured_hits"].as_u64() == Some(1) {
            break;
        }
    }
    let receipt = run
        .app
        .world()
        .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
        .export();
    let event = |kind: &str| {
        receipt["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == kind && event["spell_id"] == 185358)
            .expect("actual Arcane Shot projectile hit")
    };
    let observed = event("result")["factors"]["native_spell_mod_product"]
        .as_f64()
        .expect("actual native damage spell-mod product");
    // Gear procs change live modifiers during flight AND after the impact.
    // Normalize by the other native modifiers actually applied at this hit,
    // rather than a different frame's stats, damage variance or crit outcome.
    let other_modifiers: f64 = event("direct_amount")["applied_modifiers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|modifier| modifier["op"] == 0 && modifier["source_spell_id"] != 260242)
        .map(|modifier| 1.0 + modifier["value"].as_f64().unwrap() / 100.0)
        .product();
    (observed / other_modifiers, expected)
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn marksmanship_precise_damage_survives_cast_consumption_until_actual_projectile_hit() {
    let (plain, plain_live) = arcane_damage_after_precise_launch_expiry(false);
    let (empowered, empowered_live) = arcane_damage_after_precise_launch_expiry(true);
    let expected = empowered_live / plain_live;
    assert!(plain > 0.0 && expected > 1.0);
    assert!(
        (empowered / (plain * expected) - 1.0).abs() < 0.01,
        "resolved native Precise modifier must survive consumption: plain{plain}, empowered{empowered}, expected{expected}"
    );
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn marksmanship_unstable_trigger_allows_one_free_explosive_recast_inside_native_cooldown() {
    let mut run = player(1);
    assert!(
        cast(&mut run, 212431, false) > 0.0,
        "ordinary Explosive Shot pays native Focus"
    );
    assert!(
        owned_buff(&run, 1301778),
        "selected Unstable Trigger grants its native free-recast buff"
    );
    let first = crate::spell_cast::now(run.app.world());
    assert_eq!(
        cast(&mut run, 212431, false),
        0.0,
        "Unstable recast spends no Focus"
    );
    assert!(
        crate::spell_cast::now(run.app.world()) - first < 3.0,
        "second cast uses the sourced recast window, not a waited30s cooldown"
    );
    assert!(
        !owned_buff(&run, 1301778),
        "free recast does not create another window"
    );
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn marksmanship_aimed_shot_earns_precise_shots_consumed_by_arcane_shot() {
    let mut run = player(1);
    cast(&mut run, 19434, false);
    assert!(
        owned_buff(&run, 260242),
        "actual Aimed Shot earns Precise Shots"
    );
    cast(&mut run, 185358, false);
    assert!(
        !owned_buff(&run, 260242),
        "an actual Precise Shots spender consumes the buff"
    );
    assert!(
        owned_buff(&run, 1253750),
        "selected Stargazer is granted by the actual Precise spender"
    );
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn marksmanship_selected_no_scope_earns_precise_shots_from_rapid_fire() {
    let mut run = player(1);
    cast(&mut run, 257044, false);
    assert!(
        owned_buff(&run, 260242),
        "selected native No Scope makes Rapid Fire grant Precise Shots"
    );
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn marksmanship_chakram_is_not_granted_by_an_ordinary_aimed_shot() {
    let mut run = player(1);
    assert!(!owned_buff(&run, 1264946));
    cast(&mut run, 19434, false);
    assert!(
        !owned_buff(&run, 1264946),
        "only actual Trueshot may produce Moonlight Chakram"
    );
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn marksmanship_trueshot_grants_one_chakram_that_its_cast_consumes() {
    let mut run = player(1);
    cast(&mut run, 288613, true);
    assert!(owned_buff(&run, 288613), "real native Trueshot applied");
    assert!(
        owned_buff(&run, 1264946),
        "selected talent grants its actual native buff"
    );
    cast(&mut run, 1264949, false);
    assert!(
        !owned_buff(&run, 1264946),
        "a successful Chakram consumes its enabler"
    );
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn marksmanship_chakram_reaches_all_seven_native_damage_bounces() {
    let mut run = player(1);
    cast(&mut run, 288613, true);
    cast(&mut run, 1264949, false);
    for _ in 0..100 {
        run.app.update();
    }
    let impacts: Vec<_> = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1266081)
                && event.kind == CombatLogKind::Damage
        })
        .collect();
    assert_eq!(impacts.len(), 7, "native six-bounce count plus initial hit");
    assert!(impacts.iter().all(|hit| hit.amount > 0));
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn marksmanship_volley_periodic_damage_reaches_each_actual_fixture_target() {
    let mut run = player(3);
    cast(&mut run, 260243, false);
    for _ in 0..125 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        let hits: Vec<_> = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(260247)
                    && event.kind == CombatLogKind::Damage
            })
            .collect();
        assert_eq!(
            hits.len(),
            12,
            "native six seconds,500ms periodic interval on {target:?}"
        );
        assert!(hits.iter().all(|hit| hit.amount > 0));
    }
}
