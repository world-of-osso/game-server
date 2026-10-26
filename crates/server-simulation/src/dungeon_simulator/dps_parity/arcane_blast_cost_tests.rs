//! Ordinary Blast admission uses one charge factor and one final Mana truncation.
use super::arcane_blast_timing_tests::{charge_amount, set_native_charges};
use super::*;
use shared::casting::CastState;
use shared::components::{Mana, PowerType};

fn unrounded_blast_cost(run: &Run, spell: &crate::spell_info::SpellInfo) -> f64 {
    let world = run.app.world();
    let base_mana = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .base_mana;
    let cost = spell
        .power_costs
        .iter()
        .find(|cost| cost.power_type == PowerType::Mana as i32 && cost.order_index == 0)
        .unwrap();
    let raw =
        f64::from(cost.mana_cost) + f64::from(base_mana) * f64::from(cost.power_cost_pct) / 100.0;
    let auras = world.get::<crate::auras::ActiveAuras>(run.actor).unwrap();
    // Native461260 modifies Blast's cost; native1270845 is its global Mana
    // percentage. Read their authored amounts; do not use a rounded cost query.
    [461260, 1270845].into_iter().fold(raw, |amount, spell_id| {
        let aura = auras.iter().find(|aura| aura.spell_id == spell_id).unwrap();
        let effect = aura
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap();
        amount * (1.0 + f64::from(effect.amount) / 100.0)
    })
}

fn set_mana(run: &mut Run, amount: f32) {
    run.app
        .world_mut()
        .get_mut::<Mana>(run.actor)
        .unwrap()
        .current = amount;
}

fn request_blast(run: &mut Run) -> Result<(), String> {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(30451, run.targets[0], Some(run.targets[0])),
    )
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_blast_cost_native_admission_uses_one_charge_cost_multiplier() {
    // SimC76f8118 action.cpp1192–1238/sc_mage.cpp2850–2855 multiply the
    // unrounded cost once. Native aura423 then truncates the final Mana cost.
    // Probe1eca672f29: raw6204.6875, not the zero-charge query's6204.
    for charges in [2.0, 4.0] {
        let mut run = prepare_literal(62, 1).unwrap();
        set_native_charges(&mut run, 0.0);
        let spell = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(30451)
            .unwrap()
            .clone();
        let unrounded = unrounded_blast_cost(&run, &spell);
        let baseline: f32 = crate::spell_cast::applicable_costs(run.app.world(), run.actor, &spell)
            .unwrap()
            .into_iter()
            .filter_map(|(pool, amount, _)| {
                (pool == crate::spell_cast::CostPool::Power(PowerType::Mana)).then_some(amount)
            })
            .sum();
        assert_eq!(
            baseline,
            unrounded.trunc() as f32,
            "native fixture's zero-charge cost"
        );
        let factor = 1.0 + f64::from(charges) * f64::from(charge_amount(&run, 4)) / 100.0;
        let expected = (unrounded * factor).trunc() as f32;
        set_native_charges(&mut run, charges);
        assert!(
            expected > baseline && expected < run.app.world().get::<Mana>(run.actor).unwrap().max
        );

        set_mana(&mut run, expected - 1.0);
        let rejected = request_blast(&mut run).unwrap_err();
        assert!(
            rejected.contains("NotEnoughPower(Mana)"),
            "one Mana below the native cost must fail: {rejected}"
        );

        // No spare Mana or wider tolerance: admit exactly one correctly rounded
        // authored charge cost after rejecting a budget one unit below it.
        set_mana(&mut run, expected);
        request_blast(&mut run).unwrap_or_else(|reason| {
            panic!("Blast at{charges}charges must admit exact cost{expected}: {reason}")
        });
        assert_eq!(
            run.app
                .world()
                .get::<CastState>(run.actor)
                .unwrap()
                .spell_id,
            30451
        );
    }
}
