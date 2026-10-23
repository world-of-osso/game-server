//! Native trigger/consumer boundaries independent of follower action selection.
use super::kit_clearcasting_tests::{frame_seconds, wait_for_gcd};
use super::kit_tests::at_level;
use crate::auras::{ActiveAuras, aura_now, has_aura};
use crate::spell_cast::test_harness::Harness;
use crate::spell_cast::{creature_cast, powers::power_value};
use bevy::prelude::*;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

pub(super) fn prepare(entry: u32) -> (Harness, Entity) {
    let (mut h, caster) = at_level(entry, 90, |_| {});
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().max = 1_000_000_000.0;
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 1_000_000_000.0;
    (h, caster)
}

pub(super) fn complete_cast(h: &mut Harness, caster: Entity, spell: u32, target: Entity) {
    wait_for_gcd(h, caster, spell);
    // Native Fire Blast has a separate500ms cooldown even with spare charges.
    for _ in 0..200 {
        let world = h.app.world();
        let info = world
            .resource::<crate::spell_info::SpellStore>()
            .get(spell)
            .unwrap();
        let cooling_down = world
            .get::<crate::spell_cast::SpellHistory>(caster)
            .is_some_and(|history| {
                history.on_cooldown(info, crate::spell_cast::history_now(world))
            });
        if !cooling_down {
            break;
        }
        h.advance(1);
    }
    let first = h.inbox.go.len();
    creature_cast(h.app.world_mut(), caster, spell, Some(target), false).unwrap();
    for _ in 0..200 {
        h.advance(1);
        if h.inbox.go[first..]
            .iter()
            .any(|event| event.caster == caster.to_bits() && event.spell_id == spell)
        {
            return;
        }
    }
    panic!("native spell{spell} did not finish");
}

#[test]
fn dps_parity_fists_of_fury_simc_tick_cadence_preserves_five_native_damage_hits() {
    let (mut h, caster) = prepare(227734);
    let target = h.npc;
    // Earn actual native Chi using Tiger Palm; no pool/cooldown/clock overrides.
    complete_cast(&mut h, caster, 100780, target);
    complete_cast(&mut h, caster, 100780, target);
    let first = h.inbox.log.len();
    complete_cast(&mut h, caster, 113656, target);
    h.advance(200);
    let hits = h.inbox.log[first..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(117418)
        })
        .count();
    assert_eq!(
        hits, 5,
        "FISTS_OF_FURY_SIMC_TICK_CADENCE: one native4s channel, tick0 then1s ticks; not one hit each166ms dummy heartbeat"
    );
}

#[test]
fn proc_fidelity_1467_native_dragonrage_azure_grants_and_spends_essence_burst() {
    let (mut h, caster) = prepare(215240);
    let target = h.npc;
    complete_cast(&mut h, caster, 375087, caster);
    assert!(has_aura(h.app.world(), caster, 375087));
    complete_cast(&mut h, caster, 362969, target);
    assert!(
        has_aura(h.app.world(), caster, 359618),
        "native Dragonrage guarantees Azure Essence Burst"
    );
    let before = power_value(h.app.world(), caster, PowerType::Essence)
        .unwrap()
        .0;
    complete_cast(&mut h, caster, 356995, target);
    assert!(
        !has_aura(h.app.world(), caster, 359618),
        "the free native Disintegrate must spend its earned charge"
    );
    let after = power_value(h.app.world(), caster, PowerType::Essence)
        .unwrap()
        .0;
    assert!(after >= before, "free channel must not reduce Essence");
}

