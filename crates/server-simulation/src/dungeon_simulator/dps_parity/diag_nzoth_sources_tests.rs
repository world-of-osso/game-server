//! SimC76f8118 priest.cpp198 / priest_shadow.cpp576/775/1436.
use super::literal_rotation::execution::{self, Execution};
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::{read_native_cast_start, test_harness::ObservedCombatEvents, CastFailure};
use crate::spell_info::SpellStore;
use shared::protocol::CombatLogKind;

fn cast_when_ready(run: &mut Run, id: u32) {
    let spell = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(id)
        .unwrap()
        .clone();
    for _ in 0..400 {
        match read_native_cast_start(run.app.world_mut(), run.actor, &spell, run.targets[0]) {
            Ok(_) => {
                execution::execute(
                    run.app.world_mut(),
                    run.actor,
                    Execution::Spell(id, run.targets[0], None),
                )
                .unwrap();
                return;
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::OnCooldown
                | CastFailure::NoCharges
                | CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("ordinary native spell{id}: {reason:?}"),
        }
    }
    panic!("native spell{id} never became ready");
}

fn finish_cast(run: &mut Run) {
    for _ in 0..240 {
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none()
        {
            return;
        }
        run.app.update();
    }
    panic!("ordinary native cast must finish");
}

fn counter(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1243069 && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

fn ordinary_source(spell: u32, expected: u8) {
    let mut run = prepare_literal(258, 1).unwrap();
    let actor = run.actor;
    execution::execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(232698, actor, None),
    )
    .expect("ordinary profile precombat Shadowform");
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 373280));
    assert_eq!(counter(&run), 0);
    cast_when_ready(&mut run, spell);
    finish_cast(&mut run);
    assert_eq!(
        counter(&run),
        expected,
        "ordinary spell{spell} earns sourced N'Zoth execution stacks"
    );
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn diag_nzoth_pain_ordinary_execution_earns_three() {
    ordinary_source(589, 3);
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn diag_nzoth_mind_blast_ordinary_execution_earns_six() {
    ordinary_source(8092, 6);
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn diag_nzoth_vampiric_touch_ordinary_execution_earns_four() {
    ordinary_source(34914, 4);
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn diag_nzoth_torrent_actual_ticks_earn_two_each() {
    let mut run = prepare_literal(258, 1).unwrap();
    engage(&mut run);
    cast_when_ready(&mut run, 263165);
    finish_cast(&mut run);
    let ticks = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(263165)
                && event.amount > 0
        })
        .count();
    assert!(ticks > 0, "ordinary Torrent lands actual ticks");
    assert_eq!(
        usize::from(counter(&run)),
        ticks * 2,
        "each real Torrent tick earns two stacks"
    );
}
