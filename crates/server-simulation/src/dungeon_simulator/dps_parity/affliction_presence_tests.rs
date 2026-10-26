//! Native Darkglare Presence follows the actual owned guardian, not a copied timer.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::summon::SummonedCreature;
use shared::components::{Health, Npc};

fn summon_darkglare(run: &mut Run) -> Entity {
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 205180, Some(run.actor), 0);
    run.app
        .world_mut()
        .query::<(Entity, &SummonedCreature, &Npc)>()
        .iter(run.app.world())
        .find(|(_, summon, npc)| summon.owner == run.actor && npc.template_id == 103673)
        .map(|(entity, _, _)| entity)
        .expect("native205180 creates actual103673")
}

fn damage_factor(run: &mut Run, spell: u32, periodic: bool) -> f64 {
    let (caster, target) = (run.actor, run.targets[0]);
    run.app
        .world_mut()
        .insert_resource(FactorCapture::new(caster, target, spell, "presence", 64));
    crate::spell_cast::cast_triggered(run.app.world_mut(), caster, spell, Some(target), 0);
    for _ in 0..60 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let event = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| {
            event["event"]
                == if periodic {
                    "periodic_amount"
                } else {
                    "direct_amount"
                }
                && event["spell_id"] == spell
        })
        .expect("actual native damage must resolve");
    event["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|factor| factor["factor"] == "ret_damage")
        .expect("native context multiplier is captured")["value"]
        .as_f64()
        .unwrap()
}

fn assert_native_factor(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.00001,
        "actual={actual}, native expected={expected}"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_presence_owned_living_guardian_boosts_masked_direct_until_death() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    assert_native_factor(damage_factor(&mut run, 1261166, false), 1.0);
    let guardian = summon_darkglare(&mut run);
    let amount =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1280663, 0);
    assert_eq!(amount, 20.0, "native70448 supplies the amount");
    // SimC76f8118 pets2487–2500/actions200: arise enables the native mask;
    // actual demise ends it. Shadow Bolt is not in that native mask.
    assert_native_factor(
        damage_factor(&mut run, 1261166, false),
        1.0 + f64::from(amount) / 100.0,
    );
    assert_native_factor(damage_factor(&mut run, 686, false), 1.0);
    run.app
        .world_mut()
        .get_mut::<Health>(guardian)
        .unwrap()
        .current = 0.0;
    assert_native_factor(damage_factor(&mut run, 1261166, false), 1.0);
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_presence_periodic_ends_on_real_expiry() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    assert_native_factor(damage_factor(&mut run, 980, true), 1.0);
    let guardian = summon_darkglare(&mut run);
    assert_native_factor(damage_factor(&mut run, 980, true), 1.2);
    let now = crate::auras::aura_now(run.app.world());
    run.app
        .world_mut()
        .get_mut::<SummonedCreature>(guardian)
        .unwrap()
        .expires_at = Some(now);
    for _ in 0..2 {
        run.app.update();
    }
    assert!(
        run.app.world().get_entity(guardian).is_err(),
        "native summon expiry despawns the guardian"
    );
    assert_native_factor(damage_factor(&mut run, 980, true), 1.0);
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_presence_foreign_guardian_does_not_boost_owner() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let guardian = summon_darkglare(&mut run);
    run.app
        .world_mut()
        .get_mut::<SummonedCreature>(guardian)
        .unwrap()
        .owner = run.targets[0];
    assert_native_factor(damage_factor(&mut run, 1261166, false), 1.0);
    assert_native_factor(damage_factor(&mut run, 980, true), 1.0);
}
