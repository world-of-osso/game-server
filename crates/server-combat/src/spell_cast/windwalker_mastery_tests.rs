//! Native WW cast-state damage policies; DB2 coefficients, no granted buff.
use super::*;
use crate::spell_cast::test_harness::Harness;

const TIGER_PALM: u32 = 100780;
const COMBO_STRIKES: u32 = 115636;

fn native_damage_before_crit_and_defenses(h: &Harness, caster: Entity, id: u32) -> f32 {
    let world = h.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
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
                | CastFailure::OnCooldown
                | CastFailure::NoCharges
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
    let first = native_damage_before_crit_and_defenses(&h, caster, TIGER_PALM);
    // Native Provoke115546 is utility, not a source may_combo_strike action.
    cast_native(&mut h, caster, 115546, target);
    let after_utility = native_damage_before_crit_and_defenses(&h, caster, TIGER_PALM);
    assert!(
        (after_utility - first).abs() < 0.01,
        "non-combo utility must preserve mastery state"
    );
    cast_native(&mut h, caster, TIGER_PALM, target);
    let repeated = native_damage_before_crit_and_defenses(&h, caster, TIGER_PALM);
    let expected = repeated * (1.0 + percent / 100.0);
    assert!(
        (first - expected).abs() < 0.01,
        "WW_COMBO_STRIKES_SIMC_POLICY: first native action must gain {percent}% and repeat must not; first={first}, repeated={repeated}, expected={expected}"
    );
}

#[test]
fn dps_parity_windwalker_sunfire_spiral_empowers_actual_combo_rsk_only() {
    const SUNFIRE: u32 = 1272415;
    const RSK: u32 = 107428;
    const RSK_DAMAGE: u32 = 185099;
    let _scope = crate::random::seed_scope(70448);
    let (mut h, caster) = crate::followers::kit_tests::at_level(227734, 69, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(shared::components::Health {
            current: 100_000_000.0,
            max: 100_000_000.0,
        });
    h.advance(6);
    assert!(
        crate::auras::has_aura(h.app.world(), caster, SUNFIRE),
        "native learned Sunfire Spiral"
    );
    let target = h.npc;
    // Ordinary melee keeps native combat/Chi alive while RSK's charge recharges.
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::combat::AutoAttack::new(target));
    for _ in 0..12 {
        let chi = crate::spell_cast::powers::power_value(
            h.app.world(),
            caster,
            shared::components::PowerType::Chi,
        )
        .unwrap()
        .0;
        if chi >= 4.0 {
            break;
        }
        cast_native(&mut h, caster, TIGER_PALM, target);
    }
    let chi = crate::spell_cast::powers::power_value(
        h.app.world(),
        caster,
        shared::components::PowerType::Chi,
    )
    .unwrap()
    .0;
    assert!(
        chi >= 4.0,
        "two actual RSK casts need naturally earned Chi, not a resource grant"
    );
    cast_native(&mut h, caster, RSK, target);
    let combo = native_damage_before_crit_and_defenses(&h, caster, RSK_DAMAGE);
    let world = h.app.world();
    let auras = world.get::<crate::auras::ActiveAuras>(caster).unwrap();
    let sunfire = auras
        .iter()
        .find(|aura| aura.spell_id == SUNFIRE)
        .unwrap()
        .effects[0]
        .amount;
    let mastery = world
        .resource::<crate::spell_info::SpellStore>()
        .get(COMBO_STRIKES)
        .unwrap()
        .effects[0]
        .bonus_coefficient
        * world
            .get::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .mastery_points;
    let hit_combo = auras.iter().find(|aura| aura.spell_id == 196741).unwrap();
    let hit_combo_percent = hit_combo
        .effects
        .iter()
        .find(|effect| effect.misc_values[0] == 0)
        .unwrap()
        .amount
        * f32::from(hit_combo.stacks);
    cast_native(&mut h, caster, RSK, target);
    assert!(
        !crate::auras::has_aura(h.app.world(), caster, 196741),
        "actual repeated RSK clears Hit Combo"
    );
    let repeated = native_damage_before_crit_and_defenses(&h, caster, RSK_DAMAGE);
    let expected = repeated
        * (1.0 + mastery / 100.0)
        * (1.0 + hit_combo_percent / 100.0)
        * (1.0 + sunfire / 100.0);
    eprintln!(
        "SUNFIRE_NATIVE combo={combo} repeated={repeated} mastery={mastery} hit_combo={hit_combo_percent} sunfire={sunfire} expected={expected}"
    );
    assert!(
        (combo - expected).abs() < 0.1,
        "native Sunfire effect must empower actual combo RSK by {sunfire}% only; combo={combo}, repeated={repeated}, expected={expected}"
    );
}

