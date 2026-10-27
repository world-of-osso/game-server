//! Actual paid critical hits earn Fiendish Cruelty for above-threshold Shadowburn.
use super::*;
use crate::auras::{AuraApplication, AuraSource};
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::Health;
use shared::protocol::CombatLogKind;

const FIENDISH: u32 = 1245633;
const BUFF: u32 = 1245664;
const INCINERATE: u32 = 29722;
const SHADOWBURN: u32 = 17877;

fn real_hits(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
                && event.crit
        })
        .count()
}

fn cast_paid(run: &mut Run, spell: u32) {
    let actor = run.actor;
    let target = run.targets[0];
    let actual = literal_rotation::execution::resolve_action(run.app.world(), actor, spell)
        .unwrap()
        .expect("selected native action");
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(actual, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
    for _ in 0..20 {
        run.app.update();
    }
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_fiendish_actual_critical_incinerate_earns_above_threshold_shadowburn() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    let caster = run.actor;
    let selected = run
        .app
        .world()
        .get::<TraitLoadout>(caster)
        .unwrap()
        .active_talent_spells()
        .any(|talent| talent.spell_id == FIENDISH);
    assert!(selected, "frozen native profile selects Fiendish Cruelty");
    let hp = run.app.world().get::<Health>(run.targets[0]).unwrap();
    assert!(
        hp.current > hp.max * 0.20,
        "target begins above native Shadowburn threshold"
    );
    engage(&mut run);
    // Actual native critical buff controls only critical outcomes; no forced
    // Fiendish proc, buff grant, RNG override or shard refill is used.
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster,
            target: caster,
            spell_id: 75060,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    run.app.update();
    for _ in 0..60 {
        if crate::auras::has_aura(run.app.world(), caster, BUFF) {
            break;
        }
        cast_paid(&mut run, INCINERATE);
    }
    let critical = real_hits(&run, INCINERATE);
    assert!(
        critical > 0,
        "real paid critical Incinerate events required: {critical}"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), caster, BUFF),
        "selected Fiendish must earn its native buff from {critical} actual critical Incinerate hits"
    );
    // SimC76f8118 actions433–439,4795–4800,4832–4834: earned proc makes
    // actual above20% Shadowburn usable and is consumed by its execution.
    cast_paid(&mut run, SHADOWBURN);
    assert_eq!(
        real_hits(&run, SHADOWBURN),
        1,
        "actual above-threshold Shadowburn must land"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), caster, BUFF),
        "real Shadowburn consumes earned buff"
    );
}
