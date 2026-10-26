//! Actual Darkglare damage counts the owner's distinct active Affliction DoTs.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::summon::SummonedCreature;
use shared::components::Npc;

fn beam_factor(run: &mut Run, guardian: Entity) -> f64 {
    let target = run.targets[0];
    run.app.world_mut().insert_resource(FactorCapture::new(guardian, target, 205231, "eye_beam", 16));
    crate::spell_cast::cast_triggered(run.app.world_mut(), guardian, 205231, Some(target), 0);
    let capture = run.app.world().resource::<FactorCapture>().export();
    let event = capture["events"].as_array().unwrap().iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 205231)
        .expect("actual guardian beam must resolve");
    event["applied_multipliers"].as_array().unwrap().iter()
        .find(|factor| factor["factor"] == "giantkiller")
        .expect("actual beam retains target-script multiplier")["value"].as_f64().unwrap()
}

fn apply_dot(run: &mut Run, caster: Entity, spell: u32) {
    crate::auras::apply_aura(run.app.world_mut(), &crate::auras::AuraApplication {
        caster, target: run.targets[0], spell_id: spell, effect_mask: u32::MAX,
        source: crate::auras::AuraSource::Cast,
    });
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_darkglare_dots_beam_counts_distinct_owned_dots_not_ua_stacks_or_foreign() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 3).unwrap();
    engage(&mut run);
    let (caster, target, foreign) = (run.actor, run.targets[0], run.targets[1]);
    crate::spell_cast::cast_triggered(run.app.world_mut(), caster, 205180, Some(caster), 0);
    let guardian = run.app.world_mut().query::<(Entity, &SummonedCreature, &Npc)>()
        .iter(run.app.world()).find(|(_, summon, npc)| summon.owner == caster && npc.template_id == 103673)
        .map(|(entity, _, _)| entity).expect("actual owned native Darkglare");
    let baseline = beam_factor(&mut run, guardian);
    assert!((baseline - 1.0).abs() < 0.00001);
    for spell in [980, 1259790, 445474] { apply_dot(&mut run, foreign, spell); }
    assert!((beam_factor(&mut run, guardian) - baseline).abs() < 0.00001);
    apply_dot(&mut run, caster, 980);
    for _ in 0..3 { apply_dot(&mut run, caster, 1259790); }
    let percent = crate::spell_cast::scripts::script_effect_value(run.app.world(), caster, 205180, 2);
    assert_eq!(percent, 10.0);
    // SimC76f8118 pets2445–2455/warlock.cpp248–264: distinct owned kinds,
    // not UA stacks; native205180 effect2 supplies10% per active kind.
    let expected = 1.0 + 2.0 * f64::from(percent) / 100.0;
    let actual = beam_factor(&mut run, guardian) / baseline;
    assert!((actual - expected).abs() < 0.00001, "actual={actual}, native expected={expected}");
    crate::auras::remove_auras(run.app.world_mut(), target, crate::auras::RemoveReason::Consumed,
        |aura| aura.caster == caster && aura.spell_id == 1259790);
    assert!((beam_factor(&mut run, guardian) / baseline - 1.1).abs() < 0.00001);
}
