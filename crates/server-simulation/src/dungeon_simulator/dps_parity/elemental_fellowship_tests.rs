//! SimC76f8118 sc_shaman.cpp4437–4439/13016–13023: Ancestor expiry draws the native Fellowship deck.
use super::elemental_mechanics_tests::{fixture, own_aura};
use super::*;
use shared::components::{Npc, UnitSummonedBy};
use shared::protocol::CombatLogKind;

fn ancestors(run: &mut Run) -> Vec<Entity> {
    run.app
        .world_mut()
        .query::<(Entity, &Npc, &UnitSummonedBy)>()
        .iter(run.app.world())
        .filter(|(_, npc, owner)| npc.template_id == 221177 && owner.0 == run.actor.to_bits())
        .map(|(entity, _, _)| entity)
        .collect()
}

fn summons(run: &Run) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(445624)
                && event.kind == CombatLogKind::Summon
        })
        .count()
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_fellowship_natural_expiries_draw_native_deck() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    let aura = own_aura(&run, 443423).expect("selected native Ancient Fellowship");
    let amount = |index| {
        aura.effects
            .iter()
            .find(|effect| effect.index == index)
            .unwrap()
            .amount as usize
    };
    let (total, successes) = (amount(0), amount(1));
    assert_eq!((total, successes), (30, 6), "native Fellowship deck");
    for _ in 0..total {
        crate::spell_cast::scripts::summon_ancestor(run.app.world_mut(), run.actor, 0);
    }
    let initial = ancestors(&mut run);
    assert_eq!(
        initial.len(),
        total,
        "each native summon produced an Ancestor"
    );
    let before = summons(&run);
    for _ in 0..1000 {
        run.app.update();
        if initial
            .iter()
            .all(|actor| run.app.world().get_entity(*actor).is_err())
        {
            break;
        }
    }
    assert!(
        initial
            .iter()
            .all(|actor| run.app.world().get_entity(*actor).is_err()),
        "original native Ancestors expired"
    );
    assert_eq!(
        summons(&run) - before,
        successes,
        "exactly one native deck drawn on natural expiries"
    );
}
