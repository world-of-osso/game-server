//! Actual Overfiend casts, sourced coefficient, crit/mastery, owner fuel and expiry.
use super::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::stats::CharacterStats;
use shared::components::{Npc, PowerType};
use shared::protocol::{CombatLogEvent, CombatLogKind};

struct BoltSample {
    damage: f64,
    crit: f64,
    owner_power: f64,
    mastery: f64,
    minimum: f64,
}

fn prepare_owner(keep_mastery: bool, boost_crit: bool) -> Run {
    let mut run = prepare_literal(267, 1).unwrap();
    engage(&mut run);
    if !keep_mastery {
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.actor,
            RemoveReason::Consumed,
            |aura| aura.spell_id == 77220,
        );
    }
    if boost_crit {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: 113858,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    run.app.update();
    run
}

fn summon_actual_overfiend(run: &mut Run) -> Entity {
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(434587)
        .unwrap()
        .clone();
    assert!(crate::spell_cast::launch_triggered_spell(
        run.app.world_mut(),
        run.actor,
        &spell,
        Some(run.actor),
        1
    ));
    for _ in 0..5 {
        run.app.update();
    }
    run.app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature, &Npc)>()
        .iter(run.app.world())
        .find(|(_, summon, npc)| {
            (summon.owner, summon.spell_id, npc.template_id) == (run.actor, 434587, 217429)
        })
        .map(|(actor, _, _)| actor)
        .expect("native434587 must create the actual owned217429")
}

fn is_bolt(event: &CombatLogEvent, actor: Entity, target: Entity) -> bool {
    let identity = (event.source, event.target, event.spell_id);
    let expected = (Some(actor.to_bits()), Some(target.to_bits()), Some(434589));
    let damage = event.kind == CombatLogKind::Damage;
    identity == expected && damage && event.amount > 0
}

fn read_minimum(world: &World, owner: Entity) -> f64 {
    let incarnate = world
        .get::<crate::class_progression::TraitLoadout>(owner)
        .unwrap()
        .active_talent_spells()
        .any(|talent| talent.spell_id == 387275);
    if !incarnate {
        return 0.5;
    }
    f64::from(
        world
            .resource::<crate::spell_info::SpellStore>()
            .get(387275)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .base_points,
    ) / 100.0
}

fn read_first_sample(run: &Run, actor: Entity) -> Option<BoltSample> {
    let world = run.app.world();
    let hit = world
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| is_bolt(event, actor, run.targets[0]))?;
    assert!(hit.crit, "actual434589 must guarantee its critical outcome");
    let stats = world.get::<CharacterStats>(run.actor).unwrap();
    let spell_crit = world
        .get::<crate::auras::AuraStatMods>(run.actor)
        .map_or(0.0, |mods| mods.spell_crit_pct);
    Some(BoltSample {
        damage: f64::from(hit.amount),
        crit: f64::from(stats.crit_pct + spell_crit),
        owner_power: f64::from(stats.spell_power),
        mastery: f64::from(stats.mastery_pct) / 100.0,
        minimum: read_minimum(world, run.actor),
    })
}

fn sample_bolt(keep_mastery: bool, boost_crit: bool) -> BoltSample {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_owner(keep_mastery, boost_crit);
    let actor = summon_actual_overfiend(&mut run);
    for _ in 0..160 {
        run.app.update();
        if let Some(sample) = read_first_sample(&run, actor) {
            return sample;
        }
    }
    panic!("actual Overfiend must land434589 before its native expiry");
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_overfiend_actual_bolts_borrow_native_value_fund_owner_and_expire() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_owner(true, false);
    let before =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::SoulShards)
            .unwrap()
            .0;
    let actor = summon_actual_overfiend(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 457578));
    let target = run.targets[0];
    run.app
        .insert_resource(FactorCapture::new(actor, target, 434589, "overfiend", 1));
    let deadline = run
        .app
        .world()
        .get::<crate::spell_cast::summon::SummonedCreature>(actor)
        .unwrap()
        .expires_at
        .expect("native Overfiend deadline");
    let mut first = None;
    for _ in 0..120 {
        run.app.update();
        if first.is_none() {
            first = read_first_sample(&run, actor);
        }
    }
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| is_bolt(event, actor, target))
        .collect::<Vec<_>>();
    assert!(
        hits.len() >= 2,
        "actual434589 must repeat before native8s expiry"
    );
    assert!(
        hits.iter().all(|event| event.crit),
        "all actual434589 hits must crit"
    );
    let first = first.expect("actual Overfiend must land before power is qualified");
    let capture = run.app.world().resource::<FactorCapture>().export();
    let formula = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 434589)
        .expect("actual434589 must expose its native calculation");
    let inherited = formula["factors"]["s_power"].as_f64().unwrap();
    assert!((inherited - first.owner_power).abs() < 1.0);
    let store = run.app.world().resource::<crate::spell_info::SpellStore>();
    let primary = |spell| {
        store
            .get(spell)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
    };
    let expected_coefficient =
        primary(116858).bonus_coefficient * primary(1245089).base_points / 100.0;
    let coefficient = formula["factors"]["s_mod"].as_f64().unwrap();
    assert!((coefficient - f64::from(expected_coefficient)).abs() < 0.00001);
    assert_eq!(
        primary(434589).bonus_coefficient,
        0.0,
        "catalog remains native"
    );
    let after =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::SoulShards)
            .unwrap()
            .0;
    assert!(
        after > before,
        "native457578 must fund owner: {before}→{after}"
    );
    for _ in 0..600 {
        if crate::spell_cast::now(run.app.world()) > deadline + 0.1 {
            break;
        }
        run.app.update();
    }
    assert!(
        run.app.world().get_entity(actor).is_err(),
        "actual native actor must expire"
    );
    let retired = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| is_bolt(event, actor, target))
        .count();
    for _ in 0..20 {
        run.app.update();
    }
    let later = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| is_bolt(event, actor, target))
        .count();
    assert_eq!(
        later, retired,
        "no ghost434589 after actual actor retirement"
    );
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_overfiend_guaranteed_crit_separately_scales_with_live_owner_crit() {
    let base = sample_bolt(false, false);
    let boost = sample_bolt(false, true);
    assert!((boost.crit - base.crit - 30.0).abs() < 0.01);
    let ratio = (1.0 + boost.crit / 100.0) / (1.0 + base.crit / 100.0);
    let expected = base.damage * ratio;
    assert!(
        (boost.damage - expected).abs() <= 3.0,
        "actual native Overfiend crit ratio: {}→{}, expected{expected}",
        base.damage,
        boost.damage
    );
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_overfiend_actual_hit_uses_its_source_owner_mastery_range() {
    let base = sample_bolt(false, false);
    let mastered = sample_bolt(true, false);
    let minimum = base.damage * (1.0 + mastered.minimum * mastered.mastery);
    let maximum = base.damage * (1.0 + mastered.mastery);
    assert!(
        mastered.damage >= minimum - 3.0 && mastered.damage <= maximum + 3.0,
        "actual Overfiend mastery {} outside native [{minimum},{maximum}]",
        mastered.damage
    );
}
