use super::*;
use shared::protocol::CombatLogKind;

const INQUISITOR: u32 = 1276283;
const MIND_SEAR: u32 = 1280457;
const MIND_SEAR_TICK: u32 = 1280460;

fn summon_inquisitor(run: &mut Run) -> Entity {
    let owner = run.actor;
    crate::spell_cast::creature_cast(run.app.world_mut(), owner, INQUISITOR, Some(owner), true)
        .unwrap();
    run.app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(_, summon)| summon.owner == owner && summon.spell_id == INQUISITOR)
        .map(|(actor, _)| actor)
        .expect("real Antoran Inquisitor")
}

fn ticked_since(run: &Run, inquisitor: Entity, target: Entity, start: usize) -> bool {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..]
        .iter()
        .any(|event| {
            event.source == Some(inquisitor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(MIND_SEAR_TICK)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
}

fn observed_len(run: &Run) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len()
}

fn channeling_mind_sear(run: &Run, inquisitor: Entity) -> bool {
    run.app
        .world()
        .get::<shared::casting::CastState>(inquisitor)
        .is_some_and(|state| {
            state.spell_id == MIND_SEAR && state.cast_type == shared::casting::CastType::Channel
        })
}

#[test]
#[ignore = "requires canonical 266 inputs; real Inquisitor Mind Sear cast and channel"]
fn demonology_inquisitor_mind_sear_ticks_after_hasted_precast() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 3).unwrap();
    let owner = run.actor;
    engage(&mut run);
    run.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    let inquisitor = summon_inquisitor(&mut run);
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1280457)
        .unwrap();
    let duration = spell.misc.as_ref().unwrap().duration.unwrap().duration_ms;
    let cast_ms = crate::spell_cast::cast_time_ms(run.app.world(), inquisitor, spell);
    assert!(
        cast_ms > 0 && cast_ms < duration,
        "native duration must receive owner haste"
    );
    for _ in 0..(cast_ms as usize / 50).saturating_sub(10) {
        run.app.update();
    }
    assert!(
        !run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(inquisitor.to_bits())
                && event.spell_id == Some(1280460)),
        "Mind Sear cannot tick before its hasted cast completes"
    );
    for _ in 0..20 {
        run.app.update();
    }
    let hits = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        assert!(
            hits.iter()
                .any(|event| event.source == Some(inquisitor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1280460)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "the completed cast begins its channel with a tick against each target"
        );
    }
}

#[test]
#[ignore = "requires canonical 266 inputs; overlapping Inquisitor cast and channel"]
fn demonology_inquisitor_recasts_mind_sear_during_channel() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 1).unwrap();
    let owner = run.actor;
    engage(&mut run);
    run.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(owner)
        .unwrap()
        .haste_pct = 150.0;
    let inquisitor = summon_inquisitor(&mut run);
    for _ in 0..190 {
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(owner)
            .unwrap()
            .haste_pct = 150.0;
        run.app.update();
    }
    assert!(
        run.app.world().get_entity(inquisitor).is_ok(),
        "native ten-second summon remains alive"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(inquisitor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1280457)
                && event.kind == CombatLogKind::AuraRefreshed
        }),
        "real Inquisitor must refresh its channel during the second execute"
    );
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(inquisitor)
            .is_some_and(|state| state.spell_id == 1280457
                && state.cast_type == shared::casting::CastType::Channel),
        "the original channel must still be active after the repeat"
    );
}

#[test]
#[ignore = "requires canonical 266 inputs; admitted Inquisitor channel after a target leaves combat"]
fn demonology_inquisitor_channel_keeps_ticking_after_a_target_leaves_combat() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 3).unwrap();
    let owner = run.actor;
    engage(&mut run);
    run.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    let inquisitor = summon_inquisitor(&mut run);
    for _ in 0..400 {
        run.app.update();
        if run
            .targets
            .iter()
            .all(|&target| ticked_since(&run, inquisitor, target, 0))
        {
            break;
        }
    }
    let dropped = run.targets[2];
    assert!(
        ticked_since(&run, inquisitor, dropped, 0),
        "the admitted channel first ticks against every engaged target"
    );
    crate::combat_refs::end_all_combat(run.app.world_mut(), dropped);
    assert!(
        crate::combat_refs::active_opponents(run.app.world(), dropped)
            .next()
            .is_none()
    );
    assert!(channeling_mind_sear(&run, inquisitor));
    let start = observed_len(&run);
    for _ in 0..30 {
        run.app.update();
    }
    for &target in &run.targets {
        assert!(
            ticked_since(&run, inquisitor, target, start),
            "a running channel keeps ticking every target in its area"
        );
    }
}

#[test]
#[ignore = "requires canonical 266 inputs; Inquisitor channel area over an idle pack"]
fn demonology_inquisitor_refuses_channel_over_idle_pack() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 3).unwrap();
    let owner = run.actor;
    engage(&mut run);
    run.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    let idle = [run.targets[1], run.targets[2]];
    for unit in idle {
        crate::combat_refs::end_all_combat(run.app.world_mut(), unit);
        run.app
            .world_mut()
            .entity_mut(unit)
            .insert(crate::creature_ai::CreatureState::Idle);
    }
    let inquisitor = summon_inquisitor(&mut run);
    for _ in 0..240 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        !events
            .iter()
            .any(|event| event.source == Some(inquisitor.to_bits())
                && event.spell_id == Some(MIND_SEAR)
                && event.kind == CombatLogKind::CastStart),
        "the summon must not start a channel whose area includes an idle pack"
    );
    for unit in idle {
        assert!(!ticked_since(&run, inquisitor, unit, 0));
        assert!(
            crate::combat_refs::active_opponents(run.app.world(), unit)
                .next()
                .is_none(),
            "the idle pack must not acquire a summon combat reference"
        );
    }
}