#[test]
fn proc_fidelity_1467_two_earned_stacks_fund_two_free_spenders() {
    let (mut h, caster) = prepare(215240);
    let target = h.npc;
    complete_cast(&mut h, caster, 375087, caster);
    complete_cast(&mut h, caster, 362969, target);
    complete_cast(&mut h, caster, 362969, target);
    let burst = super::kit_clearcasting_tests::own_aura(&h, caster, 359618).unwrap();
    assert_eq!(burst.stacks, 2, "native375722 adds one maximum stack");
    for (spender, remaining) in [(356995, 1), (357211, 0)] {
        let before = power_value(h.app.world(), caster, PowerType::Essence)
            .unwrap()
            .0;
        complete_cast(&mut h, caster, spender, target);
        let current = super::kit_clearcasting_tests::own_aura(&h, caster, 359618);
        assert_eq!(
            current.as_ref().map_or(0, |aura| aura.stacks),
            remaining,
            "one completed spender must spend exactly one earned Essence Burst"
        );
        if let Some(current) = current {
            assert_eq!(current.instance_id, burst.instance_id);
            assert_eq!(current.expires_at, burst.expires_at);
            assert_eq!(current.charges, 1);
        }
        assert!(
            power_value(h.app.world(), caster, PowerType::Essence)
                .unwrap()
                .0
                >= before
        );
        for _ in 0..200 {
            if h.app
                .world()
                .get::<shared::casting::CastState>(caster)
                .is_none()
            {
                break;
            }
            h.advance(1);
        }
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(caster)
                .is_none()
        );
    }
}

#[test]
fn proc_fidelity_63_native_fire_blast_crit_grants_heating_up_then_hot_streak() {
    let (mut h, caster) = prepare(209059);
    assert!(
        h.app
            .world()
            .get::<crate::class_progression::KnownSpells>(caster)
            .is_some_and(|known| known.contains(195283)),
        "the real selected native driver is learned even though Effect3 creates no aura"
    );
    assert!(!has_aura(h.app.world(), caster, 195283));
    let target = h.npc;
    for expected in [48107, 48108] {
        let start = h.inbox.log.len();
        complete_cast(&mut h, caster, 108853, target);
        assert!(
            h.inbox.log[start..]
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(108853)
                    && event.source == Some(caster.to_bits())
                    && event.crit),
            "native Fire Blast must produce the real critical hit used by this oracle"
        );
        assert!(
            has_aura(h.app.world(), caster, expected),
            "critical-hit causal buff{expected} missing"
        );
    }
    assert!(!has_aura(h.app.world(), caster, 48107));
    wait_for_gcd(&mut h, caster, 11366);
    creature_cast(h.app.world_mut(), caster, 11366, Some(target), false).unwrap();
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none(),
        "earned Hot Streak makes the next native Pyroblast instant"
    );
    assert!(
        !has_aura(h.app.world(), caster, 48108),
        "instant Pyroblast must spend Hot Streak at cast, not missile impact"
    );
}

#[test]
fn proc_fidelity_63_feel_the_burn_native_fire_blast_grants_mastery() {
    let (mut h, caster) = prepare(209059);
    assert!(has_aura(h.app.world(), caster, 383391));
    assert!(!has_aura(h.app.world(), caster, 383395));
    let target = h.npc;
    complete_cast(&mut h, caster, 108853, target);
    h.advance(3);
    let buff = super::kit_clearcasting_tests::own_aura(&h, caster, 383395)
        .expect("native383391 must grant real383395 from actual Fire Blast");
    assert_eq!(buff.stacks, 1);
    assert_eq!(buff.duration_ms, 4000);
    assert!(
        buff.effects
            .iter()
            .any(|effect| effect.aura_type == 318 && effect.amount == 2.0)
    );
}

#[test]
fn proc_fidelity_63_feel_the_burn_native_fireball_does_not_grant_mastery() {
    let (mut h, caster) = prepare(209059);
    assert!(has_aura(h.app.world(), caster, 383391));
    let first = h.inbox.log.len();
    let target = h.npc;
    complete_cast(&mut h, caster, 133, target);
    h.advance(20);
    assert!(
        h.inbox.log[first..].iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(133)
                && event.source == Some(caster.to_bits())
                && event.amount > 0
        }),
        "control must land actual native Fireball damage"
    );
    assert!(
        !has_aura(h.app.world(), caster, 383395),
        "native383391 names Fire Blast, not every harmful spell"
    );
}

