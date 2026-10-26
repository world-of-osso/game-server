//! SimC76f8118 sc_priest_shadow.cpp1187/1886–1926/2769–2846;
//! sc_priest.hpp1565–1620: actual background executions and channel ticks fund N'Zoth.
use super::literal_rotation::execution::{self, Execution};
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::{read_native_cast_start, test_harness::ObservedCombatEvents, CastFailure};
use crate::spell_info::SpellStore;
use shared::protocol::CombatLogKind;

fn cast_when_ready(run: &mut Run, id: u32) {
    let native = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(id)
        .unwrap()
        .clone();
    for _ in 0..400 {
        let ready = read_native_cast_start(run.app.world_mut(), run.actor, &native, run.targets[0]);
        match ready {
            Ok(_) => {
                let action = Execution::Spell(id, run.targets[0], None);
                execution::execute(run.app.world_mut(), run.actor, action).unwrap();
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
    panic!("real native channel must finish");
}

fn counter(run: &Run) -> usize {
    run.app
        .world()
        .get::<ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1243069 && aura.caster == run.actor)
        .map_or(0, |aura| usize::from(aura.stacks))
}

fn damage_hits(run: &Run, id: u32) -> usize {
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    events
        .iter()
        .filter(|event| {
            let owner = event.source == Some(run.actor.to_bits());
            let target = event.target == Some(run.targets[0].to_bits());
            let damage = event.kind == CombatLogKind::Damage && event.amount > 0;
            owner && target && damage && event.spell_id == Some(id)
        })
        .count()
}

fn assert_periodic_buff(run: &Run, id: u32) {
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == id)
        .expect("actual threshold earns its native caster-owned Insanity buff");
    let native = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    assert!(native.period_ms > 0 && native.amount > 0.0);
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_nzoth_real_slam_background_madness_funds_native_target_counter() {
    let mut run = prepare_literal(258, 1).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 373280));
    assert_eq!(counter(&run), 0);
    cast_when_ready(&mut run, 1227280);
    for _ in 0..80 {
        run.app.update();
    }
    assert!(damage_hits(&run, 1227621) > 0, "actual Slam impacts");
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(run.targets[0])
            .unwrap()
            .iter()
            .any(|aura| aura.caster == run.actor && aura.spell_id == 335467),
        "actual weak Madness executes at the real enemy"
    );
    assert_eq!(
        counter(&run),
        12,
        "one real background Madness contributes its sourced execute count"
    );
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_nzoth_ordinary_funded_madness_counts_its_execution_target() {
    let mut run = prepare_literal(258, 1).unwrap();
    let actor = run.actor;
    execution::execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(232698, actor, None),
    )
    .expect("ordinary profile precombat Shadowform");
    engage(&mut run);
    for _ in 0..3 {
        cast_when_ready(&mut run, 1227280);
        finish_cast(&mut run);
    }
    let before = counter(&run);
    cast_when_ready(&mut run, 335467);
    finish_cast(&mut run);
    assert_eq!(
        counter(&run),
        (before + 12) % 100,
        "ordinary Madness adds the sourced execution count to its actual target"
    );
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_nzoth_actual_flay_ticks_cross_native_thresholds_and_roll_over() {
    let mut run = prepare_literal(258, 1).unwrap();
    engage(&mut run);
    let native = |index| {
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 373280, index)
            as usize
    };
    let horrific = native(0);
    let vision = native(1);
    assert_eq!((horrific, vision), (50, 100));
    for _ in 0..64 {
        let old_horrific = damage_hits(&run, 1243105);
        let old_vision = damage_hits(&run, 1243106);
        cast_when_ready(&mut run, 15407);
        finish_cast(&mut run);
        let ticks = damage_hits(&run, 15407);
        assert!(ticks > 0, "real native channel lands actual ticks");
        assert_eq!(
            counter(&run),
            ticks % vision,
            "one sourced counter point per actual Flay tick, including native rollover"
        );
        let horrific_hits = damage_hits(&run, 1243105);
        let vision_hits = damage_hits(&run, 1243106);
        assert_eq!(horrific_hits, (ticks + vision - horrific) / vision);
        assert_eq!(vision_hits, ticks / vision);
        if horrific_hits > old_horrific {
            assert_periodic_buff(&run, 1243113);
        }
        if vision_hits > old_vision {
            assert_periodic_buff(&run, 1243114);
            return;
        }
    }
    panic!("actual consecutive Flays must cross native100 and emit Vision damage");
}
