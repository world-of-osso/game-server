//! Literal Arcane profile control and source-defined native producers.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn stacks(run: &Run, spell: u32) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == run.actor && aura.spell_id == spell)
        })
        .map_or(0, |aura| aura.stacks)
}

fn start_cast(run: &mut Run, spell: u32) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, run.targets[0], Some(run.targets[0])),
    )
    .unwrap_or_else(|error| panic!("ordinary Arcane{spell}: {error}"));
}

fn cast(run: &mut Run, spell: u32) {
    start_cast(run, spell);
    preparation::finish_pending_setup(run).unwrap();
}

fn grant(run: &mut Run, spell: u32, count: u8) {
    for _ in 0..count {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: spell,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
}

fn expected_charge_bonus(run: &Run, spell: u32) -> f32 {
    let world = run.app.world();
    let store = world.resource::<crate::spell_info::SpellStore>();
    let index = u8::from(spell == 44425);
    let effect = |id, index| {
        store
            .get(id)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .unwrap()
    };
    let mastery = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .mastery_points;
    let prodigious = world
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 384612)
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == index))
        .map_or(0.0, |effect| effect.amount);
    assert!(
        mastery > 0.0 && prodigious > 0.0,
        "native fixture must exercise Savant and Prodigious Savant"
    );
    // SimC76f8118 sc_mage.cpp2076–2083: mastery augments the per-charge
    // term; Prodigious Savant multiplies mastery, not the base charge bonus.
    (effect(36032, index).base_points
        + mastery * effect(190740, index + 1).bonus_coefficient * (1.0 + prodigious / 100.0))
        / 100.0
}

fn sample_charge_hit(spell: u32, charges: f32) -> (f32, f32, f32) {
    let mut run = prepare_literal(62, 1).unwrap();
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        run.actor,
        shared::components::PowerType::ArcaneCharges,
        charges,
    )
    .unwrap();
    assert!(
        charges == 0.0 || stacks(&run, 36032) > 0,
        "real power change must also exercise native36032"
    );
    let per_charge = expected_charge_bonus(&run, spell);
    run.app.insert_resource(
        crate::spell_cast::effects::factor_capture::FactorCapture::new(
            run.actor,
            run.targets[0],
            spell,
            "arcane_charge_native",
            1,
        ),
    );
    // Start the same native RNG stream after fixture setup, without replacing
    // native crit, variance, gear, charge auras, or projectile delivery.
    let _seed = crate::random::seed_scope(70448);
    cast(&mut run, spell);
    for _ in 0..20 {
        run.app.update();
    }
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
        })
        .collect::<Vec<_>>();
    assert_eq!(
        hits.len(),
        1,
        "one actual native{spell} impact at{charges} charges"
    );
    let damage = normalize_charge_hit_rolls(&run, spell, hits[0].amount);
    let trace = run
        .app
        .world()
        .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
        .export();
    let result = trace["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "result" && event["spell_id"] == spell)
        .expect("actual charge-hit result");
    let versatility = result["factors"]["versatility"].as_f64().unwrap() as f32;
    assert!(versatility > 0.0, "native charge-hit versatility: {trace}");
    (damage, per_charge, versatility)
}

fn normalize_charge_hit_rolls(run: &Run, spell: u32, amount: i32) -> f32 {
    let trace = run
        .app
        .world()
        .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
        .export();
    let events = trace["events"].as_array().expect("native charge hit trace");
    // Capture stops after its focus hit, but retains earlier non-focus hits.
    // Use the actual spell being normalized, not the first actor event.
    let direct = events
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == spell)
        .unwrap_or_else(|| panic!("native spell{spell} direct amount absent: {trace}"));
    let result = events
        .iter()
        .find(|event| event["event"] == "result" && event["spell_id"] == spell)
        .unwrap_or_else(|| panic!("native spell{spell} damage result absent: {trace}"));
    let scaled = direct["factors"]["native_scaled_amount"].as_f64().unwrap();
    let input = result["factors"]["native_input_amount"].as_f64().unwrap();
    // Native proc/cast timing changes RNG consumption even from the same seed.
    // Divide out the observed variance and critical multiplier; never change
    // native rolls, coefficients, aura state or projectile delivery.
    let rolled_multiplier = input / scaled;
    assert!(
        rolled_multiplier > 0.0,
        "invalid native charge observation: damage={amount}, input={input}, scaled={scaled}, stats={:?}, trace={trace}",
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
    );
    (f64::from(amount) / rolled_multiplier) as f32
}