#[test]
fn proc_fidelity_63_feel_the_burn_native_applications_expire_independently() {
    let (mut h, caster) = prepare(209059);
    let target = h.npc;
    complete_cast(&mut h, caster, 108853, target);
    h.advance(3);
    let first = super::kit_clearcasting_tests::own_aura(&h, caster, 383395)
        .expect("first real Fire Blast must grant383395");
    complete_cast(&mut h, caster, 108853, target);
    h.advance(3);
    let second = super::kit_clearcasting_tests::own_aura(&h, caster, 383395)
        .expect("second real Fire Blast must retain383395");
    assert_eq!(second.stacks, 2, "both actual native applications overlap");
    let first_expiry = first
        .expires_at
        .expect("native first buff has finite4s duration");
    let second_expiry = second
        .expires_at
        .expect("native second buff has finite4s duration");
    assert!(second_expiry > first_expiry);
    for _ in 0..200 {
        if aura_now(h.app.world()) > first_expiry + frame_seconds(&h) {
            break;
        }
        h.advance(1);
    }
    assert!(aura_now(h.app.world()) < second_expiry);
    let remaining = super::kit_clearcasting_tests::own_aura(&h, caster, 383395)
        .expect("second application remains after first naturally expires");
    assert_eq!(
        remaining.stacks, 1,
        "multiple native applications overlap, not a single refreshed two-stack lifetime"
    );
    for _ in 0..200 {
        if aura_now(h.app.world()) > second_expiry + frame_seconds(&h) {
            break;
        }
        h.advance(1);
    }
    assert!(!has_aura(h.app.world(), caster, 383395));
}

#[test]
fn proc_fidelity_63_feel_the_burn_native_cap_replaces_oldest_application() {
    let (mut h, caster) = prepare(209059);
    // Native child casts isolate the authored100-stack cap, not producer frequency.
    for _ in 0..100 {
        crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 383395, Some(caster), 1);
    }
    let initial = super::kit_clearcasting_tests::own_aura(&h, caster, 383395).unwrap();
    assert_eq!(initial.stacks, 100);
    let first_expiry = initial.expires_at.unwrap();
    h.advance(10);
    let target = h.npc;
    complete_cast(&mut h, caster, 108853, target);
    h.advance(3);
    let refreshed = super::kit_clearcasting_tests::own_aura(&h, caster, 383395).unwrap();
    assert_eq!(refreshed.stacks, 100);
    let latest_expiry = refreshed.expires_at.unwrap();
    assert!(latest_expiry > first_expiry);
    for _ in 0..200 {
        if aura_now(h.app.world()) > first_expiry + frame_seconds(&h) {
            break;
        }
        h.advance(1);
    }
    assert!(aura_now(h.app.world()) < latest_expiry);
    let remaining = super::kit_clearcasting_tests::own_aura(&h, caster, 383395).unwrap();
    assert_eq!(
        remaining.stacks, 1,
        "the later real Fire Blast replaces only the oldest expiry at the native cap"
    );
    for _ in 0..200 {
        if aura_now(h.app.world()) > latest_expiry + frame_seconds(&h) {
            break;
        }
        h.advance(1);
    }
    assert!(!has_aura(h.app.world(), caster, 383395));
}

#[test]
fn proc_fidelity_262_inundate_native_lightning_does_not_generate_utility_resource() {
    let (mut h, caster) = prepare(209053);
    assert!(has_aura(h.app.world(), caster, 378776));
    let target = h.npc;
    let first = h.inbox.log.len();
    complete_cast(&mut h, caster, 188196, target);
    h.advance(20);
    assert!(
        h.inbox.log[first..].iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(188196)
                && event.source == Some(caster.to_bits())
                && event.amount > 0
        }),
        "actual native Lightning Bolt must land before its resource consequences are checked"
    );
    assert!(
        h.inbox.log[first..].iter().all(|event| {
            event.spell_id != Some(378777) || event.source != Some(caster.to_bits())
        }),
        "native378776 names utility casts, not Lightning Bolt or its damaging children"
    );
}

