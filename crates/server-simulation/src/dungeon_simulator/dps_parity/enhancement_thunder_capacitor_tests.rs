//! Actual synchronous MW payment must admit the selected native Thunder Capacitor refund.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const THUNDER_CAPACITOR: u32 = 1262635;
const SUPERCHARGE: u32 = 455110;
const COUNTER: u32 = 344179;
const BOLT: u32 = 188196;
const CAST_BUDGET: usize = 32;

fn stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == COUNTER && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

fn try_cast_funded_bolt(run: &mut Run) -> Option<(u8, u8)> {
    let before = stacks(run);
    let now = crate::spell_cast::now(run.app.world());
    let gcd = run
        .app
        .world()
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap()
        .has_active_gcd(now);
    let casting = run
        .app
        .world()
        .get::<shared::casting::CastState>(run.actor)
        .is_some();
    if before < 5 || gcd || casting {
        return None;
    }
    let first_event = run.app.world().resource::<ObservedCombatEvents>().0.len();
    match crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        BOLT,
        Some(run.targets[0]),
        false,
    ) {
        Ok(()) => {
            assert!(
                run.app
                    .world()
                    .get::<shared::casting::CastState>(run.actor)
                    .is_none(),
                "the sampled paid Bolt must complete synchronously, not queue or hardcast"
            );
            let after = stacks(run);
            if after == before {
                let events = &run.app.world().resource::<ObservedCombatEvents>().0[first_event..];
                assert!(
                    events
                        .iter()
                        .any(|event| event.kind == CombatLogKind::AuraRemoved
                            && event.spell_id == Some(COUNTER)
                            && event.target == Some(run.actor.to_bits())),
                    "the seeded refund must follow actual counter consumption, not skipped payment"
                );
            }
            Some((before, after))
        }
        Err(
            crate::spell_cast::CastFailure::OnGlobalCooldown
            | crate::spell_cast::CastFailure::InProgress,
        ) => None,
        Err(reason) => panic!("native Thunder Capacitor paid Bolt: {reason:?}"),
    }
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_thunder_capacitor_real_paid_bolts_can_refund_the_consumed_maelstrom() {
    // SimC76f8118 sc_shaman.cpp7462–7469: selected effect2.percent rolls after
    // an actual normal/TI cast consumes MW, restoring exactly its consumed count.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    assert!(known.contains(THUNDER_CAPACITOR));
    assert!(
        !known.contains(SUPERCHARGE),
        "this frozen profile isolates Thunder's refund"
    );
    let chance = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == THUNDER_CAPACITOR && aura.caster == run.actor)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    assert!(
        chance > 0.0 && chance < 100.0,
        "retain the native non-guaranteed chance"
    );
    engage(&mut run);
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(target),
    )
    .unwrap();
    let mut paid = 0;
    let mut refunded = false;
    for _ in 0..40000 {
        if let Some((before, after)) = try_cast_funded_bolt(&mut run) {
            paid += 1;
            println!("THUNDER_CAPACITOR paid={paid} before={before} after={after} chance={chance}");
            refunded = after == before;
            if refunded || paid >= CAST_BUDGET {
                break;
            }
        }
        enhancement_doom_deck_tests::try_cast_native_generator(&mut run, target);
        run.app.update();
    }
    assert!(
        refunded || paid == CAST_BUDGET,
        "actual native generators must fund the complete paid-cast control: paid={paid}"
    );
    assert!(
        refunded,
        "selected native Thunder Capacitor must refund at least one actual paid Bolt in the seeded{CAST_BUDGET}-cast control; chance={chance}"
    );
}