const MYSTIC_TOUCH_PASSIVE: u32 = 8647;
const MYSTIC_TOUCH_DEBUFF: u32 = 113746;

fn native_mystic_fixture(level: u8) -> (Harness, Entity) {
    let (mut h, caster) = crate::followers::kit_tests::at_level_unengaged(227734, level, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    h.advance(6);
    let known = h
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(caster)
        .unwrap();
    assert_eq!(
        known.contains(MYSTIC_TOUCH_PASSIVE),
        level >= 49,
        "native8647 level49 learning boundary must supply this control"
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), h.npc, MYSTIC_TOUCH_DEBUFF),
        "fixture must not start with an earlier actor's target debuff"
    );
    (h, caster)
}

fn land_native_palm(h: &mut Harness, caster: Entity) {
    let target = h.npc;
    let before = h.health(target);
    for _ in 0..8 {
        cast_native(h, caster, TIGER_PALM, target);
        if h.health(target) < before {
            assert!(
                h.health(target) > 0.0,
                "producer control needs a surviving native target"
            );
            return;
        }
    }
    panic!("native TP must actually deal damage before testing its target-debuff producer");
}

#[test]
#[ignore = "requires native70448 class book/spells; learned Mystic Touch real-impact RED"]
fn dps_parity_windwalker_mystic_touch_learned_palm_applies_native_target_debuff() {
    let _scope = crate::random::seed_scope(70448);
    let (mut h, caster) = native_mystic_fixture(49);
    land_native_palm(&mut h, caster);
    assert!(
        crate::auras::has_aura(h.app.world(), h.npc, MYSTIC_TOUCH_DEBUFF),
        "learned8647 must produce native113746 after an actual landed TP"
    );
}

#[test]
#[ignore = "requires native70448 class book/spells; unlearned Mystic Touch control"]
fn dps_parity_windwalker_mystic_touch_unlearned_palm_does_not_grant_debuff() {
    let _scope = crate::random::seed_scope(70448);
    let (mut h, caster) = native_mystic_fixture(48);
    land_native_palm(&mut h, caster);
    assert!(
        !crate::auras::has_aura(h.app.world(), h.npc, MYSTIC_TOUCH_DEBUFF),
        "Monk identity without learned8647 is not permission to grant113746"
    );
}

#[test]
#[ignore = "requires native70448 class book/spells; learned Mystic Touch native white-hit RED"]
fn dps_parity_windwalker_mystic_touch_learned_auto_applies_native_target_debuff() {
    let _scope = crate::random::seed_scope(70448);
    let (mut h, caster) = native_mystic_fixture(49);
    let target = h.npc;
    let before = h.health(target);
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::combat::AutoAttack::new(target));
    for _ in 0..120 {
        h.advance(1);
        if h.health(target) < before {
            break;
        }
    }
    assert!(
        h.health(target) < before,
        "actual native white hit required"
    );
    assert!(h.health(target) > 0.0, "native target must survive the hit");
    assert!(
        crate::auras::has_aura(h.app.world(), target, MYSTIC_TOUCH_DEBUFF),
        "learned8647 producer includes the actual Monk white-hit path"
    );
}

#[test]
#[ignore = "requires native70448 class book/spells; unrelated actor damage control"]
fn dps_parity_windwalker_mystic_touch_nearby_learned_monk_does_not_brand_other_actor_damage() {
    let _scope = crate::random::seed_scope(70448);
    let (mut h, monk) = native_mystic_fixture(49);
    assert!(
        !h.app
            .world()
            .get::<crate::class_progression::KnownSpells>(h.caster)
            .unwrap()
            .contains(MYSTIC_TOUCH_PASSIVE)
    );
    let target = h.npc;
    let before = h.health(target);
    h.attack(target);
    for _ in 0..120 {
        h.advance(1);
        if h.health(target) < before {
            break;
        }
    }
    assert!(
        h.health(target) < before,
        "unrelated actor must actually deal native damage"
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::AutoAttack>(monk)
            .is_none()
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), target, MYSTIC_TOUCH_DEBUFF),
        "a nearby learned Monk cannot lend its passive to unrelated actor damage"
    );
}
