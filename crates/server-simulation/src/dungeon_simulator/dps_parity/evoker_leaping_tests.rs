//! Real Fire Breath earns Leaping Flames for the next Living Flame.
use super::*;

#[test]
#[ignore = "requires exact native Devastation profile and DB"]
fn evoker_leaping_actual_fire_breath_grants_rank_and_living_flame_hits_pack() {
    for selected in [false, true] {
        let mut run = prepare_literal(1467, 3).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let primary = run.targets[0];
        assert!(crate::auras::has_aura(run.app.world(), actor, 369939));
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Unlearned,
                |a| a.spell_id == 369939,
            );
        }
        crate::spell_cast::creature_cast_empowered_with_targets(
            run.app.world_mut(),
            actor,
            357208,
            crate::spell_cast::CastTargets {
                unit: actor,
                primary: Some(primary),
                destination: None,
            },
            2,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        let stacks = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .and_then(|a| a.iter().find(|a| a.spell_id == 370901 && a.caster == actor))
            .map_or(0, |a| a.stacks);
        assert_eq!(
            stacks,
            if selected { 2 } else { 0 },
            "completed Fire Breath earns its rank in native Leaping Flames stacks"
        );
        let before = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            actor,
            361469,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: Some(primary),
                destination: None,
            },
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..100 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[before..];
        let hit_count = run
            .targets
            .iter()
            .filter(|target| {
                events.iter().any(|e| {
                    e.source == Some(actor.to_bits())
                        && e.target == Some(target.to_bits())
                        && e.spell_id == Some(361500)
                        && e.kind == shared::protocol::CombatLogKind::Damage
                        && e.amount > 0
                })
            })
            .count();
        assert_eq!(
            hit_count,
            if selected { 3 } else { 1 },
            "earned rank-two Leaping Flames hits two additional real targets"
        );
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 370901),
            "Living Flame consumes the earned aura"
        );
    }
}
