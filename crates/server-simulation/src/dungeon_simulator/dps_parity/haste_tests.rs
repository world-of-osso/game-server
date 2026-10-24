//! TC a352b1fa Player.cpp5334–5361 and SpellAuraEffects.cpp4531–4551.
//! Real BiS haste rating combines multiplicatively with actual aura193 amounts.
use super::*;
use crate::auras::{
    ActiveAuras, AuraApplication, AuraSource, RemoveReason, apply_aura, remove_auras,
};
use crate::spell_info::SpellStore;

const LIVING_FLAME: u32 = 361469;
const POWER_INFUSION: u32 = 10060;
const HEROISM: u32 = 32182;

fn update_stats(run: &mut Run) {
    for _ in 0..4 {
        run.app.update();
    }
}

fn apply_external_haste(run: &mut Run, spell: u32) -> f64 {
    apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: run.actor,
            target: run.actor,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    update_stats(run);
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell)
        .expect("real native external haste aura applied");
    let amount = aura
        .effects
        .iter()
        .find(|effect| effect.aura_type == crate::auras::handlers::MELEE_SLOW)
        .expect("native aura193 supplies the combat-speed amount")
        .amount;
    assert!(amount > 0.0);
    1.0 + f64::from(amount) / 100.0
}

fn haste(run: &Run) -> f64 {
    f64::from(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .haste_pct,
    )
}

#[test]
#[ignore = "native generic haste RED: exact-gear frozen inputs and actual external buffs"]
fn native_haste_rating_and_two_real_combat_speed_auras_multiply_cast_and_gcd() {
    let mut run = prepare(1467, 1).unwrap();
    let stats = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap();
    assert!(
        stats.ratings.haste > 0.0,
        "actual reference gear earns haste rating"
    );
    let initial = haste(&run);
    let flame = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(LIVING_FLAME)
        .unwrap()
        .clone();
    let cast_before = crate::spell_cast::cast_time_ms(run.app.world(), run.actor, &flame);
    let gcd_before =
        crate::spell_cast::native_apl::gcd_value(run.app.world(), run.actor, &flame, "gcd")
            .unwrap();
    let mut external = 1.0;
    for spell in [POWER_INFUSION, HEROISM] {
        external *= apply_external_haste(&mut run, spell);
        let expected = ((1.0 + initial / 100.0) * external - 1.0) * 100.0;
        let actual = haste(&run);
        assert!(
            (actual - expected).abs() < 0.0001,
            "TC haste composition after native{spell}: actual{actual}, expected{expected}, initial{initial}, external{external}"
        );
        let cast_after = crate::spell_cast::cast_time_ms(run.app.world(), run.actor, &flame);
        assert!((f64::from(cast_after) - f64::from(cast_before) / external).abs() <= 2.0);
        let gcd_after =
            crate::spell_cast::native_apl::gcd_value(run.app.world(), run.actor, &flame, "gcd")
                .unwrap();
        let expected_gcd = (gcd_before / external).max(0.750);
        assert!(
            (gcd_after - expected_gcd).abs() <= 0.002,
            "native{spell}: GCD before{gcd_before}, after{gcd_after}, expected{expected_gcd}, external{external}"
        );
    }
    remove_auras(
        run.app.world_mut(),
        run.actor,
        RemoveReason::Cancelled,
        |aura| [POWER_INFUSION, HEROISM].contains(&aura.spell_id),
    );
    update_stats(&mut run);
    assert!(
        (haste(&run) - initial).abs() < 0.0001,
        "native removal restores original rating-derived speed"
    );
}
