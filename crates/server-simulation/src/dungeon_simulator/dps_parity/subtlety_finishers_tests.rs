//! Source-backed Find Weakness and Shadowed Finishers use real native casts.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

fn cast(run: &mut Run, spell: u32) {
    {
        let mut pools = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        let energy = pools.get_mut(PowerType::Energy).unwrap();
        energy.current = energy.max;
        let combo = pools.get_mut(PowerType::ComboPoints).unwrap();
        combo.current = combo.max;
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
}

fn apply_owned(run: &mut Run, spell: u32) {
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.actor,
            target: run.actor,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_finishers_shadowstrike_earns_native_find_weakness() {
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    apply_owned(&mut run, 185313);
    cast(&mut run, 185438);
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| event.spell_id == Some(185438)
            && event.kind == CombatLogKind::Damage
            && event.amount > 0),
        "real Shadowstrike landed"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1264521),
        "landed native Shadowstrike grants the caster's Find Weakness"
    );
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_finishers_native_shadow_payloads_hit_each_recipient_once() {
    for (parent, child, targets) in [(196819, 328082, 1), (319175, 319190, 3)] {
        let mut run = prepare_literal(261, targets).unwrap();
        engage(&mut run);
        apply_owned(&mut run, 1264521);
        cast(&mut run, parent);
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for target in &run.targets {
            let hits: Vec<_> = events
                .iter()
                .filter(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(child)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                })
                .collect();
            assert_eq!(
                hits.len(),
                1,
                "native parent {parent} must dispatch one {child} hit per recipient"
            );
        }
    }
}
