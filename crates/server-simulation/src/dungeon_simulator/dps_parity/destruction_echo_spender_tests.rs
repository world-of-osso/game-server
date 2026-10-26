//! Qualify each actual paid spender, independently of Incinerate's Echo PRD.
use super::*;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn damage_hits(run: &Run, spell: u32, target: Entity) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

fn assert_paid_spender_earns_echo(spell: u32) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(267, 3).unwrap();
    let traits = run.app.world().get::<TraitLoadout>(run.actor).unwrap();
    assert!(
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == 1265774)
    );
    let target = run.targets[0];
    if spell == 17877 {
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current = health.max * 0.19;
    }
    engage(&mut run);
    let actual = literal_rotation::execution::resolve_action(run.app.world(), run.actor, spell)
        .unwrap()
        .expect("selected native spender");
    let actor = run.actor;
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(actual, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..160 {
        run.app.update();
    }
    assert_eq!(
        damage_hits(&run, spell, target),
        1,
        "one actual paid native spender must land"
    );
    // SimC76f8118 actions4337/4813: each landed CB/SB attempts the shared
    // native500ms III stream. There is only one spender and no Incinerate here.
    for &recipient in &run.targets {
        assert_eq!(
            damage_hits(&run, 1265884, recipient),
            1,
            "actual paid spender{spell} must produce one native Echo at recipient{recipient:?}"
        );
    }
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_echo_paid_chaos_bolt_produces_native_pack_echo() {
    assert_paid_spender_earns_echo(116858);
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_echo_paid_shadowburn_produces_native_pack_echo() {
    assert_paid_spender_earns_echo(17877);
}
