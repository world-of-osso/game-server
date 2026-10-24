//! Pinned player.cpp7395–7400/action.cpp75–79,3920–3935: GCD-tagged ordinary history.
use super::*;

fn previous(
    h: &Harness,
    actor: Entity,
    bindings: &Bindings,
    clock: Encounter,
    operand: &str,
) -> f64 {
    NativeFacts::new(h.app.world(), actor, bindings, clock, 1)
        .value(operand)
        .unwrap()
}

#[test]
#[ignore = "shared prev_gcd native RED: actual learned actions, no history/stat injection"]
fn native_apl_prev_gcd_tracks_ordinary_gcd_casts_not_utility_triggered_or_failed_requests() {
    let (mut h, actor) = monk();
    let mut b = bindings();
    b.actions.insert("blackout_kick".into(), BLACKOUT_KICK);
    let clock = fixture(h.app.world());
    let target = h.npc;
    assert_eq!(previous(&h, actor, &b, clock, "prev_gcd.1.tiger_palm"), 0.0);
    cast(&mut h, actor, TIGER_PALM, target);
    assert_eq!(previous(&h, actor, &b, clock, "prev_gcd.1.tiger_palm"), 1.0);
    cast(&mut h, actor, PROVOKE, target);
    assert_eq!(previous(&h, actor, &b, clock, "prev_gcd.1.tiger_palm"), 1.0);
    assert_eq!(previous(&h, actor, &b, clock, "prev_gcd.1.provoke"), 0.0);
    crate::spell_cast::creature_cast(h.app.world_mut(), actor, TIGER_PALM, Some(target), true)
        .unwrap();
    assert_eq!(previous(&h, actor, &b, clock, "prev_gcd.2.tiger_palm"), 0.0);
    assert_eq!(
        crate::spell_cast::creature_cast(h.app.world_mut(), actor, TIGER_PALM, Some(target), false),
        Err(crate::spell_cast::CastFailure::OnGlobalCooldown),
        "real failed admission must not become successful history"
    );
    assert_eq!(previous(&h, actor, &b, clock, "prev_gcd.2.tiger_palm"), 0.0);
    cast(&mut h, actor, BLACKOUT_KICK, target);
    assert_eq!(
        previous(&h, actor, &b, clock, "prev_gcd.1.blackout_kick"),
        1.0
    );
    assert_eq!(previous(&h, actor, &b, clock, "prev_gcd.2.tiger_palm"), 1.0);
    assert_eq!(previous(&h, actor, &b, clock, "prev_gcd.3.tiger_palm"), 0.0);
    let facts = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
    assert!(facts.value("prev_gcd.0.tiger_palm").is_err());
    assert!(facts.value("prev_gcd.1.unknown_action").is_err());
}
