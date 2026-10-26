//! The real ordinary Blast cast bar uses each native charge timing term once.
use super::*;
use shared::casting::{CastState, CastType};
use shared::components::PowerType;

pub(super) fn charge_amount(run: &Run, index: u8) -> f32 {
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(36032)
        .unwrap();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap();
    crate::spell_cast::effects::modded_base_points(world, run.actor, spell, effect)
}

pub(super) fn set_native_charges(run: &mut Run, charges: f32) {
    let current = crate::spell_cast::powers::power_value(
        run.app.world(),
        run.actor,
        PowerType::ArcaneCharges,
    )
    .unwrap()
    .0;
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        run.actor,
        PowerType::ArcaneCharges,
        charges - current,
    )
    .unwrap();
    assert_eq!(
        crate::spell_cast::powers::power_value(
            run.app.world(),
            run.actor,
            PowerType::ArcaneCharges
        )
        .unwrap()
        .0,
        charges
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_blast_cost_native_cast_bar_uses_one_charge_time_multiplier() {
    // SimC76f8118 sc_mage.cpp2091–2100: the generic execute-time modifier
    // applies36032 effect index3 once. Retain native haste, slow and other mods.
    for charges in [2.0, 4.0] {
        let mut run = prepare_literal(62, 1).unwrap();
        set_native_charges(&mut run, 0.0);
        let spell = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(30451)
            .unwrap()
            .clone();
        let baseline = crate::spell_cast::cast_time_ms(run.app.world(), run.actor, &spell) as f32;
        let expected_ms = baseline * (1.0 + charges * charge_amount(&run, 3) / 100.0);
        set_native_charges(&mut run, charges);
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                30451,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        let cast = run
            .app
            .world()
            .get::<CastState>(run.actor)
            .expect("actual ordinary cast bar");
        assert_eq!((cast.spell_id, cast.cast_type), (30451, CastType::Normal));
        let actual_ms = cast.duration * 1000.0;
        assert!(
            (actual_ms - expected_ms).abs() <= 1.0,
            "native Blast{charges}charges: actual cast bar{actual_ms}ms, one authored time multiplier{expected_ms}ms"
        );
    }
}
