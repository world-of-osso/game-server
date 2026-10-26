//! Actual Incinerate impacts fund native shards without replacing the base grant.
use super::*;
use crate::auras::{AuraApplication, AuraSource};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

const INCINERATE: u32 = 29722;
const ENERGIZE: u32 = 244670;
const EMBERS: u32 = 387173;
const EMBERS_ENTRY: u32 = 91481;

fn prepare_incinerate(selected_embers: bool, critical: bool) -> Run {
    let (profile, mut traits) = read_reference(267).unwrap();
    if !selected_embers {
        traits.retain(|entry| entry.entry_id != EMBERS_ENTRY);
    }
    let identity = inputs::read_actor(&profile).unwrap();
    let mut run = prepare_literal_reference(profile, &traits, 1, identity).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let spell_id = if critical { 75060 } else { 171884 };
    let mask = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap()
        .effects
        .iter()
        .filter(|effect| effect.effect == 6)
        .fold(0, |mask, effect| mask | (1 << effect.index));
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id,
            effect_mask: mask,
            source: AuraSource::Cast,
        },
    );
    run.app.update();
    let selected = run
        .app
        .world()
        .get::<crate::class_progression::TraitLoadout>(actor)
        .unwrap()
        .active_talent_spells()
        .any(|talent| talent.spell_id == EMBERS);
    assert_eq!(selected, selected_embers);
    run
}

fn expected_native_gain(run: &Run, selected_embers: bool, critical: bool) -> f32 {
    let store = run.app.world().resource::<crate::spell_info::SpellStore>();
    let base = store
        .get(ENERGIZE)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    let bonus = if selected_embers {
        store
            .get(EMBERS)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .base_points
            / 100.0
    } else {
        0.0
    };
    let divisor = run
        .app
        .world()
        .resource::<crate::spell_cast::powers::PowerTypeTable>()
        .get(PowerType::SoulShards)
        .unwrap()
        .display_modifier;
    let fragment = if critical { 0.1 * divisor as f32 } else { 0.0 };
    (base + fragment) * (1.0 + bonus)
}

fn assert_incinerate_gain(selected_embers: bool, critical: bool) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_incinerate(selected_embers, critical);
    let expected = expected_native_gain(&run, selected_embers, critical);
    let before =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::SoulShards)
            .unwrap()
            .0;
    let (actor, target) = (run.actor, run.targets[0]);
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(INCINERATE, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    let hit = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(INCINERATE)
                && event.kind == CombatLogKind::Damage
        })
        .expect("completed real native Incinerate must damage its selected primary");
    assert!(hit.amount > 0);
    assert_eq!(
        hit.crit, critical,
        "native critical outcome is the prerequisite"
    );
    let after =
        crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::SoulShards)
            .unwrap()
            .0;
    assert_eq!(
        after - before,
        expected,
        "selectedEmbers={selected_embers}, critical={critical}: shared raw pool before{before}, after{after}, native expected{expected}"
    );
}

#[test]
#[ignore = "requires private267 native inputs"]
fn incinerate_selected_embers_scales_actual_primary_base_funding() {
    assert_incinerate_gain(true, false);
}

#[test]
#[ignore = "requires private267 native inputs"]
fn incinerate_selected_embers_scales_actual_critical_fragment() {
    assert_incinerate_gain(true, true);
}

#[test]
#[ignore = "requires private267 native inputs"]
fn incinerate_without_embers_retains_native_noncritical_base_grant() {
    assert_incinerate_gain(false, false);
}

#[test]
#[ignore = "requires private267 native inputs"]
fn incinerate_without_embers_still_earns_actual_critical_fragment() {
    assert_incinerate_gain(false, true);
}
