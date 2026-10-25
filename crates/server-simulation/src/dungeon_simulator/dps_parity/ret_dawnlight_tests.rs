//! Native Dawnlight producer boundary; no synthetic budget or power grants.
use super::ret_purpose_tests::current_holy_power;
use super::ret_tier_tests::native_ret;
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const TALENT: u32 = 431377;
const BUDGET: u32 = 431522;
const WAKE: u32 = 255937;
const JUDGMENT: u32 = 20271;

fn budget(run: &Run) -> Option<&crate::auras::AuraInstance> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| aura.spell_id == BUDGET && aura.caster == run.actor)
}

fn cast_and_damage(run: &mut Run, spell: u32) {
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
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
        {
            return;
        }
    }
    panic!("ordinary native spell{spell} never damaged the actual enemy");
}

fn native_count(run: &Run) -> f32 {
    let world = run.app.world();
    let selected = world
        .get::<crate::class_progression::TraitLoadout>(run.actor)
        .unwrap();
    assert!(
        selected
            .active_talent_spells()
            .any(|spell| spell.spell_id == TALENT)
    );
    let talent = world
        .resource::<crate::spell_info::SpellStore>()
        .get(TALENT)
        .unwrap();
    let raw = talent
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    assert_eq!(raw, 2.0, "unchanged native talent count");
    assert!(talent.labels.contains(&6787));
    let auras = world.get::<crate::auras::ActiveAuras>(run.actor).unwrap();
    let modifier = auras
        .iter()
        .find(|aura| aura.spell_id == 137027)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 25)
        .unwrap();
    assert_eq!(modifier.aura_type, 219);
    assert_eq!(modifier.misc_values, [3, 6787]);
    assert_eq!(modifier.amount, 1.0, "native specialization adds one use");
    let live = auras
        .iter()
        .find(|aura| aura.spell_id == TALENT)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert_eq!(live, raw + modifier.amount);
    assert_eq!(live, 3.0, "native modified count wins over SimC raw2");
    live
}

#[test]
#[ignore = "strict actual selected Wake must arm native Dawnlight uses, not maximum6"]
fn dps_parity_dawnlight_producer_native_wake_arms_modified_count() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    let count = native_count(&run);
    assert_eq!(current_holy_power(&run), 0.0);
    assert!(budget(&run).is_none());
    cast_and_damage(&mut run, WAKE);
    assert_eq!(current_holy_power(&run), 3.0, "actual native Wake Energize");
    let aura = budget(&run).expect("selected actual Wake must arm native431522");
    assert_eq!(f32::from(aura.stacks), count);
    assert_eq!(aura.max_stacks, 6, "retain the native counter maximum");
}

#[test]
#[ignore = "actual Judgment without Wake must not invent a Dawnlight budget"]
fn dps_parity_dawnlight_producer_native_no_wake_keeps_budget_absent() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    native_count(&run);
    cast_and_damage(&mut run, JUDGMENT);
    assert!(budget(&run).is_none());
}

#[test]
#[ignore = "legal unspent Hero points retain Wake but cannot arm Dawnlight"]
fn dps_parity_dawnlight_producer_native_no_hero_keeps_budget_absent() {
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
    inputs::install_traits(run.app.world_mut(), run.actor, &traits).unwrap();
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
