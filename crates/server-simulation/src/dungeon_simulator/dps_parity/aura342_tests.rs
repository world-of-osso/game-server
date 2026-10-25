//! Actual native342 cast and ordinary white clocks; producer policy is separate.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::powers::PowerPools;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::stats::CharacterStats;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

const SPEED: u32 = 451298;
const CAST_PROBE: u32 = 116;

fn prepare() -> Run {
    let mut run = prepare_measurement(269, 1).unwrap();
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    // Ordinary energy expenditure keeps the observed regeneration sample uncapped.
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
        .expect("ordinary Tiger Palm spends actual Energy");
    for _ in 0..20 {
        run.app.update();
    }
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, SPEED));
    run
}

fn cast_speed(run: &mut Run) {
    // Exercise the real catalog342 aura, not Momentum's still-unimplemented producer.
    let actor = run.actor;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, SPEED, Some(actor), false)
        .expect("ordinary native451298 catalog cast");
    // Stat folding and resource regeneration are ordered across native frames.
    for _ in 0..3 {
        run.app.update();
    }
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SPEED)
        .unwrap();
    assert!(
        aura.effects
            .iter()
            .any(|effect| effect.aura_type == 342 && effect.amount == 60.0)
    );
}

fn start_white(run: &mut Run) {
    let world = run.app.world_mut();
    let current = world.get::<crate::combat::AutoAttack>(run.actor).cloned();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    crate::combat::start_attack(&mut commands, run.actor, run.targets[0], current.as_ref());
    queue.apply(world);
}

fn deliveries(run: &Run) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id.is_none()
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .count()
}

fn observe_reset_interval(run: &mut Run) -> f64 {
    let before = deliveries(run);
    for _ in 0..120 {
        run.app.update();
        if deliveries(run) > before {
            return run
                .app
                .world()
                .get::<crate::combat::AutoAttack>(run.actor)
                .unwrap()
                .timer
                .as_secs_f64();
        }
    }
    panic!("ordinary native attack request produced no swing within6s");
}

fn energy_regen(run: &Run) -> f32 {
    run.app
        .world()
        .get::<PowerPools>(run.actor)
        .unwrap()
        .pools
        .iter()
        .find(|pool| pool.power == PowerType::Energy)
        .unwrap()
        .regen_per_sec
}

fn cast_and_gcd(run: &Run) -> (i32, f64) {
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(CAST_PROBE)
        .unwrap();
    (
        crate::spell_cast::cast_time_ms(world, run.actor, spell),
        crate::spell_cast::native_apl::gcd_value(world, run.actor, spell, "gcd.max").unwrap(),
    )
}

#[test]
#[ignore = "native342 changes actual melee reset interval by its own multiplicative attack-time factor"]
fn literal_native_aura342_melee_interval_uses_attack_speed_factor() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare();
    start_white(&mut run);
    let baseline = observe_reset_interval(&mut run);
    cast_speed(&mut run);
    let boosted = observe_reset_interval(&mut run);
    let expected = baseline / 1.6;
    eprintln!("AURA342_INTERVAL baseline={baseline} boosted={boosted} expected={expected}");
    assert!(
        (boosted - expected).abs() < 0.002,
        "native60%342 multiplies attack speed, not additive spell haste"
    );
}

#[test]
#[ignore = "native342 must not change production cast time or GCD"]
fn literal_native_aura342_leaves_cast_time_and_gcd_unchanged() {
    let mut run = prepare();
    let baseline = cast_and_gcd(&run);
    assert!(baseline.0 > 0 && baseline.1 > 0.0);
    cast_speed(&mut run);
    let boosted = cast_and_gcd(&run);
    eprintln!("AURA342_CAST_GCD baseline={baseline:?} boosted={boosted:?}");
    assert_eq!(
        boosted, baseline,
        "TC342 touches only BASE/OFF/RANGED attack timers"
    );
}

#[test]
#[ignore = "native342 must not change actual derived haste or observed Energy regeneration"]
fn literal_native_aura342_leaves_regeneration_and_spell_haste_unchanged() {
    let mut run = prepare();
    let baseline_regen = energy_regen(&run);
    let baseline_haste = run
        .app
        .world()
        .get::<CharacterStats>(run.actor)
        .unwrap()
        .haste_pct;
    assert!(baseline_regen > 0.0);
    cast_speed(&mut run);
    let boosted_regen = energy_regen(&run);
    let boosted_haste = run
        .app
        .world()
        .get::<CharacterStats>(run.actor)
        .unwrap()
        .haste_pct;
    eprintln!(
        "AURA342_REGEN baseline={baseline_regen} boosted={boosted_regen} haste={baseline_haste}/{boosted_haste}"
    );
    assert_eq!(
        boosted_regen, baseline_regen,
        "auto-attack speed cannot accelerate native Energy regeneration"
    );
    assert_eq!(
        boosted_haste, baseline_haste,
        "342 is not spell/resource haste"
    );
}
