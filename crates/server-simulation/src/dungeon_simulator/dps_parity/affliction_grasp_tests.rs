//! Real channel impacts must launch native extra-DoT payloads, never foreign DoTs.
use super::*;

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_grasp_actual_ticks_copy_only_owned_native_dots() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 3).unwrap();
    engage(&mut run);
    let caster = run.actor;
    let primary = run.targets[0];
    let foreign = run.targets[1];
    for spell in [980, 1259790, 445474] {
        crate::spell_cast::cast_triggered(run.app.world_mut(), caster, spell, Some(primary), 0);
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.targets[2],
                target: foreign,
                spell_id: spell,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    let (initial_stacks, maximum_stacks) = owned_agony_counter(&run, caster, primary);
    crate::spell_cast::cast_triggered(run.app.world_mut(), caster, 205180, Some(caster), 0);
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, 1261153, Some(primary), false)
        .unwrap();
    for _ in 0..80 {
        run.app.update();
    }
    let (ramped_stacks, _) = owned_agony_counter(&run, caster, primary);
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let ramp_ticks = events
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(primary.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage
                && matches!(event.spell_id, Some(980 | 1261153))
                && event.amount > 0
        })
        .count();
    let expected_stacks =
        (usize::from(initial_stacks) + ramp_ticks).min(usize::from(maximum_stacks)) as u8;
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, 1261153, Some(foreign), false)
        .unwrap();
    for _ in 0..80 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let hits: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
        })
        .collect();
    let primary_ticks = hits
        .iter()
        .filter(|event| event.spell_id == Some(1261153) && event.target == Some(primary.to_bits()))
        .count();
    assert!(
        primary_ticks > 0,
        "actual native Grasp damage is the prerequisite"
    );
    // SimC76f8118 sc_warlock_actions.cpp136–158/2210–2214: one native
    // child for each ticking owned DoT per actual landed Grasp tick.
    for child in [1261166, 1261176, 1279686] {
        let children: Vec<_> = hits
            .iter()
            .filter(|event| event.spell_id == Some(child))
            .collect();
        assert_eq!(
            children.len(),
            primary_ticks,
            "native child {child} must execute once per real Grasp tick"
        );
        assert!(
            children
                .iter()
                .all(|event| event.target == Some(primary.to_bits())),
            "native child {child} cannot copy another caster's DoT"
        );
    }
    assert_eq!(
        ramped_stacks, expected_stacks,
        "landed Agony children must advance the native owned counter as ordinary Agony ticks do"
    );
}

fn owned_agony_counter(run: &Run, caster: Entity, target: Entity) -> (u8, u8) {
    let auras = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap();
    let aura = auras
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == 980)
        .unwrap();
    (aura.stacks, aura.max_stacks)
}
