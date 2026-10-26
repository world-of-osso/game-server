//! Actual Nathreza chains retain the primary and only owned-corrupted secondaries.
use super::*;

const HAUNT: u32 = 48181;
const SHADOW: u32 = 1262710;
const WITHER: u32 = 445474;

fn hits(run: &Run, target: Entity) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(SHADOW)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

fn apply_wither(run: &mut Run, caster: Entity, target: Entity) {
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: WITHER,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_nathreza_targets_real_haunt_pulse_requires_owned_dot_only_on_secondaries() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 3).unwrap();
    engage(&mut run);
    let (caster, primary, dotted, foreign) =
        (run.actor, run.targets[0], run.targets[1], run.targets[2]);
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, HAUNT, Some(primary), false)
        .unwrap();
    for _ in 0..80 {
        run.app.update();
    }
    assert!(
        hits(&run, primary) > 0,
        "the real Haunt pulse must emit native Nathreza on its undotted primary"
    );
    // SimC76f8118 actions2539: the primary is unconditional; additional
    // native chain recipients require this caster's ticking Corruption/Wither.
    assert_eq!(
        hits(&run, dotted),
        0,
        "undotted secondary cannot receive a Nathreza chain"
    );
    assert_eq!(hits(&run, foreign), 0);
    apply_wither(&mut run, caster, dotted);
    apply_wither(&mut run, foreign, foreign);
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        hits(&run, dotted) > 0,
        "owned ticking Wither admits the native secondary"
    );
    assert_eq!(
        hits(&run, foreign),
        0,
        "another caster's Wither is not eligible"
    );
    let before = hits(&run, dotted);
    crate::auras::remove_auras(
        run.app.world_mut(),
        dotted,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == WITHER && aura.caster == caster,
    );
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(
        hits(&run, dotted),
        before,
        "losing the actual owned DoT removes secondary eligibility"
    );
}
