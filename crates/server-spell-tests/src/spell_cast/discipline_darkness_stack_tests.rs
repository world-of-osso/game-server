//! Master the Darkness stores and spends retail Void Shield stacks.
use super::test_harness::*;
use crate::auras::ActiveAuras;
use shared::components::{ActiveSpec, Health};
use shared::protocol::CombatLogKind;

fn darkness_stacks(h: &Harness) -> u8 {
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 1253591 && aura.caster == h.caster)
        })
        .map_or(0, |aura| aura.stacks)
}

#[test]
fn two_mind_blast_procs_allow_two_void_shields() {
    let mut h = harness(
        Vec::new(),
        &[17, 8092, 194384, 81749, 94472, 1253591, 1253593, 1253827],
        5,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 1_000_000.0,
        max: 1_000_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1253827,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.cast_and_deliver(8092, Some(h.npc));
    h.advance(200);
    assert_eq!(
        darkness_stacks(&h),
        1,
        "first real Mind Blast grants one stack"
    );
    h.cast_and_deliver(8092, Some(h.npc));
    h.advance(60);
    assert_eq!(
        darkness_stacks(&h),
        2,
        "second Mind Blast stores a second stack"
    );
    h.cast_and_deliver(17, Some(h.caster));
    h.advance(10);
    assert_eq!(
        darkness_stacks(&h),
        1,
        "first Void Shield spends only one stack"
    );
    h.advance(170);
    h.cast_and_deliver(17, Some(h.caster));
    h.advance(10);
    assert_eq!(
        darkness_stacks(&h),
        0,
        "second Void Shield spends the last stack"
    );
    let casts = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::CastSuccess && event.spell_id == Some(1253593))
        .count();
    assert_eq!(casts, 2, "both actual Shield casts resolve to Void Shield");
    assert!(
        h.inbox.failed.is_empty(),
        "casts must succeed: {:?}",
        h.inbox.failed
    );
}