fn assert_charge_hit_formula(spell: u32) {
    let (base, per_charge, base_versatility) = sample_charge_hit(spell, 0.0);
    assert!(base > 0.0);
    for charges in [2.0, 4.0] {
        let (damage, charged_bonus, charged_versatility) = sample_charge_hit(spell, charges);

        assert_eq!(
            charged_bonus, per_charge,
            "same native mastery/talent values"
        );
        // SimC76f8118 action.cpp4345–4347 snapshots versatility separately.
        // Native gear procs can change it between launch and impact; compare
        // the charge term with each hit's actual observed versatility product.
        let expected = base * (1.0 + charges * per_charge) * charged_versatility / base_versatility;
        assert!(
            (damage - expected).abs() <= expected * 0.005 + 2.0,
            "native{spell}, charges={charges}, base={base}, per_charge={per_charge}, expected={expected}, actual={damage}"
        );
    }
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_charge_roll_normalization_uses_barrage_after_an_earlier_blast() {
    let mut run = prepare_literal(62, 1).unwrap();
    run.app.insert_resource(
        crate::spell_cast::effects::factor_capture::FactorCapture::new(
            run.actor,
            run.targets[0],
            44425,
            "charge_focus_after_foreign_hit",
            1,
        ),
    );
    let _seed = crate::random::seed_scope(70448);
    cast(&mut run, 30451);
    cast(&mut run, 44425);
    for _ in 0..20 {
        run.app.update();
    }
    let trace = run
        .app
        .world()
        .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
        .export();
    let events = trace["events"].as_array().unwrap();
    let first_direct = events
        .iter()
        .find(|event| event["event"] == "direct_amount")
        .unwrap();
    assert_ne!(
        first_direct["spell_id"], 44425,
        "fixture must retain the earlier real hit"
    );
    let direct = events
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 44425)
        .unwrap();
    let result = events
        .iter()
        .find(|event| event["event"] == "result" && event["spell_id"] == 44425)
        .unwrap();
    let hit = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(44425)
                && event.kind == CombatLogKind::Damage
        })
        .expect("real Barrage damage after the earlier Blast");
    let input = result["factors"]["native_input_amount"].as_f64().unwrap();
    let scaled = direct["factors"]["native_scaled_amount"].as_f64().unwrap();
    assert!(
        input > 0.0 && scaled > 0.0,
        "actual Barrage factors: {trace}"
    );
    let expected = f64::from(hit.amount) * scaled / input;
    let actual = normalize_charge_hit_rolls(&run, 44425, hit.amount);
    assert!(
        (f64::from(actual) - expected).abs() < 0.01,
        "normalized Barrage must use its own native factors; actual={actual}, expected={expected}, trace={trace}"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_charge_damage_barrage_applies_source_multiplier_once_per_hit() {
    assert_charge_hit_formula(44425);
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_charge_damage_blast_applies_source_multiplier_once_per_hit() {
    assert_charge_hit_formula(30451);
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_batch_surge_guarantees_clearcasting() {
    let mut run = prepare_literal(62, 1).unwrap();
    assert_eq!(stacks(&run, 263725), 0);
    cast(&mut run, 365350);
    assert!(
        stacks(&run, 263725) > 0,
        "Surge must earn native Clearcasting"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_batch_missiles_ticks_generate_damage_salvo_and_cumulative_power() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(62, 1).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1296582),
        "native gear must really enable the MID2 four-piece producer"
    );
    grant(&mut run, 263725, 1);
    cast(&mut run, 5143);
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(7268)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        hits >= 4,
        "ordinary native Missiles must deliver its scheduled ticks: {hits}"
    );
    assert!(stacks(&run, 1242974) >= 4, "one Salvo per native tick");
    assert!(
        stacks(&run, 1296930) >= 4,
        "one Cumulative Power per native tick"
    );
    let charges = crate::spell_cast::powers::power_value(
        run.app.world(),
        run.actor,
        shared::components::PowerType::ArcaneCharges,
    )
    .unwrap()
    .0;
    assert!(
        charges > 0.0,
        "selected High Voltage must earn native charges across ticks"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_batch_barrage_consumes_salvo_and_rolls_native_orb_and_prismatic() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(62, 1).unwrap();
    // Freeze the actual native max-stack proc as the Barrage input; do not alter its values.
    grant(&mut run, 1242974, 20);
    let mut prism = false;
    let mut orb = false;
    for _ in 0..12 {
        if prism && orb {
            break;
        }
        start_cast(&mut run, 44425);
        // Observe the execution boundary before later Infused Splinter impacts
        // can earn new Salvo during the GCD.
        assert_eq!(
            stacks(&run, 1242974),
            3,
            "Barrage consumes Salvo, then native Polished Focus refunds three"
        );
        prism |= stacks(&run, 1295942) > 0;
        preparation::finish_pending_setup(&mut run).unwrap();
        orb |= run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(153640)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            });
        grant(&mut run, 1242974, 20);
    }
    assert!(prism, "earned Salvo must roll native Prismatic Bolt");
    assert!(
        orb,
        "earned Salvo must roll Orb Barrage and its actual native damage child"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_touch_native_cast_marks_target_and_expires_into_stored_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(62, 1).unwrap();
    cast(&mut run, 321507);
    let owned_touch = run
        .app
        .world()
        .get::<ActiveAuras>(run.targets[0])
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.caster == run.actor && aura.spell_id == 210824)
        });
    assert!(
        owned_touch,
        "actual Touch cast must apply its native accumulation debuff"
    );
    cast(&mut run, 30451);
    let until = crate::spell_cast::now(run.app.world()) + 15.0;
    while crate::spell_cast::now(run.app.world()) < until {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(210833)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "owned spell damage must discharge into native210833 on expiration"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_touch_aegwynns_clearcasting_also_grants_selected_brainstorm() {
    let mut run = prepare_literal(62, 1).unwrap();
    cast(&mut run, 321507);
    assert!(
        stacks(&run, 263725) > 0,
        "selected Aegwynn's Technique guarantees Clearcasting"
    );
    assert!(
        stacks(&run, 461531) > 0,
        "every successful Clearcasting also triggers Brainstorm"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_touch_consumed_cumulative_power_survives_blast_projectile() {
    let damage = |count| {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(62, 1).unwrap();
        grant(&mut run, 1296930, count);
        cast(&mut run, 30451);
        assert_eq!(stacks(&run, 1296930), 0, "Blast spends Cumulative Power");
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(30451)
                    && event.kind == CombatLogKind::Damage
            })
            .map(|event| event.amount)
            .sum::<i32>()
    };
    let plain = damage(0);
    let empowered = damage(8);
    assert!(plain > 0);
    let ratio = f64::from(empowered) / f64::from(plain);
    assert!(
        (ratio - 1.24).abs() < 0.002,
        "eight native3% stacks must survive flight: {ratio}"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_area_barrage_uses_real_charges_for_secondary_targets() {
    let mut run = prepare_literal(62, 3).unwrap();
    cast(&mut run, 321507);
    cast(&mut run, 44425);
    for target in &run.targets {
        let hits = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(44425)
                    && event.kind == CombatLogKind::Damage
            })
            .count();
        assert_eq!(
            hits, 1,
            "charged Barrage hits each native selected recipient once"
        );
    }
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_area_prismatic_splash_excludes_its_primary() {
    let mut run = prepare_literal(62, 3).unwrap();
    grant(&mut run, 1295942, 1);
    cast(&mut run, 1295924);
    // Native1295924 has a2.5s cast and40yd/s flight. Completing its cast/GCD
    // does not deliver the parent missile or the impact-triggered splash yet.
    finish_projectile_flight(&mut run);
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let primary_hits = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1295924)
                && event.kind == CombatLogKind::Damage
        })
        .count();
    assert_eq!(
        primary_hits, 1,
        "real Prismatic primary impact must precede splash checks"
    );
    for (index, target) in run.targets.iter().enumerate() {
        let splash_hits = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1295939)
                    && event.kind == CombatLogKind::Damage
            })
            .count();
        assert_eq!(
            splash_hits,
            usize::from(index != 0),
            "only secondary enemies take Prismatic splash"
        );
    }
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_area_orb_parent_delivers_one_bolt_to_each_native_enemy() {
    let mut run = prepare_literal(62, 3).unwrap();
    cast(&mut run, 153626);
    for target in &run.targets {
        let hits = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(153640)
                    && event.kind == CombatLogKind::Damage
            })
            .count();
        assert_eq!(
            hits, 1,
            "each ordinary Orb impacts every selected native enemy once"
        );
    }
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_area_overpowered_missiles_never_double_hits_primary_per_tick() {
    let mut run = prepare_literal(62, 3).unwrap();
    grant(&mut run, 263725, 1);
    grant(&mut run, 1277009, 1);
    cast(&mut run, 5143);
    let hits: Vec<_> = run
        .targets
        .iter()
        .map(|target| {
            run.app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(7268)
                        && event.kind == CombatLogKind::Damage
                })
                .count()
        })
        .collect();
    assert!(hits[0] >= 4, "actual full channel must deliver its ticks");
    assert_eq!(
        hits[0], hits[1],
        "primary has exactly one full hit, not full plus cleave"
    );
    assert_eq!(
        hits[1], hits[2],
        "both nearby secondary recipients receive each cleave"
    );
}