#[test]
fn proc_fidelity_262_inundate_native_wind_shear_interrupt_grants_once() {
    let (mut h, caster) = prepare(209053);
    assert!(has_aura(h.app.world(), caster, 378776));
    let first = h.inbox.log.len();
    super::utility_tests::start_enemy_fireball(&mut h, caster);
    let enemy = h.npc;
    complete_cast(&mut h, caster, 57994, enemy);
    h.advance(3);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(enemy)
            .is_none()
    );
    assert!(
        h.inbox.log[first..].iter().any(|event| {
            event.kind == CombatLogKind::Interrupt
                && event.spell_id == Some(57994)
                && event.extra_spell_id == Some(133)
                && event.source == Some(caster.to_bits())
        }),
        "actual native Wind Shear must interrupt a real Fireball"
    );
    let gains: Vec<_> = h.inbox.log[first..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Energize
                && event.spell_id == Some(378777)
                && event.source == Some(caster.to_bits())
                && event.target == Some(caster.to_bits())
        })
        .collect();
    assert_eq!(
        gains.len(),
        1,
        "one actual utility cast grants Inundate once"
    );
    assert_eq!(gains[0].amount, 8, "native378777s1 remains authoritative");
}

#[test]
fn proc_fidelity_262_inundate_native_wind_shear_without_passive_does_not_grant_resource() {
    let (mut h, caster) = prepare(209053);
    assert!(has_aura(h.app.world(), caster, 378776));
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 378776,
    );
    assert!(!has_aura(h.app.world(), caster, 378776));
    let first = h.inbox.log.len();
    super::utility_tests::start_enemy_fireball(&mut h, caster);
    let enemy = h.npc;
    complete_cast(&mut h, caster, 57994, enemy);
    h.advance(3);
    assert!(h.inbox.log[first..].iter().any(|event| {
        event.kind == CombatLogKind::Interrupt
            && event.spell_id == Some(57994)
            && event.extra_spell_id == Some(133)
            && event.source == Some(caster.to_bits())
    }));
    assert!(
        h.inbox.log[first..]
            .iter()
            .all(|event| event.spell_id != Some(378777) || event.source != Some(caster.to_bits())),
        "a successful native utility cast does not invent its removed Inundate driver"
    );
}

#[test]
fn proc_fidelity_262_native_stormkeeper_lightning_bolt_triggers_overload_damage() {
    let (mut h, caster) = prepare(209053);
    assert!(has_aura(h.app.world(), caster, 168534));
    complete_cast(&mut h, caster, 191634, caster);
    assert!(has_aura(h.app.world(), caster, 191634));
    let target = h.npc;
    let first = h.inbox.log.len();
    complete_cast(&mut h, caster, 188196, target);
    h.advance(30);
    assert!(
        h.inbox.log[first..].iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(188196)
                && event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.amount > 0
        }),
        "actual Stormkeeper-affected native Lightning Bolt must land"
    );
    assert!(
        h.inbox.log[first..].iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(45284)
                && event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.amount > 0
        }),
        "real native Stormkeeper-affected cast guarantees actual45284 Elemental Overload, not a synthetic scalar on Lightning Bolt"
    );
}

