//! Shared Agony emits one real secondary application inside its native radius.
use super::*;
use shared::components::Position;

const SHARED_AGONY: u32 = 1259825;
const AGONY: u32 = 980;

fn owned_agony(run: &Run, target: Entity) -> bool {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.caster == run.actor && aura.spell_id == AGONY)
        })
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_shared_agony_real_root_applies_one_native_secondary_without_recursion() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 3).unwrap();
    engage(&mut run);
    let (caster, primary, secondary, outside) =
        (run.actor, run.targets[0], run.targets[1], run.targets[2]);
    assert!(crate::auras::has_aura(
        run.app.world(),
        caster,
        SHARED_AGONY
    ));
    let radius =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), caster, SHARED_AGONY, 1);
    assert!(radius > 0.0);
    let origin = *run.app.world().get::<Position>(primary).unwrap();
    for (target, distance) in [(secondary, radius / 2.0), (outside, radius + 10.0)] {
        let mut position = run.app.world_mut().get_mut::<Position>(target).unwrap();
        position.x = origin.x + distance;
        position.y = origin.y;
        position.z = origin.z;
    }
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: outside,
            target: secondary,
            spell_id: AGONY,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, AGONY, Some(primary), false)
        .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert!(
        owned_agony(&run, primary),
        "actual native root must apply its Agony"
    );
    // SimC76f8118 actions1419–1450: one smart secondary, native radius,
    // no extra cost or recursively spawned twin; foreign Agony is not owned.
    assert!(
        owned_agony(&run, secondary),
        "selected Shared Agony must earn its actual secondary aura"
    );
    assert!(
        !owned_agony(&run, outside),
        "outside-radius recipient cannot be a twin"
    );
}
