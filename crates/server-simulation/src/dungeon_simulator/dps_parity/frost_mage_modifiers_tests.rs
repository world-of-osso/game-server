use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn shatter_critical_damage(deep_shatter: bool) -> i32 {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 1247025,
    );
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 100.0;
    if deep_shatter {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: 1247025,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    let mut payload = crate::spell_cast::load_triggered_spell(run.app.world(), 1246949, 1).unwrap();
    payload.cast_damage_multiplier = Some(1.0);
    let _damage_seed = crate::random::seed_scope(6401);
    assert!(crate::spell_cast::launch_triggered_spell(
        run.app.world_mut(),
        run.actor,
        &payload,
        Some(run.targets[0]),
        1
    ));
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let hit = events
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1246949)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .expect("native Shatter primary hit");
    assert!(
        hit.crit,
        "both native controls must actually critically hit"
    );
    hit.amount
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_deep_shatter_increases_actual_critical_shatter_damage() {
    let plain = shatter_critical_damage(false);
    let deep = shatter_critical_damage(true);
    assert!(
        deep > plain,
        "native selected Deep Shatter must increase the real critical payload: plain{plain}, deep{deep}"
    );
}

fn flurry_payload_damage(brain_freeze: bool) -> (i64, usize) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 190446,
    );
    if brain_freeze {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: 190446,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    let _damage_seed = crate::random::seed_scope(6402);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(44614, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 190446),
        "the live Brain Freeze is spent before delayed Flurry payloads"
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let hits: Vec<_> = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(228354)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .collect();
    assert!(
        !hits.is_empty(),
        "real native Flurry damage must land in both controls"
    );
    (
        hits.iter().map(|event| i64::from(event.amount)).sum(),
        hits.len(),
    )
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_brain_freeze_flurry_retains_damage_after_proc_consumption() {
    let (plain, plain_count) = flurry_payload_damage(false);
    let (earned, earned_count) = flurry_payload_damage(true);
    assert_eq!(
        plain_count, earned_count,
        "Brain Freeze modifies payloads, not native bolt count"
    );
    assert!(
        earned > plain,
        "native earned Brain Freeze must modify its delayed payloads after consumption: plain{plain}, earned{earned}"
    );
}
