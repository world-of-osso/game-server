//! Selected native replacements must reject only the replaced foreground actions.
use super::*;
use crate::auras::ActiveAuras;

fn prepare_selected_actor() -> Run {
    let mut run = prepare_literal(263, 1).unwrap();
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    for spell in [470057, 384352, 114051] {
        assert!(
            known.contains(spell),
            "the full frozen profile must select native spell{spell}"
        );
    }
    engage(&mut run);
    run
}

fn owns_aura(run: &Run, target: Entity, spell: u32) -> bool {
    run.app
        .world()
        .get::<ActiveAuras>(target)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == spell && aura.caster == run.actor)
        })
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_admission_selected_voltaic_rejects_normal_flame_shock_but_keeps_its_real_dot() {
    // SimC76f8118 sc_shaman.cpp8463–8470: selected Voltaic makes normal Flame Shock unready.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_selected_actor();
    let target = run.targets[0];
    let result = crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        188389,
        Some(target),
        false,
    );
    assert!(
        matches!(result, Err(crate::spell_cast::CastFailure::CasterAuraState)),
        "selected native Voltaic must reject replaced foreground Flame Shock: {result:?}"
    );
    enhancement_generation_tests::cast(&mut run, 470057, target);
    assert!(
        owns_aura(&run, target, 188389),
        "real Voltaic must still apply its caster-owned native Flame Shock in the background"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_admission_selected_ascendance_rejects_normal_doom_but_keeps_its_real_mode() {
    // SimC76f8118 sc_shaman.cpp8860–8868: normal Doom is unready with Ascendance/DRE selected.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_selected_actor();
    let actor = run.actor;
    let result =
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 384352, Some(actor), false);
    assert!(
        matches!(result, Err(crate::spell_cast::CastFailure::CasterAuraState)),
        "selected native Ascendance must reject the replaced normal Doom button: {result:?}"
    );
    enhancement_generation_tests::cast(&mut run, 114051, actor);
    for spell in [114051, 466772, 384437] {
        assert!(
            owns_aura(&run, actor, spell),
            "real Ascendance must still produce native mode/Static aura{spell}"
        );
    }
}
