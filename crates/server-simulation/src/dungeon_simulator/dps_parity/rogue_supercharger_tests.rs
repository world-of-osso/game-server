use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

const CHARGES: &[u32] = &[470398, 470406, 470409, 470412, 470414, 470415, 470416];

fn fund(run: &mut Run, points: f32) {
    let mut powers = run
        .app
        .world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
        .unwrap();
    let energy = powers.get_mut(PowerType::Energy).unwrap();
    energy.current = energy.max;
    powers.get_mut(PowerType::ComboPoints).unwrap().current = points;
}

fn cast(run: &mut Run, spell: u32, self_target: bool) {
    let target = if self_target {
        run.actor
    } else {
        run.targets[0]
    };
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, target, Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
}

fn read_charges(run: &Run) -> Vec<u32> {
    CHARGES
        .iter()
        .copied()
        .filter(|spell| crate::auras::has_aura(run.app.world(), run.actor, *spell))
        .collect()
}

fn earn_charges(run: &mut Run, producer: u32, self_target: bool) -> usize {
    assert!(read_charges(run).is_empty());
    let count =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 470347, 0)
            as usize;
    cast(run, producer, self_target);
    // SimC76f8118 sc_rogue.cpp8913–8933/10961–10966:
    // actual producers fill the first inactive native counters in order.
    assert_eq!(
        read_charges(run),
        CHARGES[..count],
        "native producer{producer} must earn ordered Supercharger uses"
    );
    count
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn rogue_supercharger_outlaw_adrenaline_earns_effective_spree_points() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    let count = earn_charges(&mut run, 13750, true);
    let extra =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 470347, 1);
    let forced = if crate::auras::has_aura(run.app.world(), run.actor, 470668) {
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 470668, 0)
    } else {
        0.0
    };
    let effective = (5.0 + extra + forced) as usize;
    fund(&mut run, 5.0);
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    cast(&mut run, 51690, true);
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let returned = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1235074)
                && event.kind == CombatLogKind::Energize
        })
        .count();
    let maximum =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::ComboPoints)
            .unwrap()
            .1 as usize;
    // sc_rogue.cpp4794–4805: excess effective points increase hand damage,
    // NOT duration. Native Duration1319's3500ms maximum caps this7-CP actor.
    assert_eq!(
        returned,
        effective.min(maximum),
        "earned Supercharger must cap actual Spree ticks at the spend limit"
    );
    assert_eq!(
        read_charges(&run),
        CHARGES[..count - 1],
        "the effective finisher consumes the last earned counter, not the first"
    );
}

fn first_noncritical_spree_hand(overcap: bool) -> (f32, f32) {
    // Both controls earn the same actual AR/counters. Only paid CP differs,
    // so ordinary AR and item modifiers cannot masquerade as an overcap bonus.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    cast(&mut run, 13750, true);
    let maximum =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::ComboPoints)
            .unwrap()
            .1;
    let forced = if crate::auras::has_aura(run.app.world(), run.actor, 470668) {
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 470668, 0)
    } else {
        0.0
    };
    let extra =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 470347, 1)
            + forced;
    assert!(extra > 0.0 && extra < maximum);
    let paid = if overcap { maximum } else { maximum - extra };
    let per_point =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 51690, 3)
            / 100.0;
    fund(&mut run, paid);
    cast(&mut run, 51690, true);
    let event = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(57841)
                && event.kind == CombatLogKind::Damage
                && !event.crit
                && event.amount > 0
        })
        .expect("actual native Spree must supply a noncritical hand hit");
    let excess = if overcap { extra } else { 0.0 };
    (event.amount as f32, 1.0 + per_point * excess)
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn rogue_supercharger_overcap_spree_scales_native_hand_damage() {
    let (ordinary, _) = first_noncritical_spree_hand(false);
    let (charged, expected) = first_noncritical_spree_hand(true);
    // SimC76f8118 sc_rogue.cpp4702–4714: each excess effective point
    // adds the native Spree effect4 fraction to both actual hand payloads.
    let ratio = charged / ordinary;
    assert!(
        (expected * 0.97..expected * 1.03).contains(&ratio),
        "native overcap hand ratio must be{expected}, got{ratio} ({ordinary}/{charged})"
    );
}

fn assert_other_rogue_producer(spec: u32, producer: u32, finisher: u32, self_target: bool) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(spec, 1).unwrap();
    engage(&mut run);
    let count = earn_charges(&mut run, producer, self_target);
    fund(&mut run, 5.0);
    cast(&mut run, finisher, false);
    assert_eq!(
        read_charges(&run),
        CHARGES[..count - 1],
        "native spec{spec} finisher must consume one highest earned use"
    );
}

#[test]
#[ignore = "requires private native259 player profile and traits"]
fn rogue_supercharger_unselected_assassination_does_not_earn_counters() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(259, 1).unwrap();
    engage(&mut run);
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 470347));
    cast(&mut run, 385627, false);
    assert!(
        read_charges(&run).is_empty(),
        "unselected Supercharger must not grant counters"
    );
}

#[test]
#[ignore = "requires private native261 player profile and traits"]
fn rogue_supercharger_subtlety_dance_earns_then_eviscerate_consumes() {
    assert_other_rogue_producer(261, 185313, 196819, true);
}
