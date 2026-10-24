//! Post-tp1034 witnesses using native selected profiles and actual passive effects.
use super::*;
use crate::auras::ActiveAuras;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use server::simc_apl::Facts;

fn assert_selected_rank_two(run: &Run, spell: u32) {
    let bindings = Bindings {
        talents: [("observed".into(), spell)].into(),
        ..Default::default()
    };
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let facts = NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1);
    assert_eq!(facts.value("talent.observed.rank").unwrap(), 2.0);
}

fn passive_amount(run: &Run, spell: u32, index: u8) -> f32 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
        .expect("actual selected native passive applied")
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap()
        .amount
}

#[test]
#[ignore = "requires prepared1467 reference inputs and current canonical trait-points data"]
fn native_trait_recheck_dev_arcane_intensity_rank_two_has_sixteen_percent() {
    let run = prepare(1467, 1).unwrap();
    const ARCANE_INTENSITY: u32 = 375618;
    assert_selected_rank_two(&run, ARCANE_INTENSITY);
    // Definition92648/120592, Set curves54429/64806: native rank2 Y=16.
    // Only effect0 has a TraitDefinitionEffectPoints row, not all spell effects.
    assert_eq!(passive_amount(&run, ARCANE_INTENSITY, 0), 16.0);
    assert_eq!(passive_amount(&run, ARCANE_INTENSITY, 1), 8.0);
}

#[test]
#[ignore = "requires DPS_PARITY_FERAL_INPUTS plus current canonical trait-points data"]
fn native_trait_recheck_feral_tireless_energy_rank_two_has_one_hundred_forty_energy() {
    let directory = std::path::PathBuf::from(
        std::env::var_os("DPS_PARITY_FERAL_INPUTS")
            .expect("DPS_PARITY_FERAL_INPUTS must name the prepared103 reference directory"),
    );
    let profile: inputs::Profile = inputs::read_json(&directory.join("103.profile.json")).unwrap();
    assert_eq!(profile.spec_id, 103);
    let traits: Vec<inputs::AllocatedTrait> =
        inputs::read_json(&directory.join("103.selected-traits-active.json")).unwrap();
    let run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    const TIRELESS_ENERGY: u32 = 383352;
    assert_selected_rank_two(&run, TIRELESS_ENERGY);
    // Definition108190, Multiply curve58943 rank2 Y=2: native20 =>40 Energy.
    assert_eq!(passive_amount(&run, TIRELESS_ENERGY, 0), 40.0);
    let (_, maximum) = crate::spell_cast::powers::power_value(
        run.app.world(),
        run.actor,
        shared::components::PowerType::Energy,
    )
    .unwrap();
    assert_eq!(
        maximum, 140.0,
        "actual native100 Energy plus the learned40-point passive"
    );
}
