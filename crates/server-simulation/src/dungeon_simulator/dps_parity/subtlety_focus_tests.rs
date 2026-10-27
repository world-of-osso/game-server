//! SimC76f8118 sc_rogue.cpp7345–7362/7381–7390/11210–11213: selected Shadow
//! Focus damage lasts while Stealth, Vanish or Shadow Dance remains.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use shared::components::PowerType;

const DANCE: u32 = 185313;
const FOCUS: u32 = 112942;

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_focus_real_dance_amplifies_backstab_until_dance_ends() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    let (actor, target) = (run.actor, run.targets[0]);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(DANCE, actor, Some(actor)),
    )
    .unwrap();
    assert!(crate::auras::has_aura(run.app.world(), actor, DANCE));
    assert!(
        crate::auras::has_aura(run.app.world(), actor, FOCUS),
        "real Shadow Dance grants selected Shadow Focus"
    );

    run.app
        .insert_resource(FactorCapture::new(actor, target, 53, "backstab", 1));
    {
        let mut pools = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
            .unwrap();
        let energy = pools.get_mut(PowerType::Energy).unwrap();
        energy.current = energy.max;
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(53, target, Some(target)),
    )
    .unwrap();
    let capture = run.app.world().resource::<FactorCapture>().export();
    let focus: Vec<f64> = capture["modifiers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["spell_id"] == 53 && row["source_spell_id"] == FOCUS)
        .map(|row| row["value"].as_f64().unwrap())
        .collect();
    assert_eq!(focus, vec![10.0], "Backstab takes Shadow Focus effect1 once");

    assert!(crate::auras::set_owned_duration(
        run.app.world_mut(),
        actor,
        actor,
        DANCE,
        1
    ));
    for _ in 0..3 {
        run.app.update();
    }
    assert!(!crate::auras::has_aura(run.app.world(), actor, DANCE));
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, FOCUS),
        "Shadow Focus ends with the last stealth-like state"
    );
}
