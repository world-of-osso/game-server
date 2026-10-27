//! Selected-profile Fury caps with native Untethered Fury.
use super::*;
use shared::components::PowerType;

fn assert_fury_cap(spec: u32, expected: f32) {
    let mut run = prepare(spec, 1).expect("native Demon Hunter profile");
    for _ in 0..5 {
        run.app.update();
    }
    let (_, max) = crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::Fury)
        .expect("native Fury pool");
    let world = run.app.world();
    let auras = world.get::<crate::auras::ActiveAuras>(run.actor).unwrap();
    for aura in auras.iter() {
        for effect in &aura.effects {
            if effect.aura_type == 418 || effect.aura_type == 35 {
                println!("spec={spec} aura={} caster={:?} effect={:?}", aura.spell_id, aura.caster, effect);
            }
        }
    }
    println!("spec={spec} player={} filter0={} filter1={}", world.get::<shared::components::Player>(run.actor).is_some(), crate::spell_cast::script_registry::effect_enabled(world, run.actor, 452411, 0), crate::spell_cast::script_registry::effect_enabled(world, run.actor, 452411, 1));
    assert_eq!(max, expected, "selected profile Fury cap for spec {spec}");
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_selected_profile_fury_cap() {
    assert_fury_cap(1480, 140.0);
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec577 exact profile/traits"]
fn havoc_selected_profile_fury_cap() {
    assert_fury_cap(577, 170.0);
}
