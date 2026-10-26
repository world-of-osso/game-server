//! Real UA applications and independent expirations own the native MID2 buff.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;

const FOUR_PIECE: u32 = 1296569;
const EMPOWERMENT: u32 = 1305774;
const UA: u32 = 1259790;

fn owned_stacks(run: &Run, target: Entity, caster: Entity, spell: u32) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == caster && aura.spell_id == spell)
        })
        .map_or(0, |aura| aura.stacks)
}

fn buff_stacks(run: &Run) -> u8 {
    owned_stacks(run, run.actor, run.actor, EMPOWERMENT)
}

fn remove_passive(run: &mut Run, spell: u32) {
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Unlearned,
        |aura| aura.spell_id == spell,
    );
}

fn cast_ua(run: &mut Run, target: Entity) {
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, UA, Some(target), 0);
}

fn cast_bolt(run: &mut Run) {
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 686, Some(run.targets[0]), 0);
    for _ in 0..8 {
        run.app.update();
    }
}

fn bolt_products(run: &Run) -> Vec<f64> {
    let capture = run.app.world().resource::<FactorCapture>().export();
    capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["event"] == "result" && event["spell_id"] == 686)
        .map(|event| {
            event["factors"]["native_spell_mod_product"]
                .as_f64()
                .unwrap()
        })
        .collect()
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_empowerment_actual_ua_counts_owned_global_stacks_and_native_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 3).unwrap();
    engage(&mut run);
    let (caster, first, second, foreign) =
        (run.actor, run.targets[0], run.targets[1], run.targets[2]);
    assert!(crate::auras::has_aura(run.app.world(), caster, FOUR_PIECE));
    remove_passive(&mut run, 1260229);
    run.app
        .world_mut()
        .insert_resource(FactorCapture::new(caster, first, 686, "shadow_bolt", 16));
    cast_bolt(&mut run);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: foreign,
            target: first,
            spell_id: UA,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert_eq!(buff_stacks(&run), 0, "foreign UA cannot empower this actor");
    cast_ua(&mut run, first);
    assert_eq!(
        owned_stacks(&run, first, caster, UA),
        1,
        "real native UA must attach"
    );
    // SimC76f8118 actions1567–1568/5673–5699: count owned UA stacks globally.
    assert_eq!(
        buff_stacks(&run),
        1,
        "one actual UA earns one native buff stack"
    );
    cast_bolt(&mut run);
    cast_ua(&mut run, second);
    assert_eq!(
        buff_stacks(&run),
        2,
        "another target contributes its owned stack"
    );
    cast_ua(&mut run, first);
    assert_eq!(
        buff_stacks(&run),
        3,
        "two stacks on one target count separately"
    );
    cast_ua(&mut run, second);
    assert_eq!(
        buff_stacks(&run),
        3,
        "native cap wins over four active stacks"
    );
    cast_bolt(&mut run);
    let products = bolt_products(&run);
    assert_eq!(products.len(), 3);
    let percent = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        caster,
        EMPOWERMENT,
        0,
    )) / 100.0;
    assert!(percent > 0.0);
    for (product, count) in products[1..].iter().zip([1.0, 3.0]) {
        let ratio = product / products[0];
        let expected = 1.0 + count * percent;
        assert!(
            (ratio - expected).abs() < 0.00001,
            "actual native damage observes {count} earned stacks: {ratio}, expected={expected}"
        );
    }
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_empowerment_independent_ua_expiry_updates_and_clears_native_buff() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 3).unwrap();
    engage(&mut run);
    remove_passive(&mut run, 1260229);
    assert!(crate::auras::has_aura(
        run.app.world(),
        run.actor,
        FOUR_PIECE
    ));
    let first = run.targets[0];
    let second = run.targets[1];
    for target in [first, first, second, second] {
        cast_ua(&mut run, target);
        for _ in 0..20 {
            run.app.update();
        }
    }
    assert_eq!(buff_stacks(&run), 3);
    let mut seen = std::collections::BTreeSet::new();
    // SimC76f8118 actions5779–5780: each actual asynchronous drop updates the buff.
    for _ in 0..240 {
        run.app.update();
        let active: u16 = run
            .targets
            .iter()
            .map(|&target| u16::from(owned_stacks(&run, target, run.actor, UA)))
            .sum();
        let expected = active.min(3) as u8;
        assert_eq!(
            buff_stacks(&run),
            expected,
            "actual UA lifetime owns the buff"
        );
        seen.insert(expected);
    }
    assert!(
        seen.contains(&2) && seen.contains(&1) && seen.contains(&0),
        "staggered actual expirations must pass through2,1,0: {seen:?}"
    );
    assert_eq!(buff_stacks(&run), 0);
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_empowerment_absent_four_piece_cannot_earn_the_buff() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    remove_passive(&mut run, FOUR_PIECE);
    let (caster, target) = (run.actor, run.targets[0]);
    cast_ua(&mut run, target);
    assert_eq!(owned_stacks(&run, target, caster, UA), 1);
    assert_eq!(
        buff_stacks(&run),
        0,
        "real UA without selected four-piece has no buff"
    );
}
