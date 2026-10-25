//! Real class-NPC and HunterPet white critical results, with native profile stats.
use super::*;
use crate::auras::AuraStatMods;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::stats::CharacterStats;
use shared::protocol::CombatLogKind;

const REQUIRED_ATTEMPTS: usize = 128;
// This distinguishes native31%+30%/25% from the erroneous creature0–5% base.
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
    panic!("native source {source:?} did not deliver128 actual white attempts in300s");
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
    let white_bonus = world
        .get::<AuraStatMods>(actor)
        .unwrap()
        .autoattack_crit_pct;
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
#[ignore = "actual Call Pet HunterPet must inherit live owner white critical chance"]
fn literal_native_melee_crit_hunter_pet_inherits_owner() {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    let world = run.app.world();
    assert_eq!(
        world
            .get::<crate::hunter_pet::HunterPet>(pet)
            .unwrap()
            .owner,
        run.actor
    );
    let owner_crit = world.get::<CharacterStats>(run.actor).unwrap().crit_pct;
    let pet_bonus = world
        .get::<AuraStatMods>(pet)
        .map_or(0.0, |mods| mods.npc_crit_pct + mods.autoattack_crit_pct);
    assert!(
        owner_crit > SAMPLING_TOLERANCE_PCT,
        "native owner crit distinguishes inherited crit from base5%"
    );
    let target = run.targets[0];
    let actor = run.actor;
    crate::pet_control::order_companion_pet(run.app.world_mut(), actor, Some(target));
    assert_critical_rate(&mut run, pet, owner_crit + pet_bonus);
}