#[test]
fn proc_fidelity_262_native_stormkeeper_chain_lightning_overloads_each_hit_target() {
    let (mut h, caster) = prepare(209053);
    let target = h.npc;
    let second = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 10.0);
    let third = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 12.0);
    complete_cast(&mut h, caster, 191634, caster);
    assert!(has_aura(h.app.world(), caster, 168534));
    assert!(has_aura(h.app.world(), caster, 191634));
    let first_log = h.inbox.log.len();
    let first_go = h.inbox.go.len();
    complete_cast(&mut h, caster, 188443, target);
    h.advance(60);
    for target in [target, second, third] {
        assert!(
            h.inbox.log[first_log..].iter().any(|event| {
                event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(188443)
                    && event.source == Some(caster.to_bits())
                    && event.target == Some(target.to_bits())
            }),
            "actual native Chain Lightning must hit each of three hostile targets"
        );
    }
    let child_casts = h.inbox.go[first_go..]
        .iter()
        .filter(|event| event.caster == caster.to_bits() && event.spell_id == 45297)
        .count();
    assert_eq!(
        child_casts, 3,
        "TC HIT-phase native SK modifier guarantees one normal45297 cast per root hit, not one per parent cast"
    );
    assert!(h.inbox.log[first_log..].iter().any(|event| {
        event.kind == CombatLogKind::Damage
            && event.spell_id == Some(45297)
            && event.source == Some(caster.to_bits())
            && event.amount > 0
    }));
}

#[test]
fn proc_fidelity_262_native_stormkeeper_chain_without_mastery_has_no_overload() {
    let (mut h, caster) = prepare(209053);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 168534,
    );
    complete_cast(&mut h, caster, 191634, caster);
    assert!(has_aura(h.app.world(), caster, 191634));
    let target = h.npc;
    let first = h.inbox.log.len();
    complete_cast(&mut h, caster, 188443, target);
    h.advance(60);
    assert!(h.inbox.log[first..].iter().any(|event| {
        event.kind == CombatLogKind::Damage
            && event.spell_id == Some(188443)
            && event.source == Some(caster.to_bits())
            && event.target == Some(target.to_bits())
    }));
    assert!(
        h.inbox.log[first..].iter().all(|event| {
            event.spell_id != Some(45297) || event.source != Some(caster.to_bits())
        })
    );
}

fn complete_recovered_lava_burst(h: &mut Harness, caster: Entity, target: Entity) {
    for _ in 0..400 {
        let recovery = {
            let world = h.app.world();
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(51505)
                .unwrap();
            crate::spell_cast::read_healer_recovery(world, caster, spell)
        };
        match recovery {
            Ok(()) => {
                complete_cast(h, caster, 51505, target);
                return;
            }
            Err(
                crate::spell_cast::CastFailure::NoCharges
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::OnGlobalCooldown,
            ) => h.advance(1),
            Err(failure) => panic!("native Lava Burst recovery failed: {failure:?}"),
        }
    }
    panic!("native Lava Burst did not naturally recover a charge within400 frames");
}

#[test]
#[ignore = "native Lava Burst Overload mapping pending natural-chance RED/GREEN"]
fn proc_fidelity_262_native_lava_burst_natural_mastery_produces_overload() {
    const ACQUISITION_MISS_BUDGET: f64 = 1e-6;
    let (mut h, caster) = prepare(209053);
    let chance = super::kit_clearcasting_tests::own_aura(&h, caster, 168534)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount as f64;
    assert!((0.0..100.0).contains(&chance) && chance > 0.0);
    let attempts = (ACQUISITION_MISS_BUDGET.ln() / (1.0 - chance / 100.0).ln()).ceil() as usize;
    let target = h.npc;
    let first = h.inbox.log.len();
    for attempt in 1..=attempts {
        let cast_start = h.inbox.log.len();
        complete_recovered_lava_burst(&mut h, caster, target);
        h.advance(30);
        assert!(
            h.inbox.log[cast_start..].iter().any(|event| {
                event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(285452)
                    && event.source == Some(caster.to_bits())
                    && event.target == Some(target.to_bits())
            }),
            "each actual native51505 must land its authored285452 damage payload"
        );
        if h.inbox.log[first..].iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(285466)
                && event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
        }) {
            println!("LAVA_OVERLOAD native_chance={chance}% attempts={attempt}/{attempts}");
            return;
        }
    }
    panic!(
        "actual51505 casts/{attempts} authored285452 hits with native chance={chance}% produced no285466; miss budget={ACQUISITION_MISS_BUDGET}"
    );
}