fn splinter_hits(run: &Run) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(443763)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

fn finish_projectile_flight(run: &mut Run) {
    let end = crate::spell_cast::now(run.app.world()) + 3.0;
    while crate::spell_cast::now(run.app.world()) < end {
        run.app.update();
    }
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_splinter_touch_conjures_signature_spell_count() {
    let mut run = prepare_literal(62, 1).unwrap();
    cast(&mut run, 321507);
    finish_projectile_flight(&mut run);
    assert!(
        splinter_hits(&run) >= 4,
        "selected native Signature Spell grants four Touch Splinters"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_splinter_barrage_converts_spent_salvo_through_force_of_will() {
    let mut run = prepare_literal(62, 1).unwrap();
    grant(&mut run, 1242974, 20);
    cast(&mut run, 44425);
    finish_projectile_flight(&mut run);
    assert!(
        splinter_hits(&run) >= 4,
        "native Force of Will conjures one Splinter per five spent Salvo"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_splinter_orb_uses_selected_splintering_orbs() {
    let mut run = prepare_literal(62, 1).unwrap();
    cast(&mut run, 153626);
    finish_projectile_flight(&mut run);
    assert!(
        splinter_hits(&run) >= 2,
        "ordinary selected Orb impact conjures two native Splinters"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_splinter_surge_uses_native_splinterstorm_count() {
    let mut run = prepare_literal(62, 1).unwrap();
    cast(&mut run, 365350);
    finish_projectile_flight(&mut run);
    assert!(
        splinter_hits(&run) >= 8,
        "ordinary selected Surge conjures native Splinterstorm eight"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_splinter_infused_hits_generate_native_salvo_without_missiles() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(62, 1).unwrap();
    cast(&mut run, 321507);
    cast(&mut run, 365350);
    finish_projectile_flight(&mut run);
    assert!(
        splinter_hits(&run) >= 12,
        "real Signature/Splinterstorm impacts precede the proc check"
    );
    assert!(
        stacks(&run, 1242974) > 0,
        "selected Infused Splinters rolls native Salvo on actual Splinter hits"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_splinter_missiles_spends_one_clearcasting_stack_not_the_whole_bank() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(62, 1).unwrap();
    grant(&mut run, 263725, 3);
    assert_eq!(stacks(&run, 263725), 3, "native Improved Clearcasting cap");
    cast(&mut run, 5143);
    assert!(
        stacks(&run, 263725) >= 2,
        "one Missiles spends one use; its independent proc may refill at most one"
    );
}

fn reacted(run: &Run, alias: &str, spell: u32) -> f64 {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::Facts;
    let bindings = Bindings {
        buffs: [(alias.into(), spell)].into(),
        ..default()
    };
    NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        1,
    )
    .value(&format!("buff.{alias}.react"))
    .expect("native Arcane reaction must be configured before acquisition")
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_reaction_literal_precast_predicts_its_guaranteed_salvo() {
    let _seed = crate::random::seed_scope(70448);
    let run = prepare_measurement(62, 1).unwrap();
    assert!(
        stacks(&run, 1242974) >= 2,
        "real Expanded Mind precast must earn Salvo first"
    );
    assert!(
        reacted(&run, "arcane_salvo", 1242974) >= 2.0,
        "deterministic precast Salvo is immediately predicted, not reaction-delayed"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_reaction_surge_predicts_its_guaranteed_clearcasting() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(62, 1).unwrap();
    cast(&mut run, 365350);
    assert!(
        stacks(&run, 263725) > 0,
        "native Surge must really grant Clearcasting first"
    );
    assert!(
        reacted(&run, "clearcasting", 263725) >= 1.0,
        "guaranteed Surge acquisition is immediately predicted"
    );
}

#[test]
#[ignore = "merge-train shared PRD regression"]
fn arcane_batch_clearcasting_prd_uses_the_pinned_coefficient_cap() {
    let constant = crate::random::capped_accumulated_proc_constant(0.12, 13);
    let mut survival = 1.0;
    let mut expected_attempts = 0.0;
    for attempt in 1..=13 {
        expected_attempts += survival;
        survival *= 1.0 - (constant * f64::from(attempt)).min(1.0);
    }
    assert!((expected_attempts - 1.0 / 0.12).abs() < 1e-10);
    assert_eq!(
        crate::random::capped_accumulated_proc_constant(0.05, 13),
        1e-300
    );
    assert!(
        (crate::random::accumulated_proc_constant(0.15) - 0.03222091437308769).abs() < 1e-12,
        "uncapped Arms caller unchanged"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec62 profile/traits"]
fn arcane_literal_fixture_executes_profile_cooldowns_and_filler() {
    let result = measure(62, 1).expect("native Arcane profile fixture");
    assert_eq!(result["driver"], "LITERAL_SIMC_ARCANE_76f8118");
    assert_eq!(result["assisted_decisions"], 0);
    assert!(
        result["arcane_rotation"]["snapshot_stats"]["native"]["spell_power"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    let actions = result["combat"]["actions"].as_array().unwrap();
    for spell in [30451, 365350, 321507] {
        assert!(
            actions.iter().any(
                |action| action["spell_id"] == spell && action["count"].as_f64().unwrap() > 0.0
            ),
            "literal profile must execute native spell{spell}: {actions:?}"
        );
    }
}
