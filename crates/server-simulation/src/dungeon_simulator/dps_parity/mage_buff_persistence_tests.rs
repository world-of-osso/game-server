//! A persistent Mage follower restores its precombat Arcane Intellect after each loss.
use super::follower_dps_diagnostic::*;
use super::*;
use crate::auras::{ActiveAuras, RemoveReason};
use server_follower_apl::{FollowerApl, FollowerAplFailed};
use shared::protocol::CombatLogKind;

const ARCANE_INTELLECT: u32 = 1459;

/// Actor casts landing in one frame, and whether its APL is in combat afterwards.
fn step(app: &mut App, actor: Entity) -> (Vec<u32>, bool) {
    app.world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    app.update();
    let world = app.world();
    assert!(world.get::<FollowerAplFailed>(actor).is_none());
    let in_combat = world
        .get::<FollowerApl>(actor)
        .expect("follower APL persists between frames")
        .in_combat();
    let casts = world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
        })
        .filter_map(|event| event.spell_id)
        .collect();
    (casts, in_combat)
}

fn has_intellect(app: &App, actor: Entity) -> bool {
    app.world()
        .get::<ActiveAuras>(actor)
        .is_some_and(|auras| auras.iter().any(|aura| aura.spell_id == ARCANE_INTELLECT))
}

fn set_dead(app: &mut App, target: Entity) {
    app.world_mut()
        .entity_mut(target)
        .insert(shared::components::Health {
            current: 0.0,
            max: 1.0,
        });
}

/// Idle frames after a loss: only Arcane Intellect may be cast, exactly once.
fn assert_single_out_of_combat_recast(app: &mut App, actor: Entity, frames: usize, label: &str) {
    let mut casts = Vec::new();
    for _ in 0..frames {
        let (cast, in_combat) = step(app, actor);
        assert!(!in_combat, "{label}: no pull is active");
        casts.extend(cast);
    }
    assert_eq!(casts, [ARCANE_INTELLECT], "{label}: out-of-combat casts");
    assert!(has_intellect(app, actor), "{label}: buff restored");
}

fn assert_buff_restored_across_three_pulls(spec: u32) {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, spare, _) = prepare_class_follower(app, copy, spec, 90);
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    set_dead(&mut app, spare);
    let mut precombat = Vec::new();
    for _ in 0..200 {
        let (casts, in_combat) = step(&mut app, actor);
        assert!(!in_combat, "no pull yet");
        precombat.extend(casts);
    }
    assert_eq!(
        precombat
            .iter()
            .filter(|&&spell| spell == ARCANE_INTELLECT)
            .count(),
        1,
        "precombat casts {precombat:?}"
    );
    assert!(has_intellect(&app, actor));
    for pull in 1..=3 {
        let target = spawn_level_dummy(&mut app, actor, 90, pull).unwrap();
        app.world_mut().entity_mut(owner).insert((
            crate::networking::CombatTarget(Some(target)),
            crate::combat::AutoAttack::new(target),
        ));
        crate::combat_refs::set_in_combat_with(app.world_mut(), owner, target);
        let mut combat_frames = 0;
        let mut combat_casts = 0;
        for _ in 0..400 {
            let (casts, in_combat) = step(&mut app, actor);
            if in_combat {
                combat_frames += 1;
                combat_casts += casts.len();
                assert!(
                    !casts.contains(&ARCANE_INTELLECT),
                    "pull {pull}: Arcane Intellect is never cast in combat"
                );
            }
        }
        assert!(
            combat_frames > 300,
            "pull {pull} in combat {combat_frames}/400 frames"
        );
        assert!(
            combat_casts > 0,
            "pull {pull}: the rotation casts in combat"
        );
        set_dead(&mut app, target);
        let mut leave = 0;
        while step(&mut app, actor).1 {
            leave += 1;
            assert!(leave < 100, "pull {pull} leaves combat");
        }
        // In-flight effects (a landing Meteor) resolve before the buff is lost.
        for _ in 0..60 {
            let (casts, in_combat) = step(&mut app, actor);
            assert!(!in_combat && !casts.contains(&ARCANE_INTELLECT));
        }
        let removed =
            crate::auras::remove_auras(app.world_mut(), actor, RemoveReason::Expired, |aura| {
                aura.spell_id == ARCANE_INTELLECT
            });
        assert!(!removed.is_empty(), "pull {pull}: the buff was up to lose");
        assert_single_out_of_combat_recast(&mut app, actor, 200, &format!("after pull {pull}"));
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn fire_follower_recasts_lost_arcane_intellect_once_out_of_combat() {
    assert_buff_restored_across_three_pulls(63);
}
