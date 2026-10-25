//! Producer/phase only: no Toll child, echo, stack-spend or ICD qualification.
use super::ret_tier_tests::native_ret;
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const TALENT: u32 = 384027;
const TOLL: u32 = 375576;
const JUDGMENT: u32 = 20271;
const RESONANCE: u32 = 1266308;

fn resonance(run: &Run) -> Option<&crate::auras::AuraInstance> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| aura.spell_id == RESONANCE && aura.caster == run.actor)
}

fn cast(run: &mut Run, spell: u32) {
    assert!(resonance(run).is_none());
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        spell,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::CastSuccess
            }),
        "actual ordinary cast, not a direct aura grant, must own this setup"
    );
}

#[test]
#[ignore = "actual selected Toll produces native Resonance after cast-success and survives first proc update"]
fn dps_parity_ret_resonance_native_new_budget_survives_its_own_toll_cast() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|spell| spell.spell_id == TALENT)
    );
    let native = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(RESONANCE)
        .unwrap();
    let maximum = native.aura_options.as_ref().unwrap().cumulative_aura;
    let duration = native
        .misc
        .as_ref()
        .unwrap()
        .duration
        .as_ref()
        .unwrap()
        .duration_ms;
    cast(&mut run, TOLL);
    let aura = resonance(&run).expect("actual selected Toll must produce native Resonance");
    assert_eq!(u32::from(aura.stacks), maximum);
    assert_eq!(aura.stacks, aura.max_stacks);
    assert_eq!(i64::from(aura.duration_ms), i64::from(duration));
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let cast_index = events
        .iter()
        .position(|event| event.spell_id == Some(TOLL) && event.kind == CombatLogKind::CastSuccess)
        .unwrap();
    let apply_index = events
        .iter()
        .position(|event| {
            event.spell_id == Some(RESONANCE) && event.kind == CombatLogKind::AuraApplied
        })
        .unwrap();
    assert!(
        cast_index < apply_index,
        "source AfterCast order must precede the new aura application"
    );
    for _ in 0..4 {
        run.app.update();
    }
    let retained =
        resonance(&run).expect("new native Resonance must not be consumed by its own Toll cast");
    assert_eq!(u32::from(retained.stacks), maximum);
    assert_eq!(retained.stacks, retained.max_stacks);
}

#[test]
#[ignore = "selected Resonance cannot gain a budget from ordinary Judgment without Toll"]
fn dps_parity_ret_resonance_phase_ordinary_no_toll_keeps_budget_absent() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    cast(&mut run, JUDGMENT);
    for _ in 0..4 {
        run.app.update();
    }
    assert!(resonance(&run).is_none());
}

#[test]
#[ignore = "actual native alternate choice379391 preserves legal point spend and cannot grant Resonance"]
fn dps_parity_ret_resonance_phase_other_choice_keeps_budget_absent() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    let (_, mut traits) = read_reference(70).unwrap();
    let choice = traits
        .iter_mut()
        .find(|entry| entry.entry_id == 115468)
        .unwrap();
    assert_eq!(choice.node_id, 93181);
    choice.entry_id = 115467;
    inputs::install_traits(run.app.world_mut(), run.actor, &traits)
        .expect("actual alternate choice379391 must preserve legal native spend");
    for _ in 0..4 {
        run.app.update();
    }
    assert_eq!(
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, TOLL).unwrap(),
        Some(TOLL)
    );
    cast(&mut run, TOLL);
    for _ in 0..4 {
        run.app.update();
    }
    assert!(resonance(&run).is_none());
}
