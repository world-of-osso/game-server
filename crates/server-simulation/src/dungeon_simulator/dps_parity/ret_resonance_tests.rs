//! Selected Ret384027 is next-three-casts Resonance, not the older periodic version.
use super::ret_tier_tests::native_ret;
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const TALENT: u32 = 384027;
const TALENT_ENTRY: u32 = 115468;
const TOLL: u32 = 375576;
const JUDGMENT: u32 = 20271;
const TOLL_JUDGMENT: u32 = 406957;
const RESONANCE: u32 = 1266308;

fn resonance(run: &Run) -> Option<&crate::auras::AuraInstance> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| aura.spell_id == RESONANCE && aura.caster == run.actor)
}

fn selected(run: &Run) -> bool {
    run.app
        .world()
        .get::<crate::class_progression::TraitLoadout>(run.actor)
        .unwrap()
        .active_talent_spells()
        .any(|spell| spell.spell_id == TALENT)
}

fn cast(run: &mut Run, spell: u32, damage_spell: u32) {
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
    for _ in 0..100 {
        run.app.update();
        if run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(damage_spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
        {
            break;
        }
    }
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(damage_spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
        "actual ordinary action{spell} must reach its native damage before checking Resonance"
    );
}

#[test]
#[ignore = "selected384027 actual Toll must grant native1266308 initialized at native max stacks"]
fn dps_parity_ret_resonance_native_selected_toll_grants_maximum_native_budget() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert!(selected(&run));
    let maximum = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(RESONANCE)
        .unwrap()
        .aura_options
        .as_ref()
        .unwrap()
        .cumulative_aura;
    assert_eq!(
        maximum, 3,
        "native budget metadata must be the actual reference version"
    );
    cast(&mut run, TOLL, TOLL_JUDGMENT);
    let aura = resonance(&run).expect("actual selected Toll must produce native Resonance1266308");
    assert_eq!(u32::from(aura.stacks), maximum);
    assert_eq!(aura.stacks, aura.max_stacks);
}

#[test]
#[ignore = "ordinary Judgment without Toll must not manufacture selected Resonance budget"]
fn dps_parity_ret_resonance_native_without_toll_keeps_budget_absent() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert!(selected(&run));
    cast(&mut run, JUDGMENT, JUDGMENT);
    assert!(resonance(&run).is_none());
}

#[test]
#[ignore = "legal other native choice at93181 retains Toll but cannot grant Resonance budget"]
fn dps_parity_ret_resonance_native_other_choice_keeps_budget_absent() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    let (_, mut traits) = read_reference(70).unwrap();
    let choice = traits
        .iter_mut()
        .find(|entry| entry.entry_id == TALENT_ENTRY)
        .unwrap();
    assert_eq!(choice.node_id, 93181);
    // Native node93181's other legal entry is115467 (Quickened Invocation379391).
    // Simply deleting Resonance is invalid: it leaves only22 of23 required earlier-row points.
    choice.entry_id = 115467;
    inputs::install_traits(run.app.world_mut(), run.actor, &traits)
        .expect("native validator must accept the actual alternate choice at the same node");
    for _ in 0..4 {
        run.app.update();
    }
    assert!(!selected(&run));
    assert_eq!(
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, TOLL).unwrap(),
        Some(TOLL)
    );
    cast(&mut run, TOLL, TOLL_JUDGMENT);
    assert!(resonance(&run).is_none());
}
