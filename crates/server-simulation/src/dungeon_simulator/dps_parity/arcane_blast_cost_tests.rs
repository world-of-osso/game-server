//! Separate follow-up to the timing fix: retain the tight native Mana boundary.
use super::arcane_blast_timing_tests::{charge_amount, set_native_charges};
use super::*;
use shared::casting::CastState;
use shared::components::{Mana, PowerType};

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_blast_cost_native_admission_uses_one_charge_cost_multiplier() {
    // SimC76f8118 sc_mage.cpp2850–2855: the native per-charge Mana
    // percentage multiplies cost once, in addition to ordinary caster mods.
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
        let base: f32 = crate::spell_cast::applicable_costs(run.app.world(), run.actor, &spell)
            .unwrap()
            .into_iter()
            .filter_map(|(pool, amount, _)| {
                (pool == crate::spell_cast::CostPool::Power(PowerType::Mana)).then_some(amount)
            })
            .sum();
        assert!(base > 0.0);
        let budget = base * (1.0 + charges * charge_amount(&run, 4) / 100.0);
        set_native_charges(&mut run, charges);
        let mana = run.app.world_mut().get_mut::<Mana>(run.actor).unwrap();
        assert!(
            budget + 1.0 < mana.max,
            "authored cost fits the native Mana pool"
        );
        // Preserve the original boundary while diagnosing its rounding assumption.
        run.app
            .world_mut()
            .get_mut::<Mana>(run.actor)
            .unwrap()
            .current = budget + 1.0;
        let actual_costs =
            crate::spell_cast::applicable_costs(run.app.world(), run.actor, &spell).unwrap();
        let base_mana = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .base_mana;
        let modifiers = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .flat_map(|aura| {
                aura.effects
                    .iter()
                    .filter(|effect| {
                        effect.misc_values[0] == crate::auras::SpellModOp::PowerCost0 as i32
                            || matches!(
                                effect.aura_type,
                                crate::auras::handlers::MOD_MANA_COST_PCT
                                    | crate::auras::handlers::MOD_POWER_COST_SCHOOL_PCT
                            )
                    })
                    .map(move |effect| {
                        (
                            aura.spell_id,
                            aura.stacks,
                            effect.index,
                            effect.aura_type,
                            effect.amount,
                            effect.misc_values,
                        )
                    })
            })
            .collect::<Vec<_>>();
        let result = literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                30451,
                run.targets[0],
                Some(run.targets[0]),
            ),
        );
        assert!(
            result.is_ok(),
            "native Blast at{charges}charges must admit one authored cost{budget}, not its square: {result:?}; zero_charge_cost={base}, base_mana={base_mana}, actual_costs={actual_costs:?}, modifiers={modifiers:?}"
        );
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
