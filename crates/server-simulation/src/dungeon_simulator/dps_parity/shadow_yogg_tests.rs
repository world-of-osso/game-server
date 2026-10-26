//! SimC76f8118 sc_priest_shadow.cpp2206–2214/2628–2632/2850–2858.
//! Actual Slam cohorts fund the native counter and actual Thing From Beyond actor.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::{summon::SummonedCreature, test_harness::ObservedCombatEvents};
use shared::components::Npc;
use shared::protocol::CombatLogKind;

fn advance(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        run.app.update();
    }
}

fn slam(run: &mut Run) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(1227280, run.targets[0], None),
    )
    .expect("ordinary native Slam cohort");
}

fn counter(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 373276 && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_yogg_real_three_vt_cohort_earns_one_counter_stack() {
    let mut run = prepare_literal(258, 3).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 373273));
    assert_eq!(counter(&run), 0);
    slam(&mut run);
    advance(&mut run, 80);
    let apparitions = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            let apparition = matches!(event.spell_id, Some(413231 | 1264176));
            apparition
                && event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(apparitions, 3, "one real cohort reaches all three own VTs");
    assert_eq!(
        counter(&run),
        1,
        "Yogg advances once per source cohort, not per projectile"
    );
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_yogg_real_native_cohort_threshold_summons_owned_thing_and_actual_spikes() {
    let mut run = prepare_literal(258, 3).unwrap();
    // The normal measurement engagement selects the player's real enemy.
    // Combat references alone do not give a native guardian an owner target.
    engage(&mut run);
    let cap = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(373276)
        .unwrap()
        .aura_options
        .as_ref()
        .unwrap()
        .cumulative_aura;
    assert_eq!(cap, 20, "native Yogg counter threshold");
    let recovery_ms = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1227280)
        .unwrap()
        .categories
        .as_ref()
        .unwrap()
        .charge_recovery_ms;
    assert!(recovery_ms > 0);
    let recovery_frames = (recovery_ms as usize).div_ceil(50);
    // Real native charge cadence keeps combat/VT live, unlike45s idle gaps.
    for cohort in 1..cap {
        slam(&mut run);
        advance(&mut run, recovery_frames);
        assert_eq!(
            counter(&run),
            cohort as u8,
            "each actual cohort retains one earned counter stack during continuous combat"
        );
    }
    slam(&mut run);
    advance(&mut run, 120);
    let actor = run.actor;
    let things = run
        .app
        .world_mut()
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(run.app.world())
        .filter(|(_, npc, summon)| {
            npc.template_id == 189988 && summon.owner == actor && summon.spell_id == 373277
        })
        .map(|(entity, _, _)| entity.to_bits())
        .collect::<Vec<_>>();
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let summon_events = events
        .iter()
        .filter(|event| event.spell_id == Some(373277))
        .map(|event| (event.kind, event.amount))
        .collect::<Vec<_>>();
    assert!(
        !things.is_empty(),
        "actual threshold earns a native owned Thing From Beyond, not caster damage; counter={}, summon_events={summon_events:?}",
        counter(&run)
    );
    for &target in &run.targets {
        assert!(
            events.iter().any(|event| {
                let owned = event.source.is_some_and(|source| things.contains(&source));
                owned
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(373279)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
            "real Thing casts native Void Spike at each nearby enemy"
        );
        let bursts = events
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1231479)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        assert_eq!(
            bursts, cap as usize,
            "only ordinary parent cohorts Shadeburst, never the parentless Yogg bonus"
        );
    }
}
