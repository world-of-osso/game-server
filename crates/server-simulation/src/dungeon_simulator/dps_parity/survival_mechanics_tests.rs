//! Real Survival casts with the exact selected MID2 player traits.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use shared::protocol::CombatLogKind;

fn player() -> Run {
    let (profile, traits) = read_reference(255).unwrap();
    let identity = inputs::read_actor(&profile).unwrap();
    let mut run = prepare_literal_reference(profile, &traits, 1, identity).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    engage(&mut run);
    run
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_tip_stacks_are_uses_not_damage_multipliers() {
    use crate::auras::spell_mods::{self, SpellModOp};
    use crate::auras::{AuraApplication, AuraSource};
    let mut run = player();
    let apply = |run: &mut Run| {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: 260286,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        )
    };
    let modifier = |run: &Run| {
        let spell = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(186270)
            .unwrap();
        spell_mods::modify(
            run.app.world(),
            run.actor,
            spell,
            SpellModOp::HealingAndDamage,
            1.0,
        )
    };
    let baseline = modifier(&run);
    apply(&mut run);
    let one_use = modifier(&run);
    assert!(
        one_use > baseline,
        "native Tip must increase actual Raptor damage"
    );
    apply(&mut run);
    assert_eq!(tip_stacks(&run), 2);
    assert_eq!(
        modifier(&run),
        one_use,
        "sc_hunter.cpp1407–1408 reads one native Tip effect, never stack_value"
    );
}

fn tip_stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == 260286)
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_command_earns_selected_tip_stacks() {
    // SimC76f8118 sc_hunter.cpp6606–6608: base one plus native Primal Surge.
    let mut run = player();
    let primal = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1272154)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points as u8;
    cast(&mut run, 259489, false);
    assert_eq!(
        tip_stacks(&run),
        1 + primal,
        "actual Survival Kill Command must earn native Tip plus selected Primal Surge"
    );
    cast(&mut run, 186270, false);
    assert_eq!(
        tip_stacks(&run),
        primal,
        "a real Raptor impact consumes exactly one earned Tip use"
    );
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_raptor_cast_upgrades_then_spends_swipe() {
    // SimC76f8118 sc_hunter.cpp6217–6236: selected rank3 guarantees Swipe
    // after a normal Strike; the next ordinary root resolves the native override.
    let mut run = player();
    cast(&mut run, 186270, false);
    assert!(
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 1273155 && aura.caster == run.actor),
        "selected rank3 must earn the real Raptor Swipe replacement"
    );
    cast(&mut run, 186270, false);
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1262293)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "the next Raptor request must deal actual native Swipe damage"
    );
    assert!(
        !run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 1273155 && aura.caster == run.actor),
        "Swipe cannot leave its own replacement active"
    );
}
