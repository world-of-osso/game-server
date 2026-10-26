//! Paid Incinerate with actual selected FnB and native Havoc marks.
use super::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason};
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn selected_run(fnb: bool, havoc: bool) -> Run {
    let (profile, mut traits) = read_reference(267).unwrap();
    if fnb {
        traits.push(inputs::AllocatedTrait {
            node_id: 71982,
            entry_id: 91499,
            ranks: 1,
            active_ranks: 1,
        });
    }
    if havoc {
        traits.push(inputs::AllocatedTrait {
            node_id: 71979,
            entry_id: 91493,
            ranks: 1,
            active_ranks: 1,
        });
    }
    let identity = inputs::read_actor(&profile).unwrap();
    let mut run = prepare_literal_reference(profile, &traits, 4, identity).unwrap();
    let actor = run.actor;
    let active = run.app.world().get::<TraitLoadout>(actor).unwrap();
    assert_eq!(
        active
            .active_talent_spells()
            .any(|talent| talent.spell_id == 196408),
        fnb
    );
    assert_eq!(
        active
            .active_talent_spells()
            .any(|talent| talent.spell_id == 80240),
        havoc
    );
    engage(&mut run);
    // The native crit-only control makes each recipient comparable; remove the
    // separately qualified random mastery rather than modifying any coefficient.
    crate::auras::remove_auras(run.app.world_mut(), actor, RemoveReason::Consumed, |aura| {
        aura.spell_id == 77220
    });
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 75060,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    run.app.update();
    if havoc {
        let target = run.targets[1];
        literal_rotation::execution::execute(
            run.app.world_mut(),
            actor,
            literal_rotation::execution::Execution::Spell(80240, target, Some(target)),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        assert!(
            crate::auras::has_aura(run.app.world(), target, 80240),
            "actual paid Havoc must land"
        );
    }
    run
}

fn assert_selected_recipients(fnb: bool, havoc: bool) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = selected_run(fnb, havoc);
    let actor = run.actor;
    let targets = run.targets.clone();
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(29722, targets[0], Some(targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..60 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let amounts: Vec<Vec<f64>> = targets
        .iter()
        .map(|target| {
            events
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(29722)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                })
                .map(|event| f64::from(event.amount))
                .collect()
        })
        .collect();
    assert_eq!(amounts[0].len(), 1, "one paid primary impact");
    let store = run.app.world().resource::<crate::spell_info::SpellStore>();
    let fraction = |spell| {
        f64::from(
            store
                .get(spell)
                .unwrap()
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .unwrap()
                .base_points,
        ) / 100.0
    };
    let fnb_fraction = if fnb { fraction(196408) } else { 0.0 };
    for index in 1..targets.len() {
        let expected = fnb_fraction
            + if havoc && index == 1 {
                fraction(80240)
            } else {
                0.0
            };
        if expected == 0.0 {
            assert!(
                amounts[index].is_empty(),
                "no selected source for secondary{index}"
            );
            continue;
        }
        assert_eq!(
            amounts[index].len(),
            1,
            "one source-qualified impact on secondary{index}"
        );
        let actual = amounts[index][0] / amounts[0][0];
        // Native5% variance is independently rolled per hit; +/-12% relative
        // admits both extrema without fitting native damage values.
        assert!(
            (actual / expected - 1.0).abs() <= 0.12,
            "secondary{index} ratio{actual} must match native fraction{expected}"
        );
    }
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_incinerate_selected_fnb_hits_uncapped_pack_at_native_fraction() {
    assert_selected_recipients(true, false);
}
#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_incinerate_selected_havoc_without_fnb_hits_one_mark_at_native_fraction() {
    assert_selected_recipients(false, true);
}
#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_incinerate_selected_havoc_and_fnb_combine_without_duplicate_hits() {
    assert_selected_recipients(true, true);
}
