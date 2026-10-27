//! Actual Infernal native driver clocks, power inputs and owner energize.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::{Npc, PowerType};
use shared::protocol::CombatLogKind;

fn find_actual_infernal(run: &mut Run) -> Option<Entity> {
    run.app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature, &Npc)>()
        .iter(run.app.world())
        .find(|(_, summon, npc)| {
            (summon.owner, summon.spell_id, npc.template_id) == (run.actor, 111685, 89)
        })
        .map(|(entity, _, _)| entity)
}

fn wait_for_actual_infernal(run: &mut Run) -> Entity {
    for _ in 0..100 {
        run.app.update();
        if let Some(actor) = find_actual_infernal(run) {
            return actor;
        }
    }
    panic!("actual111685 must summon its real owned Infernal89");
}

fn count_immolation_hits(run: &Run, infernal: Entity, target: Entity) -> usize {
    let expected = (
        Some(infernal.to_bits()),
        Some(target.to_bits()),
        Some(20153),
    );
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            let identity = (event.source, event.target, event.spell_id);
            let damage = event.kind == CombatLogKind::Damage;
            identity == expected && damage && event.amount > 0
        })
        .count()
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_infernal_actual_native_drivers_delay_repeat_and_fund_owner() {
    // Native attack RED02e475cbb6 reaches the real89, but no repeated20153.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 3).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    let actual = literal_rotation::execution::resolve_action(run.app.world(), run.actor, 1122)
        .unwrap()
        .expect("selected native Infernal action");
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(actual, target, Some(target)),
    )
    .unwrap();
    // Observe actual creation, not owner GCD completion: instant1122 has a
    // native1500ms GCD, longer than the source's nominal1s Embers delay.
    let infernal = wait_for_actual_infernal(&mut run);
    let observed_spawn = crate::spell_cast::now(run.app.world());
    assert!(!crate::auras::has_aura(run.app.world(), infernal, 264364));
    assert!(!crate::auras::has_aura(run.app.world(), infernal, 19483));
    let before =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::SoulShards)
            .unwrap()
            .0;
    run.app
        .insert_resource(FactorCapture::new(infernal, target, 20153, "immolation", 1));
    let mut embers_at = None;
    let mut immolation_at = None;
    let mut owner_power_at_hit = None;
    for _ in 0..160 {
        let power = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .spell_power;
        run.app.update();
        let now = crate::spell_cast::now(run.app.world());
        if embers_at.is_none() && crate::auras::has_aura(run.app.world(), infernal, 264364) {
            embers_at = Some(now);
        }
        if immolation_at.is_none() && crate::auras::has_aura(run.app.world(), infernal, 19483) {
            immolation_at = Some(now);
        }
        if owner_power_at_hit.is_none() && count_immolation_hits(&run, infernal, target) > 0 {
            owner_power_at_hit = Some(power);
        }
    }
    let embers_at = embers_at.expect("actual native264364 driver must activate");
    let immolation_at = immolation_at.expect("actual native19483 driver must activate");
    // Fixed70448 fixture: distinguish source delay from next-frame dispatch.
    let arise_delay = embers_at - observed_spawn;
    assert!(
        (0.5..=1.55).contains(&arise_delay),
        "seeded native Embers activation delay={arise_delay}"
    );
    let extra = immolation_at - embers_at;
    // pets.cpp2103–2119: uniform0–0.75s extra; normal native50ms frames.
    assert!((0.0..=0.80).contains(&extra), "native extra delay={extra}");
    for &recipient in &run.targets {
        assert!(
            count_immolation_hits(&run, infernal, recipient) >= 2,
            "actual Infernal must repeat native20153 at every native pack recipient"
        );
    }
    let after =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::SoulShards)
            .unwrap()
            .0;
    assert!(
        after > before,
        "native Embers must fund its existing owner: {before}→{after}"
    );
    let owner_power = owner_power_at_hit.expect("actual Immolation must land");
    let capture = run.app.world().resource::<FactorCapture>().export();
    let calculation = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 20153)
        .expect("actual20153 must capture native input power");
    let expected = f64::from(owner_power) * 2.2275;
    for factor in ["s_power", "a_power"] {
        let inherited = calculation["factors"][factor].as_f64().unwrap();
        assert!(
            (inherited - expected).abs() < 1.0,
            "actual Infernal {factor}: {inherited} vs {expected}"
        );
    }
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_infernal_immolation_actual_tick_interval_inherits_owner_haste() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    let actual = literal_rotation::execution::resolve_action(run.app.world(), run.actor, 1122)
        .unwrap()
        .expect("selected native Infernal action");
    let caster = run.actor;
    literal_rotation::execution::execute(
        run.app.world_mut(),
        caster,
        literal_rotation::execution::Execution::Spell(actual, target, Some(target)),
    )
    .unwrap();
    let infernal = wait_for_actual_infernal(&mut run);
    // Source pet.cpp217–228 and buff.cpp752/1945: native19483 attr5bit13
    // scales its actual periodic20153 interval by the owner's spell haste.
    let native_period = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(19483)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .aura_period_ms;
    let mut observed = Vec::new();
    let mut previous_count = 0;
    let mut expected = None;
    for _ in 0..300 {
        run.app.update();
        if expected.is_none() && crate::auras::has_aura(run.app.world(), infernal, 19483) {
            let haste = run
                .app
                .world()
                .get::<crate::stats::CharacterStats>(caster)
                .unwrap()
                .haste_pct;
            assert!(
                haste > 10.0,
                "real native owner haste is the test input: {haste}"
            );
            // SimC buff.cpp:1203-1229 applies owner passive period modifiers.
            // Selected Destructive Rapidity 1244928 reduces this period by 10%.
            expected =
                Some(f64::from(native_period) / 1000.0 * 0.9 / (1.0 + f64::from(haste) / 100.0));
        }
        let count = count_immolation_hits(&run, infernal, target);
        if count > previous_count {
            observed.push(crate::spell_cast::now(run.app.world()));
            previous_count = count;
        }
        if observed.len() == 3 {
            break;
        }
    }
    assert_eq!(
        observed.len(),
        3,
        "actual native Infernal must land three ticks"
    );
    let expected = expected.expect("actual19483 must be active before damage");
    for pair in observed.windows(2) {
        let interval = pair[1] - pair[0];
        assert!(
            (interval - expected).abs() <= 0.051,
            "actual Infernal tick interval={interval} must follow native owner haste, expected={expected}"
        );
    }
}
