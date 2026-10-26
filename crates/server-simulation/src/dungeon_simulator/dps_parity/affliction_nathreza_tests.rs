//! The selected apex uses Haunt's actual native periodic-trigger clock.
use super::*;

const SHADOW_OF_NATHREZA: u32 = 1262710;
const APEX: u32 = 1261984;
const HAUNT: u32 = 48181;

fn shadow_hits(run: &Run, caster: Entity, target: Entity) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(SHADOW_OF_NATHREZA)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_nathreza_actual_haunt_clock_requires_selected_apex() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    assert!(crate::auras::has_aura(run.app.world(), caster, APEX));
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, HAUNT, Some(target), false)
        .unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), target, HAUNT),
        "actual landed Haunt is the prerequisite"
    );
    let hits = shadow_hits(&run, caster, target);
    // SimC76f8118 sc_warlock.cpp41–49: selected apex, no tick_zero,
    // Haunt's native effect5 period and the actual Haunt recipient.
    assert!(
        hits >= 3,
        "actual native Haunt pulses must launch Shadow of Nathreza: {hits}"
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == APEX && aura.caster == caster,
    );
    for _ in 0..40 {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), target, HAUNT));
    assert_eq!(
        shadow_hits(&run, caster, target),
        hits,
        "an active Haunt cannot emit the apex payload after its talent is removed"
    );
}
