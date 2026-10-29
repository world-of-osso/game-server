//! Lunar Beam earns timed native self-heals from its real cast.
use super::*;
use shared::{components::Health, protocol::CombatLogKind};

#[test]
#[ignore = "requires Guardian MID2 profile and native world data"]
fn guardian_lunar_beam_earns_caster_owned_atmospheric_exposure() {
    use crate::auras::ActiveAuras;
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 429532 && aura.caster == actor)
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 204066, Some(target), false)
        .unwrap();
    for _ in 0..25 {
        run.app.update();
    }
    let debuff = run
        .app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 430589 && aura.caster == actor)
        .expect("Lunar Beam damage must apply its own exposure");
    let amount = debuff
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert_eq!(amount, 6.0);
}

#[test]
#[ignore = "requires Guardian MID2 profile and native world data"]
fn guardian_lunar_beam_pulse_respects_native_five_target_cap() {
    use shared::components::Position;
    let mut run = prepare_registered_player(104, 7).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    let origin = *run.app.world().get::<Position>(actor).unwrap();
    for &enemy in &run.targets {
        let mut position = run.app.world_mut().get_mut::<Position>(enemy).unwrap();
        *position = origin;
        position.x += 2.0;
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 204066, Some(target), false)
        .unwrap();
    for _ in 0..25 {
        run.app.update();
    }
    let victims: std::collections::BTreeSet<_> = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(414613)
                && event.source == Some(actor.to_bits())
                && event.amount > 0
        })
        .filter_map(|event| event.target)
        .collect();
    assert_eq!(
        victims.len(),
        5,
        "one pulse must hit five of seven nearby enemies"
    );
}

#[test]
#[ignore = "requires Guardian MID2 profile and native world data"]
fn guardian_lunar_beam_damages_enemies_in_the_original_ground_area() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let count = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(414613)
                    && event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.amount > 0
            })
            .count()
    };
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 204066, Some(target), false)
        .unwrap();
    for _ in 0..45 {
        run.app.update();
    }
    let near = count(&run);
    assert_eq!(near, 2, "two ground pulses must damage the nearby enemy");
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let damage_index = events
        .iter()
        .position(|event| {
            event.spell_id == Some(414613)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(target.to_bits())
        })
        .unwrap();
    let heal_index = events
        .iter()
        .position(|event| {
            event.spell_id == Some(204069)
                && event.kind == CombatLogKind::Heal
                && event.target == Some(actor.to_bits())
        })
        .unwrap();
    assert!(
        damage_index < heal_index,
        "the pulse damages before its self-heal"
    );
    run.app
        .world_mut()
        .get_mut::<shared::components::Position>(target)
        .unwrap()
        .x += 100.0;
    for _ in 0..45 {
        run.app.update();
    }
    assert_eq!(
        count(&run),
        near,
        "ground pulses must not follow the enemy out of the cast area"
    );
}

#[test]
#[ignore = "requires Guardian MID2 profile and native world data"]
fn guardian_lunar_beam_heals_on_each_ground_pulse() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    {
        let mut h = run.app.world_mut().get_mut::<Health>(actor).unwrap();
        h.current = h.max * 0.1;
    }
    let count = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|e| {
                e.kind == CombatLogKind::Heal
                    && e.spell_id == Some(204069)
                    && e.source == Some(actor.to_bits())
                    && e.target == Some(actor.to_bits())
            })
            .count()
    };
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 204066, Some(target), false)
        .unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    assert_eq!(count(&run), 0);
    for expected in 1..=11 {
        for _ in 0..20 {
            run.app.update();
        }
        assert_eq!(count(&run), expected);
    }
    for _ in 0..20 {
        run.app.update();
    }
    assert_eq!(count(&run), 11);
}
