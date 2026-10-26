//! MID2 Seed earns an actual partial-strength UA, not a synthetic damage payload.
use super::feral_growth_tests::cast_native_spell_after_regeneration as cast_native;
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use shared::components::Health;

const SEED: u32 = 27243;
const UA: u32 = 1259790;
const MID2: u32 = 1296569;
const EMPOWERMENT: u32 = 1305774;

fn fund_seed(run: &mut Run) {
    for recipient in run.targets.clone() {
        crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 980, Some(recipient), 0);
    }
    for _ in 0..600 {
        let world = run.app.world();
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(SEED)
            .unwrap();
        let costs = crate::spell_cast::applicable_costs(world, run.actor, spell).unwrap();
        if costs.iter().all(|(_, cost, current)| current >= cost) {
            return;
        }
        run.app.update();
    }
    panic!("actual Agony must fund native Seed without a resource grant");
}

fn capture_ua(run: &mut Run) {
    run.app.world_mut().insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        UA,
        "unstable_affliction",
        128,
    ));
}

fn last_ua_context_factor(run: &Run) -> f64 {
    let capture = run.app.world().resource::<FactorCapture>().export();
    let event = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["event"] == "periodic_amount" && event["spell_id"] == UA)
        .last()
        .expect("actual native UA must tick");
    event["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|factor| factor["factor"] == "ret_damage")
        .expect("actual context factor is captured")["value"]
        .as_f64()
        .unwrap()
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_seed_ua_native_partial_stacks_expire_independently_and_suppress_embrace() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    crate::auras::remove_auras(
        run.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Unlearned,
        |aura| aura.spell_id == 1260229,
    );
    fund_seed(&mut run);
    {
        let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
        health.current = health.max * 0.175;
    }
    capture_ua(&mut run);
    cast_native(&mut run, SEED);
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(owned_stacks(&run, target, UA), 1);
    let percent = crate::spell_cast::scripts::script_effect_value(run.app.world(), caster, MID2, 0);
    assert_eq!(percent, 20.0);
    let seed_weight = f64::from(percent) / 100.0;
    // SimC76f8118 warlock.cpp169–172/actions1652–1664: seed applications
    // count visibly as one, but contribute only native20% to actual UA ticks.
    let actual = last_ua_context_factor(&run);
    assert!(
        (actual - seed_weight).abs() < 0.0001,
        "seed-only factor={actual}, native={seed_weight}"
    );
    capture_ua(&mut run);
    crate::spell_cast::cast_triggered(run.app.world_mut(), caster, UA, Some(target), 0);
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(owned_stacks(&run, target, UA), 2);
    // actions531–534: any seed-UA stack suppresses Embrace for ordinary UA too.
    let expected = (1.0 + seed_weight) / 2.0;
    let actual = last_ua_context_factor(&run);
    assert!(
        (actual - expected).abs() < 0.0001,
        "mixed factor={actual}, native={expected}"
    );
    for _ in 0..200 {
        run.app.update();
        if owned_stacks(&run, target, UA) == 1 {
            break;
        }
    }
    assert_eq!(
        owned_stacks(&run, target, UA),
        1,
        "real native seed-stack deadline must leave the later full stack"
    );
    capture_ua(&mut run);
    for _ in 0..22 {
        run.app.update();
    }
    let actual = last_ua_context_factor(&run);
    let bonus = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        caster,
        234876,
        0,
    )) / 100.0;
    // Real preceding hits lower health below the initial17.5%; resumed Embrace
    // is at least its half-threshold value, bounded by the native maximum.
    assert!(
        actual >= 1.0 + bonus * 0.5 - 0.0001 && actual <= 1.0 + bonus + 0.0001,
        "only ordinary UA remains, so native Embrace resumes: factor={actual}"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_seed_ua_tracks_actual_redirected_smart_primary_once() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 3).unwrap();
    engage(&mut run);
    let (caster, original) = (run.actor, run.targets[0]);
    fund_seed(&mut run);
    // Instant admission keeps the blocking Seed alive until the actual plan;
    // no damage tick can detonate it during an unrelated hardcast delay.
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target: caster,
            spell_id: 1260269,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target: original,
            spell_id: SEED,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert_eq!(owned_stacks(&run, original, SEED), 1);
    cast_native(&mut run, SEED);
    assert_eq!(
        owned_stacks(&run, original, UA),
        0,
        "already-seeded original is not the smart primary"
    );
    // SimC76f8118 actions1851–1890/1903–1910: use the same selected primary,
    // not another random choice or the original explicit target.
    let applications: u32 = run.targets[1..]
        .iter()
        .map(|&target| u32::from(owned_stacks(&run, target, UA)))
        .sum();
    assert_eq!(
        applications, 1,
        "only the actual redirected primary earns tier UA"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_seed_ua_instant_seed_and_tier_ua_spend_one_admitted_instability() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    for _ in 0..2 {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster,
                target: caster,
                spell_id: 1260269,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    assert_eq!(owned_stacks(&run, caster, 1260269), 2);
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, SEED, Some(target), false)
        .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(
        owned_stacks(&run, target, UA),
        1,
        "actual instant Seed earns its actual tier UA"
    );
    // SimC76f8118 actions1913–1924: the instant Seed and its seed-applied
    // UA consume one admitted native Instability stack in total, never zero/two.
    assert_eq!(
        owned_stacks(&run, caster, 1260269),
        1,
        "one admitted instant Seed spends exactly one native use"
    );
}

fn owned_stacks(run: &Run, target: Entity, spell: u32) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == run.actor && aura.spell_id == spell)
        })
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_seed_ua_native_mid2_seed_applies_owned_ua_and_empowerment() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 3).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    assert!(crate::auras::has_aura(run.app.world(), caster, MID2));
    assert_eq!(owned_stacks(&run, target, UA), 0);
    // SimC76f8118 actions1799–1813/1903–1910: once at Seed execution,
    // on its smart primary, before any explosion and without a second shard cost.
    fund_seed(&mut run);
    cast_native(&mut run, SEED);
    assert_eq!(
        owned_stacks(&run, target, UA),
        1,
        "actual Seed must produce one real MID2 UA"
    );
    assert_eq!(
        owned_stacks(&run, caster, EMPOWERMENT),
        1,
        "real seed UA contributes to native UE"
    );
    for secondary in &run.targets[1..] {
        assert_eq!(
            owned_stacks(&run, *secondary, UA),
            0,
            "secondary recipients cannot receive tier UA"
        );
    }
    crate::auras::remove_auras(
        run.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Unlearned,
        |aura| aura.spell_id == MID2,
    );
    let other = run.targets[1];
    fund_seed(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, SEED, Some(other), false)
        .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(
        owned_stacks(&run, other, UA),
        0,
        "unselected set cannot emit UA"
    );
}