#[test]
#[ignore = "native Lava Burst removed-Mastery control pending qualification"]
fn proc_fidelity_262_native_lava_burst_without_mastery_has_no_overload() {
    let (mut h, caster) = prepare(209053);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 168534,
    );
    let target = h.npc;
    let first = h.inbox.log.len();
    complete_recovered_lava_burst(&mut h, caster, target);
    h.advance(30);
    assert!(h.inbox.log[first..].iter().any(|event| {
        event.kind == CombatLogKind::Damage
            && event.spell_id == Some(285452)
            && event.source == Some(caster.to_bits())
            && event.target == Some(target.to_bits())
    }));
    assert!(
        h.inbox.log[first..].iter().all(|event| {
            event.spell_id != Some(285466) || event.source != Some(caster.to_bits())
        })
    );
}

#[test]
fn proc_fidelity_262_native_long_flight_overload_waits_400ms_after_root_hit() {
    let (mut h, caster) = prepare(209053);
    assert!(has_aura(h.app.world(), caster, 168534));
    let mut target_position = *h
        .app
        .world()
        .get::<shared::components::Position>(caster)
        .unwrap();
    target_position.x += 35.0;
    let target = h.npc;
    h.app.world_mut().entity_mut(target).insert(target_position);
    complete_cast(&mut h, caster, 191634, caster);
    assert!(has_aura(h.app.world(), caster, 191634));
    wait_for_gcd(&mut h, caster, 188196);
    let first_log = h.inbox.log.len();
    let first_go = h.inbox.go.len();
    let launched_at = crate::spell_cast::now(h.app.world());
    creature_cast(h.app.world_mut(), caster, 188196, Some(target), false).unwrap();
    let mut root_hit_at = None;
    let mut child_go_at = None;
    for _ in 0..120 {
        h.advance(1);
        let now = crate::spell_cast::now(h.app.world());
        if root_hit_at.is_none()
            && h.inbox.log[first_log..].iter().any(|event| {
                event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(188196)
                    && event.source == Some(caster.to_bits())
                    && event.target == Some(target.to_bits())
            })
        {
            root_hit_at = Some(now);
        }
        if child_go_at.is_none()
            && h.inbox.go[first_go..]
                .iter()
                .any(|event| event.caster == caster.to_bits() && event.spell_id == 45284)
        {
            child_go_at = Some(now);
        }
        if root_hit_at.is_some() && child_go_at.is_some() {
            break;
        }
    }
    let root_hit_at = root_hit_at.expect("actual long-flight188196 must damage target");
    let child_go_at = child_go_at.expect("actual SK modifier must guarantee normal45284 cast");
    let frame = frame_seconds(&h);
    assert!(
        root_hit_at - launched_at > 0.4 + frame,
        "native root flight must distinguish HIT scheduling from launch scheduling"
    );
    let delay = child_go_at - root_hit_at;
    assert!(
        (0.4 - frame..=0.4 + 2.0 * frame).contains(&delay),
        "TC168534 SpellPhaseMask2=HIT: normal child must launch400ms after actual root hit; root={root_hit_at}, child={child_go_at}, delay={delay}"
    );
}

#[test]
fn proc_fidelity_262_native_stormkeeper_without_mastery_does_not_invent_overload() {
    let (mut h, caster) = prepare(209053);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 168534,
    );
    assert!(!has_aura(h.app.world(), caster, 168534));
    complete_cast(&mut h, caster, 191634, caster);
    assert!(has_aura(h.app.world(), caster, 191634));
    let target = h.npc;
    let first = h.inbox.log.len();
    complete_cast(&mut h, caster, 188196, target);
    h.advance(4);
    assert!(
        h.inbox.log[first..]
            .iter()
            .all(|event| event.spell_id != Some(45284)),
        "overload follows the sourced400ms callback, not immediate root impact"
    );
    h.advance(30);
    assert!(h.inbox.log[first..].iter().any(|event| {
        event.kind == CombatLogKind::Damage
            && event.spell_id == Some(188196)
            && event.source == Some(caster.to_bits())
    }));
    assert!(
        h.inbox.log[first..]
            .iter()
            .all(|event| event.spell_id != Some(45284) || event.source != Some(caster.to_bits())),
        "Stormkeeper cannot invent the removed Mastery passive"
    );
}

