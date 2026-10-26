//! Actual paid Wither ticks earn Devil Fruit's native-duration Malevolence.
use super::*;
use crate::auras::{ActiveAuras, AuraInstance};
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const WITHER_ACTION: u32 = 445468;
const WITHER: u32 = 445474;
const DEVIL_FRUIT: u32 = 1266805;
const MALEVOLENCE: u32 = 442726;

fn owned<'a>(run: &'a Run, target: Entity, spell: u32) -> Option<&'a AuraInstance> {
    run.app
        .world()
        .get::<ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == spell)
}

fn cast_actual_wither(run: &mut Run) {
    let target = run.targets[0];
    let actual =
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, WITHER_ACTION)
            .unwrap()
            .expect("selected native Wither action");
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(actual, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
    for _ in 0..100 {
        if owned(run, target, WITHER).is_some() {
            return;
        }
        run.app.update();
    }
    panic!("ordinary paid Wither must reach its actual target");
}

fn assert_ticks_earn_native_malevolence(spec: u32) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(spec, 1).unwrap();
    let traits = run.app.world().get::<TraitLoadout>(run.actor).unwrap();
    assert!(
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == DEVIL_FRUIT),
        "frozen native profile must select actual Devil Fruit"
    );
    let duration = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(DEVIL_FRUIT)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points as f64;
    assert!(duration > 0.0, "native Devil Fruit owns its proc duration");
    assert!(owned(&run, run.actor, MALEVOLENCE).is_none());
    engage(&mut run);
    let started = crate::spell_cast::now(run.app.world());
    // SimC76f8118 actions1154–1159/init1475: actual Wither ticks attempt the
    // selected native RPPM. No Malevolence button, forced aura/child or timer/RNG
    // override; refresh only through ordinary paid Wither casts for300 seconds.
    for _ in 0..7000 {
        let now = crate::spell_cast::now(run.app.world());
        if now - started >= 300.0 {
            break;
        }
        let remaining = owned(&run, run.targets[0], WITHER)
            .and_then(|aura| aura.expires_at)
            .map_or(0.0, |expiry| expiry - now);
        if remaining <= 3.0 {
            cast_actual_wither(&mut run);
        }
        run.app.update();
        if let Some(buff) = owned(&run, run.actor, MALEVOLENCE) {
            let remaining = buff
                .expires_at
                .expect("native earned Malevolence is finite")
                - crate::spell_cast::now(run.app.world());
            assert!(
                remaining > duration - 0.1 && remaining <= duration + 0.001,
                "Devil Fruit earns its native{duration}s, not the ordinary button duration: {remaining}"
            );
            let ticks = run
                .app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.spell_id == Some(WITHER)
                        && event.kind == CombatLogKind::Damage
                        && event.periodic
                        && event.amount > 0
                })
                .count();
            assert!(
                ticks > 0,
                "actual owned Wither ticks must precede earned Malevolence"
            );
            return;
        }
    }
    let ticks = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(WITHER)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0
        })
        .count();
    assert!(
        ticks >= 50,
        "reach the actual repeated Wither-tick boundary, observed{ticks}"
    );
    panic!(
        "selected Devil Fruit must earn native Malevolence from{ticks} actual Wither ticks on spec{spec}"
    );
}

#[test]
#[ignore = "requires private265 native player inputs"]
fn hellcaller_devil_fruit_actual_affliction_wither_ticks_earn_native_malevolence() {
    assert_ticks_earn_native_malevolence(265);
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn hellcaller_devil_fruit_actual_destruction_wither_ticks_earn_native_malevolence() {
    assert_ticks_earn_native_malevolence(267);
}
