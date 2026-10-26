//! SimC76f8118 shadow626–638/2052–2084/2322–2339 and priest298–316.
//! Actual Pain ticks earn an ordinary usable instant Mind Blast, not a fake buff.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::{CastFailure, test_harness::ObservedCombatEvents};
use server::simc_apl::Facts;
use shared::protocol::CombatLogKind;

fn mind_blast_charges(run: &Run) -> f64 {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    let bindings = Bindings {
        actions: [("mind_blast".into(), 8092)].into(),
        ..Default::default()
    };
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1)
        .value("cooldown.mind_blast.charges")
        .unwrap()
}

fn insight_stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == 375981)
        .map_or(0, |aura| aura.stacks)
}

fn cast(run: &mut Run, spell: u32) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, run.targets[0], None),
    )
    .unwrap();
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_insight_real_pain_ticks_earn_usable_instant_mind_blast_and_consume_one_stack() {
    let mut run = prepare_literal(258, 1).unwrap();
    engage_fixture_opponents(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 375888));
    assert_eq!(insight_stacks(&run), 0);
    cast(&mut run, 8092);
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_some(),
        "unprocced native Mind Blast really has a cast time"
    );
    for _ in 0..100 {
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none()
        {
            break;
        }
        run.app.update();
    }
    let recovery = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(8092)
        .unwrap()
        .categories
        .as_ref()
        .unwrap()
        .charge_recovery_ms;
    for _ in 0..(recovery as usize).div_ceil(50) + 1 {
        run.app.update();
    }
    assert_eq!(
        mind_blast_charges(&run),
        1.0,
        "real native recharge restores the base charge"
    );
    cast(&mut run, 589);
    for frame in 0..1600 {
        if insight_stacks(&run) > 0 {
            break;
        }
        if frame > 0 && frame % 160 == 0 {
            cast(&mut run, 589);
        }
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.spell_id == Some(589) && event.periodic && event.amount > 0),
        "the producer is actual native Pain tick damage"
    );
    assert!(
        insight_stacks(&run) > 0,
        "actual Pain ticks earn native Shadowy Insight"
    );
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(8092)
        .unwrap()
        .clone();
    for _ in 0..100 {
        match crate::spell_cast::read_native_cast_start(
            run.app.world_mut(),
            run.actor,
            &spell,
            run.targets[0],
        ) {
            Ok(_) => break,
            Err(
                CastFailure::OnGlobalCooldown | CastFailure::OnCooldown | CastFailure::NoCharges,
            ) => {
                run.app.update();
            }
            Err(reason) => panic!("earned ordinary Mind Blast: {reason:?}"),
        }
    }
    let earned = insight_stacks(&run);
    assert!(earned > 0);
    let charges = mind_blast_charges(&run);
    assert_eq!(
        charges,
        1.0 + f64::from(earned),
        "native Insight grants its usable extra charge"
    );
    cast(&mut run, 8092);
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none(),
        "earned native Insight makes the actual cast instant"
    );
    assert_eq!(
        insight_stacks(&run),
        earned - 1,
        "one actual Mind Blast consumes one earned stack"
    );
    // Let the normal fixed update publish the reduced native maximum after
    // consuming the buff. This must not consume the underlying base charge.
    run.app.update();
    assert_eq!(
        mind_blast_charges(&run),
        charges - 1.0,
        "consume the earned extra use once, not the underlying base charge again"
    );
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(8092)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(
        hits, 2,
        "ordinary base and earned instant casts both pay native damage"
    );
}
