//! SimC76f8118 sc_shaman.cpp11579–11596: each actual Ancestor advances the running Stormkeeper cooldown.
use super::elemental_mechanics_tests::{fixture, own_aura, settle};
use super::*;

const STORMKEEPER: u32 = 191634;
const OFFERING: u32 = 443451;

fn remaining(run: &Run) -> u32 {
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(STORMKEEPER)
        .unwrap();
    world
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap()
        .remaining_cooldown_ms(spell, crate::spell_cast::now(world))
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_offering_each_native_ancestor_advances_running_stormkeeper() {
    let mut run = fixture();
    let offering = own_aura(&run, OFFERING).expect("selected native Offering from Beyond");
    let reduction_ms = (-offering
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount) as u32;
    assert!(reduction_ms > 0);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        STORMKEEPER,
        Some(run.actor),
        false,
    )
    .unwrap();
    settle(&mut run);
    let before = remaining(&run);
    assert!(
        before > reduction_ms,
        "native Stormkeeper recharge is running"
    );
    let before_summons = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.spell_id == Some(445624) && event.kind == shared::protocol::CombatLogKind::Summon
        })
        .count();
    crate::spell_cast::scripts::summon_ancestor(run.app.world_mut(), run.actor, 0);
    let after_summons = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.spell_id == Some(445624) && event.kind == shared::protocol::CombatLogKind::Summon
        })
        .count();
    assert_eq!(
        after_summons,
        before_summons + 1,
        "actual native Ancestor spawned"
    );
    let after = remaining(&run);
    assert_eq!(
        before - after,
        reduction_ms,
        "native Offering shortens the running timer once per actual summon"
    );
}
