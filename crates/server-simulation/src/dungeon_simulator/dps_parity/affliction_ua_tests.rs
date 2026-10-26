//! Native UA application and asynchronous expiration producers.
use super::*;

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_ua_overlap_earns_native_cascading_calamity() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    assert!(crate::auras::has_aura(run.app.world(), caster, 1261124));
    crate::spell_cast::cast_triggered(run.app.world_mut(), caster, 1259790, Some(target), 0);
    assert!(!crate::auras::has_aura(run.app.world(), caster, 1261125));
    for _ in 0..20 {
        run.app.update();
    }
    crate::spell_cast::cast_triggered(run.app.world_mut(), caster, 1259790, Some(target), 0);
    // SimC76f8118 sc_warlock_actions.cpp1549–1550: overlap, not initial application.
    assert!(
        crate::auras::has_aura(run.app.world(), caster, 1261125),
        "actual overlapping UA must earn selected native Cascading Calamity"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_ua_expiration_earns_free_native_fatal_echoes() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    assert!(crate::auras::has_aura(run.app.world(), caster, 1260229));
    let mut echoed = false;
    for _ in 0..60 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), caster, 1259790, Some(target), 0);
        for _ in 0..140 {
            run.app.update();
        }
        crate::spell_cast::cast_triggered(run.app.world_mut(), caster, 1260269, Some(caster), 0);
        for _ in 0..40 {
            run.app.update();
        }
        echoed = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .is_some_and(|auras| {
                auras
                    .iter()
                    .any(|aura| aura.caster == caster && aura.spell_id == 1259790)
            });
        if echoed {
            break;
        }
    }
    // SimC76f8118 actions1587–1606/5757–5780: expired stacks roll native PRD
    // and reapply a fully effective, free UA; never fabricate a damage event.
    assert!(
        echoed,
        "actual expired UA must earn selected Fatal Echoes reapplication"
    );
    assert_eq!(
        crate::spell_cast::powers::power_value(
            run.app.world(),
            caster,
            shared::components::PowerType::SoulShards
        )
        .unwrap()
        .0,
        0.0,
        "Fatal Echoes cannot spend or grant foreground shards"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), caster, 1261125),
        "Fatal Echoes also earns Cascading Calamity when the previous UA ended"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), caster, 1260269),
        "a background Fatal Echoes UA cannot consume Shard Instability"
    );
    let before = report::combat(run.app.world(), &run.targets, 1.0)["total_damage"]
        .as_u64()
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let after = report::combat(run.app.world(), &run.targets, 1.0)["total_damage"]
        .as_u64()
        .unwrap();
    assert!(
        after > before,
        "the earned native UA must actually tick after its parent's expiration"
    );
}
