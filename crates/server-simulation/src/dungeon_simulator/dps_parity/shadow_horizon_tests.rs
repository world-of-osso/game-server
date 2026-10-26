//! SimC76f8118 sc_priest.cpp387–394/4358–4391: actual Void Blasts extend Rift.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::CastFailure;

fn cast_and_finish(run: &mut Run, spell_id: u32) {
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap()
        .clone();
    for _ in 0..400 {
        let ready = crate::spell_cast::read_native_cast_start(
            run.app.world_mut(),
            run.actor,
            &spell,
            run.targets[0],
        );
        match ready {
            Ok(_) => {
                literal_rotation::execution::execute(
                    run.app.world_mut(),
                    run.actor,
                    literal_rotation::execution::Execution::Spell(spell_id, run.targets[0], None),
                )
                .unwrap();
                for _ in 0..100 {
                    if run
                        .app
                        .world()
                        .get::<shared::casting::CastState>(run.actor)
                        .is_none()
                    {
                        return;
                    }
                    run.app.update();
                }
                panic!("native spell{spell_id} did not finish");
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::OnCooldown
                | CastFailure::NoCharges
                | CastFailure::InProgress,
            ) => {
                run.app.update();
            }
            Err(reason) => panic!("ordinary native spell{spell_id}: {reason:?}"),
        }
    }
    panic!("native spell{spell_id} did not become ready");
}

fn rift_expiry(run: &Run) -> f64 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == 459314)
        .expect("real Rift remains active")
        .expires_at
        .unwrap()
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_horizon_ordinary_void_blast_extends_actual_rift_only_to_native_cap() {
    let mut run = prepare_literal(258, 1).unwrap();
    engage_fixture_opponents(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 449912));
    let native = |index| {
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 449912, index)
    };
    let cap = native(1) as usize;
    let extension = f64::from(native(2));
    assert_eq!(cap, 3);
    assert_eq!(extension, 1.0);
    cast_and_finish(&mut run, 263165);
    let original_expiry = rift_expiry(&run);
    for cast in 1..=cap + 1 {
        cast_and_finish(&mut run, 450983);
        let expected = original_expiry + extension * cast.min(cap) as f64;
        assert!(
            (rift_expiry(&run) - expected).abs() < 0.001,
            "actual Void Blast{cast} extends exactly to native cap; observed{} expected{expected}",
            rift_expiry(&run)
        );
    }
}
