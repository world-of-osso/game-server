//! TC a352b1fa boss_leymor.cpp:374567 effect0 dummy aura expires into374570.
use super::*;
use crate::auras::{ActiveAuras, RemoveReason, remove_auras};

const BRAND: u32 = 374567;
const EXPLOSION: u32 = 374570;

fn apply_brand() -> (Harness, Entity) {
    let (mut h, leymor, _) = bound_boss_at_difficulty(186644, 2515, 0, 205, None);
    crate::spell_cast::creature_cast(h.app.world_mut(), leymor, BRAND, None, true)
        .expect("native Leymor Brand cast");
    h.advance(0.1);
    assert!(h.has_aura(h.player(), BRAND));
    assert!(!h.has_aura(h.players[1].0, BRAND), "other copy excluded");
    h.players[0].2.log.clear();
    (h, leymor)
}

fn advance_to_expiry(h: &mut Harness) {
    let expiry = h
        .app
        .world()
        .get::<ActiveAuras>(h.player())
        .unwrap()
        .iter()
        .find(|a| a.spell_id == BRAND)
        .unwrap()
        .expires_at
        .unwrap();
    let now = crate::auras::aura_now(h.app.world());
    h.advance((expiry - now - 0.1).max(0.0) as f32);
    assert!(h.has_aura(h.player(), BRAND));
    assert!(
        !h.players[0]
            .2
            .log
            .iter()
            .any(|e| e.spell_id == Some(EXPLOSION))
    );
    h.advance(0.3);
}

#[test]
fn azure_explosive_brand_natural_expiry_uses_original_caster_and_damages_target() {
    let (mut h, leymor) = apply_brand();
    let player = h.player();
    let before = h.app.world().get::<Health>(player).unwrap().current;
    advance_to_expiry(&mut h);
    assert!(!h.has_aura(player, BRAND));
    assert!(
        h.players[0]
            .2
            .log
            .iter()
            .any(|e| e.spell_id == Some(EXPLOSION)
                && e.kind == CombatLogKind::CastSuccess
                && e.source == Some(leymor.to_bits())),
        "missing natural-expiry374570 from original Leymor"
    );
    assert!(
        h.app.world().get::<Health>(player).unwrap().current < before,
        "native374570 damage must reach the branded player"
    );
    assert!(
        !h.players[1]
            .2
            .log
            .iter()
            .any(|e| e.spell_id == Some(EXPLOSION)),
        "other instance must not receive explosion"
    );
}

#[test]
fn azure_explosive_brand_nonexpiry_removal_does_not_explode() {
    for reason in [
        RemoveReason::Cancelled,
        RemoveReason::Dispelled,
        RemoveReason::Death,
        RemoveReason::Evade,
    ] {
        let (mut h, _) = apply_brand();
        let player = h.player();
        let removed = remove_auras(h.app.world_mut(), player, reason, |a| a.spell_id == BRAND);
        assert_eq!(removed.len(), 1);
        h.advance(9.0);
        assert!(
            !h.players[0]
                .2
                .log
                .iter()
                .any(|e| e.spell_id == Some(EXPLOSION)),
            "unexpected explosion on {reason:?}"
        );
    }
}

#[test]
fn azure_explosive_brand_missing_caster_cleanup_does_not_explode() {
    let (mut h, leymor) = apply_brand();
    h.app.world_mut().despawn(leymor);
    h.advance(9.0);
    assert!(!h.has_aura(h.player(), BRAND));
    assert!(
        !h.players[0]
            .2
            .log
            .iter()
            .any(|e| e.spell_id == Some(EXPLOSION))
    );
}
