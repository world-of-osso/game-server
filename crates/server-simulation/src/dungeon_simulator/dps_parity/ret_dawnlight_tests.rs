//! Selected431377/native count, not a guessed grant from distinct passive431460.
use super::ret_purpose_tests::current_holy_power;
use super::ret_tier_tests::native_ret;
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const TALENT: u32 = 431377;
const BUDGET: u32 = 431522;
const OTHER_PASSIVE: u32 = 431460;
const WAKE: u32 = 255937;
const JUDGMENT: u32 = 20271;
const RETRIBUTION: u32 = 137027;
const DAWNLIGHT_LABEL: u32 = 6787;

fn budget(run: &Run) -> Option<&crate::auras::AuraInstance> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| aura.spell_id == BUDGET && aura.caster == run.actor)
}

fn cast_and_damage(run: &mut Run, spell: u32) {
    assert!(budget(run).is_none());
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
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
        {
            return;
        }
    }
    panic!("actual ordinary native spell{spell} never damaged the real target");
}

#[test]
#[ignore = "actual selected Dawnlight431377 Wake must arm native431522 with actual talent count, not max6"]
fn dps_parity_ret_dawnlight_producer_native_wake_arms_native_talent_count() {
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
    let count = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == TALENT)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    eprintln!(
        "Dawnlight actual selected talent known={}, distinct431460 known={} active={}",
        known.contains(TALENT),
        known.contains(OTHER_PASSIVE),
        crate::auras::has_aura(run.app.world(), run.actor, OTHER_PASSIVE)
    );
    let talent = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(TALENT)
        .unwrap();
    let base_count = talent
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    assert_eq!(base_count, 2.0, "unchanged native431377 effect0");
    assert!(talent.labels.contains(&DAWNLIGHT_LABEL));
    let modifier = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == RETRIBUTION)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 25)
        .unwrap();
    assert_eq!(modifier.aura_type, 219, "native flat spell-label modifier");
    assert_eq!(modifier.misc_values, [3, DAWNLIGHT_LABEL as i32]);
    assert_eq!(modifier.amount, 1.0, "native specialization adds one use");
    assert_eq!(
        count,
        base_count + modifier.amount,
        "native modified count, not max6"
    );
    assert_eq!(current_holy_power(&run), 0.0);
    cast_and_damage(&mut run, WAKE);
    assert_eq!(
        current_holy_power(&run),
        3.0,
        "actual unchanged native Wake Energize effect1"
    );
    let aura =
        budget(&run).expect("actual selected Wake must produce native Dawnlight431522 budget");
    assert_eq!(f32::from(aura.stacks), count);
}

#[test]
#[ignore = "ordinary Judgment without Wake must not manufacture selected Dawnlight budget"]
fn dps_parity_ret_dawnlight_producer_native_no_wake_keeps_budget_absent() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    cast_and_damage(&mut run, JUDGMENT);
    assert!(budget(&run).is_none());
}

#[test]
#[ignore = "legal no-Hero config retains selected class/spec Wake but cannot arm Dawnlight"]
fn dps_parity_ret_dawnlight_producer_native_no_hero_keeps_budget_absent() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    let (_, mut traits) = read_reference(70).unwrap();
    let classes = run.app.world().resource::<crate::spell_info::ClassSpells>();
    let tree = &classes
        .classes
        .iter()
        .find(|class| class.class_id == 2)
        .unwrap()
        .traits;
    traits.retain(|entry| {
        let node = tree.node(entry.node_id).unwrap();
        node.sub_tree_id == 0 && node.kind != crate::spell_info::TraitNodeType::SubTreeSelection
    });
    inputs::install_traits(run.app.world_mut(), run.actor, &traits).expect(
        "native validator must accept unspent Hero points with unchanged class/spec purchases",
    );
    for _ in 0..4 {
        run.app.update();
    }
    assert!(
        !run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(TALENT)
    );
    assert_eq!(
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, WAKE).unwrap(),
        Some(WAKE)
    );
    cast_and_damage(&mut run, WAKE);
    assert!(budget(&run).is_none());
}
