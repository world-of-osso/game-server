//! Selected native replacements must make only the replaced foreground actions unready.
use super::*;
use crate::auras::ActiveAuras;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use server::simc_apl::Facts;

fn prepare_selected_actor() -> Run {
    let mut run = prepare_literal(263, 1).unwrap();
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    for spell in [470057, 114051] {
        assert!(
            known.contains(spell),
            "the full frozen profile must select native spell{spell}"
        );
    }
    engage(&mut run);
    run
}

fn assert_unready(run: &mut Run, spell_id: u32, target: Entity) {
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .cloned()
        .unwrap();
    // This is the same read-only admission API used by literal action preparation.
    let result =
        crate::spell_cast::read_native_cast_start(run.app.world_mut(), run.actor, &spell, target);
    assert!(
        matches!(result, Err(crate::spell_cast::CastFailure::CasterAuraState)),
        "replaced foreground spell{spell_id} must be unready: {result:?}"
    );
    let bindings = Bindings {
        actions: [("candidate".into(), spell_id)].into(),
        global_cooldown_spell: spell_id,
        ..Default::default()
    };
    let facts = NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        1,
    );
    assert_eq!(facts.value("action.candidate.ready").unwrap(), 0.0);
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
fn enhancement_admission_selected_voltaic_makes_flame_shock_unready_but_keeps_its_real_dot() {
    // SimC76f8118 sc_shaman.cpp8463–8470: selected Voltaic makes normal Flame Shock unready.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_selected_actor();
    let target = run.targets[0];
    assert_unready(&mut run, 188389, target);
    enhancement_generation_tests::cast(&mut run, 470057, target);
    assert!(
        owns_aura(&run, target, 188389),
        "real Voltaic must still apply its caster-owned native Flame Shock in the background"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_admission_selected_ascendance_makes_doom_unready_but_keeps_its_real_mode() {
    // SimC76f8118 sc_shaman.cpp8860–8868: normal Doom is unready with Ascendance/DRE selected.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_selected_actor();
    let actor = run.actor;
    assert_unready(&mut run, 384352, actor);
    enhancement_generation_tests::cast(&mut run, 114051, actor);
    for spell in [114051, 466772, 384437] {
        assert!(
            owns_aura(&run, actor, spell),
            "real Ascendance must still produce native mode/Static aura{spell}"
        );
    }
}
