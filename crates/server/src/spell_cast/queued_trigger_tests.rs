//! Native effect64 queues its child instead of resolving it inside the parent.
use super::test_harness::*;
use crate::auras::has_aura;

const STACK_PARENT: u32 = 412677;
const STACK_CHILD: u32 = 412680;
const LUNAR_PARENT: u32 = 298992;
const LUNAR_CHILD: u32 = 181448;

fn native_fixture() -> Harness {
    let data = world_db().unwrap();
    let spells = [
        STACK_PARENT,
        STACK_CHILD,
        LUNAR_PARENT,
        LUNAR_CHILD,
        298993,
        181449,
    ]
    .into_iter()
    .map(|id| data.store.get(id).expect("native trigger closure").clone())
    .collect();
    harness(spells, &[], 5).unwrap()
}

#[test]
fn native_trigger64_one_ms_child_is_not_inline_and_follows_parent_application() {
    let mut h = native_fixture();
    let caster = h.npc;
    super::creature::creature_cast(h.app.world_mut(), caster, STACK_PARENT, Some(caster), false)
        .unwrap();
    assert!(has_aura(h.app.world(), caster, STACK_PARENT));
    assert!(!has_aura(h.app.world(), caster, STACK_CHILD));
    h.advance(1);
    assert!(has_aura(h.app.world(), caster, STACK_CHILD));
}

#[test]
fn native_trigger64_zero_ms_child_is_not_reentrant_and_follows_parent_application() {
    let mut h = native_fixture();
    let caster = h.npc;
    super::creature::creature_cast(h.app.world_mut(), caster, LUNAR_PARENT, Some(caster), false)
        .unwrap();
    assert!(has_aura(h.app.world(), caster, LUNAR_PARENT));
    assert!(!has_aura(h.app.world(), caster, LUNAR_CHILD));
    h.advance(1);
    assert!(has_aura(h.app.world(), caster, LUNAR_CHILD));
}
