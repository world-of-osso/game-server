//! Actual native casts and attack requests; no timer, aura, power or stat grants.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::casting::{CastState, CastType};
use shared::protocol::CombatLogKind;

type Run = super::super::Run;
const SCK: u32 = 101546;
const FOF: u32 = 113656;
const VIVIFY: u32 = 116670;
const ALLOW_ACTIONS_DURING_CHANNEL: u32 = 1;

fn prepare() -> Run {
    let mut run = super::super::prepare_measurement(269, 1).unwrap();
    let started = run.app.world().resource::<Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, started);
    super::super::engage(&mut run);
    for _ in 0..3 {
        super::tests::cast(&mut run, 100780);
    }
    run
}

fn start_white(run: &mut Run) {
    let world = run.app.world_mut();
    let current = world.get::<crate::combat::AutoAttack>(run.actor).cloned();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    crate::combat::start_attack(&mut commands, run.actor, run.targets[0], current.as_ref());
    queue.apply(world);
}

fn actual_cast(run: &Run) -> CastState {
    run.app
        .world()
        .get::<CastState>(run.actor)
        .cloned()
        .expect("actual native source must start its noninstant cast")
}

fn begin_spell(run: &mut Run, spell: u32, target: Entity) -> CastState {
    use crate::spell_cast::CastFailure;
    for _ in 0..1200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => return actual_cast(run),
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::InProgress
                | CastFailure::NotEnoughPower(_),
            ) => run.app.update(),
            Err(reason) => panic!("native spell{spell} refused: {reason:?}"),
        }
    }
    panic!("native spell{spell} never became ready through real frames");
}

fn white_deliveries(run: &Run) -> usize {
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

fn finish_cast_and_count_white(run: &mut Run, spell: u32) -> usize {
    assert_eq!(actual_cast(run).spell_id, spell);
    let mut during = 0;
    for _ in 0..200 {
        let before = white_deliveries(run);
        run.app.update();
        let active = run
            .app
            .world()
            .get::<CastState>(run.actor)
            .is_some_and(|cast| cast.spell_id == spell);
        if !active {
            return during;
        }
        // Exclude the completion frame: its swing may correctly follow cast removal.
        during += white_deliveries(run) - before;
    }
    panic!("actual native spell{spell} did not finish within10s");
}

fn channel_flags(run: &Run, state: &CastState) -> u32 {
    assert_eq!(state.cast_type, CastType::Channel);
    run.app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(state.spell_id)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .attributes[5]
}

#[test]
#[ignore = "native SCK allows ordinary melee while its real channel stays active"]
fn literal_ww_native_channel_melee_sck_allows_real_white_swing() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare();
    let target = run.targets[0];
    let state = begin_spell(&mut run, SCK, target);
    assert_ne!(
        channel_flags(&run, &state) & ALLOW_ACTIONS_DURING_CHANNEL,
        0
    );
    start_white(&mut run);
    eprintln!(
        "NATIVE_CHANNEL_MELEE admitted={state:?} attack={:?}",
        run.app.world().get::<crate::combat::AutoAttack>(run.actor)
    );
    let during = finish_cast_and_count_white(&mut run, state.spell_id);
    eprintln!(
        "NATIVE_CHANNEL_MELEE spell={} during={during}",
        state.spell_id
    );
    assert!(
        during > 0,
        "TC permits a ready native white swing during SCK's action-allowing channel"
    );
}

#[test]
#[ignore = "native FoF lacks allow-actions; melee waits for actual completion"]
fn literal_ww_native_channel_melee_fof_stays_blocked_then_resumes() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare();
    let target = run.targets[0];
    let state = begin_spell(&mut run, FOF, target);
    assert_eq!(
        channel_flags(&run, &state) & ALLOW_ACTIONS_DURING_CHANNEL,
        0
    );
    start_white(&mut run);
    let before = white_deliveries(&run);
    assert_eq!(finish_cast_and_count_white(&mut run, state.spell_id), 0);
    run.app.update();
    assert!(
        white_deliveries(&run) > before,
        "native white intent resumes after FoF"
    );
}

#[test]
#[ignore = "native normal Vivify cast still blocks melee until completion"]
fn literal_ww_native_channel_melee_normal_cast_stays_blocked_then_resumes() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare();
    let actor = run.actor;
    let state = begin_spell(&mut run, VIVIFY, actor);
    assert_eq!(state.cast_type, CastType::Normal);
    start_white(&mut run);
    let before = white_deliveries(&run);
    assert_eq!(finish_cast_and_count_white(&mut run, state.spell_id), 0);
    run.app.update();
    assert!(
        white_deliveries(&run) > before,
        "native white intent resumes after Vivify"
    );
}

#[test]
#[ignore = "ordinary attack request still produces a native swing without a cast"]
fn literal_ww_native_channel_melee_no_cast_keeps_real_white_swing() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare();
    assert!(run.app.world().get::<CastState>(run.actor).is_none());
    let before = white_deliveries(&run);
    start_white(&mut run);
    run.app.update();
    assert!(white_deliveries(&run) > before);
}
