//! SimC76f8118 sc_priest_shadow.cpp1491–1514/sc_priest.cpp3543–3562.
//! Ordinary Torrent earns the native Rift clock; neither fabricated buffs nor payloads.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn advance(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        run.app.update();
    }
}

fn rift_hits(run: &Run, target: Entity) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            let source = event.source == Some(run.actor.to_bits());
            let recipient = event.target == Some(target.to_bits());
            let damage = event.kind == CombatLogKind::Damage && event.amount > 0;
            source && recipient && damage && event.spell_id == Some(447448)
        })
        .count()
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_rift_ordinary_torrent_starts_native_delayed_area_pulses_and_full_tail() {
    let mut run = prepare_literal(258, 3).unwrap();
    engage_fixture_opponents(&mut run);
    let actor = run.actor;
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(263165, run.targets[0], None),
    )
    .expect("ordinary native Void Torrent starts");
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == actor && aura.spell_id == 459314)
        .expect("ordinary Torrent earns the real native Rift driver")
        .clone();
    let period = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .period_ms;
    assert!(period > 0);
    assert_eq!(rift_hits(&run, run.targets[0]), 0, "no tick-zero Rift");
    let period_frames = (period as usize).div_ceil(50);
    advance(&mut run, period_frames);
    assert_eq!(
        rift_hits(&run, run.targets[0]),
        0,
        "first travelling Rift tick misses"
    );
    advance(&mut run, period_frames + 1);
    for &target in &run.targets {
        assert_eq!(
            rift_hits(&run, target),
            1,
            "second native Rift tick hits each nearby enemy once"
        );
    }
    for _ in 0..100 {
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(actor)
            .is_none()
        {
            break;
        }
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(actor)
            .is_none()
    );
    let now = crate::spell_cast::now(run.app.world());
    let tail = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 459314 && aura.caster == actor)
        .unwrap();
    let remaining = tail.expires_at.unwrap() - now;
    assert!(
        (remaining - f64::from(tail.duration_ms) / 1000.0).abs() <= 0.1,
        "channel end restores the full native Rift tail, observed {remaining}s"
    );
    advance(&mut run, 20);
    for &target in &run.targets {
        assert!(
            rift_hits(&run, target) > 1,
            "native Rift continues after Torrent ends"
        );
    }
}
