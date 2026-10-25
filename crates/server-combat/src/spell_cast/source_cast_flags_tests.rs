//! TC a352b1fa Gordoth's TRIGGERED_IGNORE_SPELL_AND_CATEGORY_CD (0x2),
//! Spell::IsIgnoringCooldowns / SpellHistory::HandleCooldowns.
use super::*;
use shared::components::{Mana, Position};

#[test]
fn source_flags_ignore_cooldowns_keeps_native_cast_time_and_existing_category_deadline() {
    let mut h = test_harness::harness(Vec::new(), &[119999, 120024], 1).unwrap();
    creature_cast(h.app.world_mut(), h.npc, 119999, Some(h.caster), false).unwrap();
    h.advance(22);
    let first_deadline = h
        .app
        .world()
        .get::<SpellHistory>(h.npc)
        .unwrap()
        .category_ready_at[&1152];
    assert!(first_deadline > now(h.app.world()));
    assert!(matches!(
        creature_cast(h.app.world_mut(), h.npc, 120024, None, false),
        Err(CastFailure::OnCooldown)
    ));
    creature_cast_ignoring_cooldowns(h.app.world_mut(), h.npc, 120024, None).unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<SpellHistory>(h.npc)
            .unwrap()
            .category_ready_at[&1152],
        first_deadline,
        "source HandleCooldowns skips the flagged cast, not just its cooldown check"
    );
    let at = *h.app.world().get::<Position>(h.npc).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert(Position {
        x: at.x + 2.0,
        ..at
    });
    creature_cast_ignoring_cooldowns(h.app.world_mut(), h.npc, 119999, Some(h.caster)).unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<CastState>(h.npc)
            .map(|cast| cast.spell_id),
        Some(119999)
    );
    h.advance(10);
    assert!(
        h.app.world().get::<CastState>(h.npc).is_some(),
        "no instant-cast/FULL_MASK shortcut"
    );
    h.advance(12);
    assert!(h.app.world().get::<CastState>(h.npc).is_none());
    assert_eq!(
        h.app
            .world()
            .get::<SpellHistory>(h.npc)
            .unwrap()
            .category_ready_at[&1152],
        first_deadline
    );
}

#[test]
fn source_flags_ignore_cooldowns_retains_gcd_caster_auras_and_power_checks() {
    let mut h = test_harness::harness(Vec::new(), &[133, 853], 8).unwrap();
    h.cast_and_deliver(133, Some(h.npc));
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    cancel_cast(h.app.world_mut(), h.caster);
    assert!(matches!(
        creature_cast_ignoring_cooldowns(h.app.world_mut(), h.caster, 133, Some(h.npc)),
        Err(CastFailure::OnGlobalCooldown)
    ));
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.npc,
            target: h.caster,
            spell_id: 853,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(matches!(
        creature_cast_ignoring_cooldowns(h.app.world_mut(), h.caster, 133, Some(h.npc)),
        Err(CastFailure::CrowdControlled(
            crate::auras::CastBlock::Stunned
        ))
    ));
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 853,
    );
    h.advance(32);
    h.app.world_mut().get_mut::<Mana>(h.caster).unwrap().current = 0.0;
    assert!(matches!(
        creature_cast_ignoring_cooldowns(h.app.world_mut(), h.caster, 133, Some(h.npc)),
        Err(CastFailure::NotEnoughPower(PowerType::Mana))
    ));
}
