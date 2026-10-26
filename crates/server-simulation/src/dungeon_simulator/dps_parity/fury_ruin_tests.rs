//! Real Rampage must not invent an unselected Rampaging Ruin payload.
use super::literal_rotation::execution::{Execution, execute};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "private current native DB and exact spec72 player inputs"]
fn fury_rampage_requires_selected_rampaging_ruin_even_with_whirlwind() {
    for whirlwind in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_measurement(72, 3).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let primary = run.targets[0];
        let selected = run
            .app
            .world()
            .get::<crate::class_progression::TraitLoadout>(actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 1265357);
        assert!(
            !selected,
            "the exact MID2 Fury profile does not allocate Rampaging Ruin"
        );
        assert!(!crate::auras::has_aura(run.app.world(), actor, 85739));
        if whirlwind {
            crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 85739, Some(actor), 1);
            assert!(crate::auras::has_aura(run.app.world(), actor, 85739));
        }
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            PowerType::Rage,
            1000.0,
        )
        .unwrap();
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(184367, primary, Some(primary)),
        )
        .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let damage: Vec<_> = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .collect();
        for weapon_hit in [184707, 184709, 201364, 201363] {
            assert!(
                damage
                    .iter()
                    .any(|event| event.spell_id == Some(weapon_hit)),
                "the real Rampage sequence must retain native weapon hit{weapon_hit}"
            );
        }
        assert!(
            damage.iter().all(|event| event.spell_id != Some(1299944)),
            "an unselected Rampaging Ruin cannot follow real Rampage: whirlwind={whirlwind}, damage={damage:?}"
        );
    }
}
