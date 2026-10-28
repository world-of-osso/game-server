//! Real class-NPC white critical results, with native profile stats.
use super::*;
use crate::auras::AuraStatMods;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::stats::CharacterStats;
use shared::protocol::CombatLogKind;

// Real2.07s swings include DT replacements;100 retained whites fit the300s fixture.
const REQUIRED_ATTEMPTS: usize = 100;
// This distinguishes native class crit plus its actual white-only aura from NPC base5%.
// Seeded native outcomes remain stochastic; do not rewrite stats or hit results.
const SAMPLING_TOLERANCE_PCT: f32 = 12.0;

fn observe_white_crit(run: &mut Run, source: Entity) -> (usize, f32) {
    for _ in 0..6000 {
        run.app.update();
        let attempts = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(source.to_bits())
                    && event.spell_id.is_none()
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
            })
            .collect::<Vec<_>>();
        if attempts.len() >= REQUIRED_ATTEMPTS {
            let critical = attempts.iter().filter(|event| event.crit).count();
            return (
                attempts.len(),
                critical as f32 * 100.0 / attempts.len() as f32,
            );
        }
    }
    panic!(
        "native source {source:?} did not deliver{REQUIRED_ATTEMPTS} actual white attempts in300s"
    );
}

fn observe_player_white_miss_rate(hit_modifier: f32) -> f32 {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(577, 1).unwrap();
    let actor = run.actor;
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(actor)
            .is_some()
    );
    assert!(
        run.app
            .world()
            .get::<CharacterStats>(actor)
            .unwrap()
            .off_hand
            .is_some()
    );
    super::channel_timing_tests::remove_equipped_proc_drivers(&mut run);
    engage(&mut run);
    let world = run.app.world_mut();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    crate::combat::start_attack(&mut commands, actor, run.targets[0], None);
    queue.apply(world);
    for _ in 0..1000 {
        run.app
            .world_mut()
            .get_mut::<AuraStatMods>(actor)
            .unwrap()
            .melee_hit_chance_pct = hit_modifier;
        let mut attack = run
            .app
            .world_mut()
            .get_mut::<crate::combat::AutoAttack>(actor)
            .unwrap();
        attack.timer = std::time::Duration::ZERO;
        attack.off_hand_timer = std::time::Duration::ZERO;
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let whites: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id.is_none()
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .collect();
    let misses = whites
        .iter()
        .filter(|event| {
            matches!(
                event.kind,
                CombatLogKind::Miss(shared::protocol::MissKind::Miss)
            )
        })
        .count();
    assert!(
        whites.len() >= 800,
        "expected actual dual-wield swings, got {}",
        whites.len()
    );
    misses as f32 / whites.len() as f32
}

#[test]
#[ignore = "real Havoc player dual-wield white swings use the retail miss table"]
fn havoc_player_white_swings_use_retail_dual_wield_miss_rate() {
    let normal = observe_player_white_miss_rate(0.0);
    let modified = observe_player_white_miss_rate(10.0);
    assert!(
        (0.16..=0.22).contains(&normal),
        "expected 19% misses, got {normal:.3}"
    );
    assert!(
        (0.06..=0.12).contains(&modified),
        "expected 9% misses after hit modifier, got {modified:.3}"
    );
}

fn assert_critical_rate(run: &mut Run, source: Entity, expected: f32) {
    let (attempts, observed) = observe_white_crit(run, source);
    eprintln!(
        "NATIVE_MELEE_CRIT source={source:?} attempts={attempts} observed={observed} expected={expected}"
    );
    assert!(
        (observed - expected).abs() < SAMPLING_TOLERANCE_PCT,
        "native white crit {observed}% must follow actual stats {expected}%, not creature base5%"
    );
}

#[test]
#[ignore = "real WW follower CharacterStats must supply its native white critical chance"]
fn literal_native_melee_crit_class_npc_uses_character_stats() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(269, 1).unwrap();
    let actor = run.actor;
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    let world = run.app.world();
    assert!(world.get::<shared::components::Npc>(actor).is_some());
    let base = world.get::<CharacterStats>(actor).unwrap().crit_pct;
    let modifiers = world.get::<AuraStatMods>(actor).unwrap();
    let white_bonus = modifiers.autoattack_crit_pct + modifiers.npc_weapon_crit_pct;
    assert!(
        base > SAMPLING_TOLERANCE_PCT,
        "actual profile distinguishes class crit from NPC base"
    );
    let target = run.targets[0];
    let world = run.app.world_mut();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    crate::combat::start_attack(&mut commands, actor, target, None);
    queue.apply(world);
    assert_critical_rate(&mut run, actor, base + white_bonus);
}

#[test]
#[ignore = "native Fury player white swings respect selected Focus in Chaos and Enrage"]
fn fury_focus_in_chaos_prevents_only_enraged_white_misses() {
    fn misses(selected: bool, enraged: bool) -> usize {
        let _seed = crate::random::seed_scope(70_448);
        let mut run = prepare_measurement(72, 1).unwrap();
        assert!(run.app.world().get::<shared::components::Player>(run.actor).is_some());
        let actor = run.actor;
        assert!(run.app.world().get::<crate::class_progression::TraitLoadout>(actor).unwrap().active_talent_spells().any(|spell| spell.spell_id == 383486));
        if !selected {
            run.app.world_mut().entity_mut(actor).insert(crate::class_progression::TraitLoadout::default());
        }
        super::channel_timing_tests::remove_equipped_proc_drivers(&mut run);
        engage(&mut run);
        let world = run.app.world_mut();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, world);
        crate::combat::start_attack(&mut commands, actor, run.targets[0], None);
        queue.apply(world);
        for _ in 0..6000 {
            if enraged && !crate::auras::has_aura(run.app.world(), actor, 184362) {
                crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 184362, Some(actor), 1);
            } else if !enraged {
                crate::auras::remove_auras(run.app.world_mut(), actor, crate::auras::RemoveReason::Consumed, |aura| aura.spell_id == 184362 && aura.caster == actor);
            }
            run.app.update();
            let events = &run.app.world().resource::<ObservedCombatEvents>().0;
            if events.iter().filter(|e| e.source == Some(actor.to_bits()) && e.spell_id.is_none() && matches!(e.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))).count() >= 60 {
                let combat = report::combat(run.app.world(), &run.damage_targets, 1.0);
                return combat["actions"].as_array().unwrap().iter().filter(|a| a["spell_id"].is_null() && a["actor"] == actor.to_bits()).map(|a| a["misses"].as_u64().unwrap() as usize).sum();
            }
        }
        panic!("expected 60 actual player white attempts");
    }
    assert_eq!(misses(true, true), 0, "selected Focus prevents white misses during Enrage");
    assert!(misses(true, false) > 0, "without Enrage normal misses remain");
    assert!(misses(false, true) > 0, "without Focus normal misses remain while Enraged");
}
