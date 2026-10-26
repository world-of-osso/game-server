//! Native Tempest owns Storm Swell and Arc Discharge's delayed chain, not ordinary casts.
use super::*;
use crate::auras::{ActiveAuras, AuraInstance};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const TEMPEST: u32 = 452201;
const CHAIN: u32 = 188443;
const ARC_DISCHARGE: u32 = 470532;
const STORM_SWELL: u32 = 455089;

fn assert_selected(run: &Run, talent: u32) {
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(talent),
        "full frozen actor must select native talent{talent}"
    );
}

fn owned_aura(run: &Run, spell: u32) -> Option<AuraInstance> {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
        .cloned()
}

fn damage_targets(run: &Run, spell: u32) -> Vec<Option<u64>> {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .map(|event| event.target)
        .collect()
}

fn launch(run: &mut Run, spell: u32) {
    for _ in 0..1200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.targets[0]),
            false,
        ) {
            Ok(()) => return,
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("native Tempest-buff cast{spell}: {reason:?}"),
        }
    }
    panic!("native Tempest-buff cast{spell} never became ready");
}

fn wait_for_hits(run: &mut Run, spell: u32, count: usize) {
    for _ in 0..180 {
        if damage_targets(run, spell).len() >= count {
            return;
        }
        run.app.update();
    }
    panic!(
        "native spell{spell} needs{count} actual hits; observed{}",
        damage_targets(run, spell).len()
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_tempest_buffs_real_tempest_earns_arc_then_chain_recasts_from_last_recipient() {
    // SimC76f8118 sc_shaman.cpp9994–10007 grants both buffs on actual Tempest.
    // 6709–6714/12437–12484 consumes Arc at the last chain hit and schedules325–425ms later.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 3).unwrap();
    assert_selected(&run, 455096);
    assert_selected(&run, 455088);
    enhancement_generation_tests::earn_tempest_by_spending(&mut run);
    let target = run.targets[0];
    enhancement_generation_tests::cast(&mut run, TEMPEST, target);
    wait_for_hits(&mut run, TEMPEST, 1);
    let arc = owned_aura(&run, ARC_DISCHARGE)
        .expect("an actually earned and cast Tempest must grant native Arc Discharge");
    assert_eq!(arc.stacks, 1, "one Tempest grants one Arc stack");
    assert!(
        owned_aura(&run, STORM_SWELL).is_some(),
        "actual Tempest grants Storm Swell"
    );
    assert!(
        damage_targets(&run, CHAIN).is_empty(),
        "the earning loop uses actual Bolt, not Chain"
    );
    launch(&mut run, CHAIN);
    let recipients = run.targets.len();
    wait_for_hits(&mut run, CHAIN, recipients);
    let root = damage_targets(&run, CHAIN);
    assert_eq!(
        root.len(),
        recipients,
        "only the native root chain has landed initially"
    );
    let root_finished = crate::spell_cast::now(run.app.world());
    wait_for_hits(&mut run, CHAIN, recipients * 2);
    let elapsed = crate::spell_cast::now(run.app.world()) - root_finished;
    let all = damage_targets(&run, CHAIN);
    assert!(
        elapsed >= 0.30,
        "Arc must not be a synchronous duplicate: elapsed={elapsed}"
    );
    assert_eq!(
        all[recipients],
        root[recipients - 1],
        "the actual last root recipient becomes Arc's first recipient"
    );
    assert!(
        owned_aura(&run, ARC_DISCHARGE).is_none(),
        "one chain impact consumes the one earned Arc stack"
    );
    println!("TEMPEST_ARC root_recipients={root:?} all={all:?} elapsed={elapsed}");
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_tempest_buffs_ordinary_bolt_does_not_grant_storm_swell() {
    // SimC76f8118 sc_shaman.cpp9994–9997: Tempest, not every spell event, owns this buff.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    assert_selected(&run, 455088);
    engage(&mut run);
    let target = run.targets[0];
    enhancement_generation_tests::cast(&mut run, 188196, target);
    wait_for_hits(&mut run, 188196, 1);
    assert!(
        owned_aura(&run, STORM_SWELL).is_none(),
        "setup and an ordinary real Bolt must not fabricate Tempest's Storm Swell"
    );
    assert!(
        owned_aura(&run, ARC_DISCHARGE).is_none(),
        "ordinary Bolt does not grant Arc either"
    );
}