#[test]
fn proc_fidelity_262_surge_earned_during_lava_burst_survives_that_cast() {
    let (mut h, caster) = at_level(209053, 32, |_| {});
    let target = h.npc;
    creature_cast(h.app.world_mut(), caster, 51505, Some(target), false).unwrap();
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_some()
    );
    // Controlled native proc-spell application isolates its in-flight consumer.
    // This does not claim natural acquisition; that has a separate tick test.
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 77762, Some(caster), 1);
    assert!(has_aura(h.app.world(), caster, 77762));
    for _ in 0..100 {
        h.advance(1);
        if h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none()
        {
            break;
        }
    }
    assert!(
        has_aura(h.app.world(), caster, 77762),
        "a proc earned during the hardcast belongs to the next Lava Burst"
    );
    wait_for_gcd(&mut h, caster, 51505);
    creature_cast(h.app.world_mut(), caster, 51505, Some(target), false).unwrap();
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none()
    );
    assert!(!has_aura(h.app.world(), caster, 77762));
}

#[test]
fn proc_fidelity_63_hot_streak_earned_during_pyroblast_is_not_spent_by_hardcast() {
    let (mut h, caster) = prepare(209059);
    let target = h.npc;
    creature_cast(h.app.world_mut(), caster, 11366, Some(target), false).unwrap();
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_some()
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 48108, Some(caster), 1);
    assert!(has_aura(h.app.world(), caster, 48108));
    for _ in 0..200 {
        h.advance(1);
        if h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none()
        {
            break;
        }
    }
    assert!(
        has_aura(h.app.world(), caster, 48108),
        "hardcast did not receive Hot Streak's instant-cast modifier at admission"
    );
}

#[test]
fn proc_fidelity_262_natural_flame_shock_ticks_grant_lava_surge() {
    // Level32 still knows Flame Shock; level90 replaces it with Voltaic Blaze.
    let (mut h, caster) = at_level(209053, 32, |_| {});
    let target = h.npc;
    h.app.world_mut().get_mut::<Health>(target).unwrap().max = 1_000_000_000.0;
    h.app.world_mut().get_mut::<Health>(target).unwrap().current = 1_000_000_000.0;
    let begin = aura_now(h.app.world());
    let mut ticks = 0;
    while aura_now(h.app.world()) - begin < 120.0 {
        let own_dot = h
            .app
            .world()
            .get::<ActiveAuras>(target)
            .is_some_and(|auras| {
                auras
                    .iter()
                    .any(|aura| aura.spell_id == 188389 && aura.caster == caster)
            });
        if !own_dot {
            complete_cast(&mut h, caster, 470411, target);
        }
        let start = h.inbox.log.len();
        h.advance(1);
        ticks += h.inbox.log[start..]
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.periodic
                    && event.spell_id == Some(188389)
                    && event.source == Some(caster.to_bits())
            })
            .count();
        if has_aura(h.app.world(), caster, 77762) {
            println!(
                "LAVA_SURGE natural_ticks={ticks} elapsed={} frame={}",
                aura_now(h.app.world()) - begin,
                frame_seconds(&h)
            );
            wait_for_gcd(&mut h, caster, 51505);
            creature_cast(h.app.world_mut(), caster, 51505, Some(target), false).unwrap();
            assert!(
                h.app
                    .world()
                    .get::<shared::casting::CastState>(caster)
                    .is_none()
            );
            assert!(
                !has_aura(h.app.world(), caster, 77762),
                "the next native instant Lava Burst consumes the earned charge"
            );
            return;
        }
    }
    assert!(
        ticks >= 50,
        "need real periodic attempts, not an empty fixture"
    );
    panic!("no Lava Surge from{ticks} real ticks over120s");
}
