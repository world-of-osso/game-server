//! Pinned unique_gear_midnight.cpp5233–5280: actual equipped weapon delivery and recipient stacks.
use super::*;
use crate::auras::{ActiveAuras, AuraInstance, AuraSource};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::{CombatLogEvent, CombatLogKind, EquipmentSlot};

const DRIVER: u32 = 1298023;
const DAMAGE: u32 = 1305395;
const TEAR: u32 = 1305391;

fn owned_tear(run: &Run, target: Entity) -> Option<&AuraInstance> {
    run.app
        .world()
        .get::<ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.spell_id == TEAR && aura.caster == run.actor)
}

fn native_payload_now(run: &mut Run, driver: &AuraInstance, target: Entity) -> CombatLogEvent {
    let actor = run.actor;
    let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let amount = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert!(crate::spell_cast::cast_triggered_with_overrides(
        run.app.world_mut(),
        actor,
        DAMAGE,
        Some(target),
        1,
        crate::spell_cast::TriggeredSpellOverrides {
            effect0_amount: Some(amount),
            item_level: driver.cast_item_level,
            ..Default::default()
        },
    ));
    run.app.world().resource::<ObservedCombatEvents>().0[before..]
        .iter()
        .find(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(DAMAGE)
                && event.kind == CombatLogKind::Damage
        })
        .expect("native instant weapon payload must deliver actual damage")
        .clone()
}

fn assert_native_recipient_amplification(run: &mut Run, driver: &AuraInstance) {
    let stacked = run
        .targets
        .iter()
        .copied()
        .find(|target| owned_tear(run, *target).is_some())
        .unwrap();
    let clean = run
        .targets
        .iter()
        .copied()
        .find(|target| owned_tear(run, *target).is_none())
        .unwrap();
    let untouched = run
        .targets
        .iter()
        .copied()
        .find(|target| *target != stacked && *target != clean)
        .unwrap();
    let untouched_stacks = owned_tear(run, untouched).map(|aura| aura.stacks);
    let percent = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(DRIVER)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .base_points
        / 100.0;
    assert!(percent > 0.0);
    // Real swings above qualify the producer. Instant native payloads, with no
    // clock advance, isolate recipient amplification from changing gear procs.
    for _ in 0..8 {
        let a = owned_tear(run, stacked).unwrap().stacks;
        let b = owned_tear(run, clean).map_or(0, |aura| aura.stacks);
        assert!(
            a > b,
            "native recipients must have distinct earned stack counts"
        );
        let first = native_payload_now(run, driver, stacked);
        let second = native_payload_now(run, driver, clean);
        if first.crit == second.crit {
            let expected = second.amount as f64 * f64::from(1.0 + f32::from(a) * percent)
                / f64::from(1.0 + f32::from(b) * percent);
            assert!(
                (first.amount as f64 - expected).abs() <= 4.0,
                "owned native Tear stacks amplify only this payload/recipient: {first:?}/{second:?}/{expected}"
            );
            assert_eq!(
                owned_tear(run, untouched).map(|aura| aura.stacks),
                untouched_stacks
            );
            return;
        }
    }
    panic!("no matching native critical result in eight paired payloads");
}

#[test]
#[ignore = "requires prepared259/261 equipper or260 negative profile and native private DB"]
fn zathatek_native_actual_profile_weapon_delivers_damage_and_owned_tear() {
    let _seed = crate::random::seed_scope(70448);
    let spec = std::env::var("DPS_PARITY_SPEC")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    let equipped = match spec {
        259 | 261 => true,
        260 => false,
        _ => panic!("unsupported weapon gate spec{spec}"),
    };
    let mut run = prepare_measurement(spec, 3).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    let driver = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == DRIVER && aura.caster == actor)
        .cloned();
    assert_eq!(
        driver.is_some(),
        equipped,
        "the actual profile supplies the weapon, not the test"
    );
    if let Some(driver) = &driver {
        assert_eq!(
            driver.source,
            AuraSource::Equipment(EquipmentSlot::MainHand)
        );
        assert!(driver.cast_item_level.is_some());
        assert!(
            driver
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .unwrap()
                .amount
                > 0.0,
            "the equipped driver must retain its native item-level-scaled budget: {driver:?}"
        );
    }
    for _ in 0..20 {
        run.app.update();
    }
    assert!(
        !run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.spell_id == Some(DAMAGE) && event.kind == CombatLogKind::Damage),
        "an idle equipped character must not proc"
    );
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    engage(&mut run);
    assert!(crate::spell_cast::targets::relation_matches(
        run.app.world(),
        actor,
        target,
        crate::spell_cast::targets::Relation::Enemy
    ));
    crate::combat::start_attack_now(
        run.app.world_mut(),
        actor,
        crate::combat::AutoAttack::new(target),
    );
    let mut cursor = 0;
    let mut swings = 0;
    let mut procs = 0;
    for _ in 0..3600 {
        run.app.update();
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        for event in &events[cursor..] {
            if event.source != Some(actor.to_bits()) || event.kind != CombatLogKind::Damage {
                continue;
            }
            if event.spell_id.is_none() {
                swings += 1;
            }
            if event.spell_id == Some(DAMAGE) {
                assert!(event.amount > 0);
                assert!(
                    run.targets
                        .iter()
                        .any(|target| event.target == Some(target.to_bits()))
                );
                procs += 1;
            }
        }
        cursor = events.len();
        if equipped && procs >= 2 {
            break;
        }
    }
    assert!(
        swings > 0,
        "actual weapon swings must reach the native proc boundary"
    );
    if !equipped {
        assert_eq!(
            procs, 0,
            "a different equipped weapon must not gain this payload"
        );
        assert!(
            run.targets
                .iter()
                .all(|target| owned_tear(&run, *target).is_none())
        );
        return;
    }
    assert!(
        procs >= 2,
        "real equipped1298023 never delivered two1305395 impacts after180simulated seconds; swings={swings},procs={procs}"
    );
    let tear = run
        .targets
        .iter()
        .find_map(|target| owned_tear(&run, *target))
        .expect("a real weapon impact must apply its owned native Tear");
    assert!(tear.stacks > 0 && tear.stacks <= tear.max_stacks);
    assert_native_recipient_amplification(&mut run, driver.as_ref().unwrap());
}
