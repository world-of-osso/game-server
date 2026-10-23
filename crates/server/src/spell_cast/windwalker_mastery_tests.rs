//! WW_COMBO_STRIKES_SIMC_POLICY: native casts, DB2 mastery coefficient, no granted buff.
use super::*;
use crate::spell_cast::test_harness::Harness;

const TIGER_PALM: u32 = 100780;
const COMBO_STRIKES: u32 = 115636;

fn native_tiger_palm_before_crit_and_defenses(h: &Harness, caster: Entity) -> f32 {
    let world = h.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(TIGER_PALM)
        .unwrap();
    let effect = &spell.effects[0];
    let ctx = EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let damage = DamageInput {
        amount: scaled_amount(world, caster, spell, effect),
        crit: false,
        periodic: false,
        resolved_source: false,
        effect: Some(effect),
        bonus_caster: caster,
    };
    // Same production amount consumer as each actual native hit, without rerolling
    // variance/crit/defenses when comparing the two causal cast states.
    damage_before_mitigation(world, &ctx, h.npc, &damage)
}

fn cast_native(h: &mut Harness, caster: Entity, spell: u32, target: Entity) {
    use crate::spell_cast::CastFailure;
    for _ in 0..1_200 {
        match crate::spell_cast::creature_cast(
            h.app.world_mut(),
            caster,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => {
                h.advance(3);
                return;
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::InProgress
                | CastFailure::NotEnoughPower(_),
            ) => h.advance(1),
            Err(error) => panic!("native spell{spell} admission failed: {error:?}"),
        }
    }
    panic!("native spell{spell} never reached readiness");
}

#[test]
fn dps_parity_windwalker_combo_strikes_first_cast_and_repeat_without_hit_combo() {
    let (mut h, caster) = crate::followers::kit_tests::at_level(227734, 10, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(h.npc)
        .unwrap()
        .max = 100_000_000.0;
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(h.npc)
        .unwrap()
        .current = 100_000_000.0;
    h.advance(6);
    assert!(crate::auras::has_aura(h.app.world(), caster, COMBO_STRIKES));
    assert!(
        !crate::auras::has_aura(h.app.world(), caster, 196740),
        "mastery exists before Hit Combo is learned"
    );
    let stats = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(caster)
        .unwrap();
    let mastery = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(COMBO_STRIKES)
        .unwrap();
    let percent = stats.mastery_points * mastery.effects[0].bonus_coefficient;
    assert!(
        percent > 0.0,
        "real native mastery points/coefficient required"
    );
    let target = h.npc;
    cast_native(&mut h, caster, TIGER_PALM, target);
    let first = native_tiger_palm_before_crit_and_defenses(&h, caster);
    // Native Provoke115546 is utility, not a source may_combo_strike action.
    cast_native(&mut h, caster, 115546, target);
    let after_utility = native_tiger_palm_before_crit_and_defenses(&h, caster);
    assert!(
        (after_utility - first).abs() < 0.01,
        "non-combo utility must preserve mastery state"
    );
    cast_native(&mut h, caster, TIGER_PALM, target);
    let repeated = native_tiger_palm_before_crit_and_defenses(&h, caster);
    let expected = repeated * (1.0 + percent / 100.0);
    assert!(
        (first - expected).abs() < 0.01,
        "WW_COMBO_STRIKES_SIMC_POLICY: first native action must gain {percent}% and repeat must not; first={first}, repeated={repeated}, expected={expected}"
    );
}
