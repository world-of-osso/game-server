//! Native critical and non-Destruction controls for periodic shard funding.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::PowerType;
use shared::protocol::{CombatLogEvent, CombatLogKind};

fn apply_native_aura(run: &mut Run, target: Entity, spell_id: u32) {
    let effect_mask = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap()
        .effects
        .iter()
        .filter(|effect| effect.effect == 6)
        .fold(0, |mask, effect| mask | (1 << effect.index));
    assert_ne!(effect_mask, 0);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.actor,
            target,
            spell_id,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn next_tick(run: &mut Run, spell: u32) -> CombatLogEvent {
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    for _ in 0..100 {
        run.app.update();
        let event = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(spell)
                    && event.periodic
                    && event.kind == CombatLogKind::Damage
            });
        if let Some(event) = event {
            assert!(
                event.amount > 0,
                "actual native periodic damage is required"
            );
            return event.clone();
        }
    }
    panic!("native{spell} must deliver its next actual tick within five seconds");
}

fn shards(run: &Run) -> f32 {
    crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::SoulShards)
        .unwrap()
        .0
}

#[test]
#[ignore = "requires private267 native profile and world DB"]
fn destruction_periodic_critical_ticks_use_native_extra_fragment_chance() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    apply_native_aura(&mut run, actor, 75060); // Native Critical Focus, not edited crit stats.
    run.app.update();
    assert!(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .crit_pct
            >= 100.0
    );
    let before = shards(&run);
    for _ in 0..12 {
        let target = run.targets[0];
        apply_native_aura(&mut run, target, 445474);
        assert!(
            next_tick(&mut run, 445474).crit,
            "the native critical result is the prerequisite"
        );
    }
    // sc_warlock_actions.cpp1113–1117 and init.cpp1443: every tick funds
    // one fragment, while only successful native50% critical rolls add another.
    let gained = shards(&run) - before;
    assert!(
        gained > 12.0 && gained < 24.0,
        "twelve native critical ticks must fund base fragments and non-guaranteed extra fragments, got{gained}"
    );
}

#[test]
#[ignore = "requires private265 native profile and world DB"]
fn destruction_periodic_shard_funding_never_fires_for_affliction_wither() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let before = shards(&run);
    let target = run.targets[0];
    apply_native_aura(&mut run, target, 445474);
    next_tick(&mut run, 445474);
    assert_eq!(
        shards(&run),
        before,
        "sc_warlock_actions.cpp1108–1117 qualifies funding on Destruction, not shared Wither identity"
    );
}
