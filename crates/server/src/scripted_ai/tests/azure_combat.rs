//! TC a352b1fa boss_leymor.cpp:142-193, ordinary combat after sourced intro.
use super::*;

const LEYMOR: u32 = 186644;
const TENDER: u32 = 191164;
const INFUSED_STRIKE: u32 = 374789;

fn wake_leymor() -> (Harness, Entity) {
    let (mut h, _, map) =
        encounter_fixture(LEYMOR, 2515, 0, 205, None, EncounterState::NotStarted, true);
    h.advance(0.1);
    let leymor = h
        .creatures_of(LEYMOR)
        .into_iter()
        .find(|&actor| h.app.world().get::<InMap>(actor) == Some(&map))
        .unwrap();
    let tenders: Vec<_> = h
        .creatures_of(TENDER)
        .into_iter()
        .filter(|&actor| h.app.world().get::<InMap>(actor) == Some(&map))
        .collect();
    assert_eq!(tenders.len(), 3);
    let player = h.player();
    for tender in tenders {
        crate::combat::kill_unit(h.app.world_mut(), player, tender);
    }
    h.advance(1.2);
    assert!(
        !h.has_aura(leymor, 375729),
        "source intro completed before combat"
    );
    (h, leymor)
}

#[test]
#[ignore = "RED: source Leymor combat schedule not implemented"]
fn azure_leymor_infused_strike_starts_at_ten_seconds_and_lands_native_effects() {
    let (mut h, leymor) = wake_leymor();
    let player = h.player();
    h.engage(leymor);
    h.advance(9.8);
    assert!(!h.has_aura(player, INFUSED_STRIKE));
    assert_eq!(h.casts(0, leymor, INFUSED_STRIKE), 0);
    h.advance(0.3);
    let cast = h
        .app
        .world()
        .get::<CastState>(leymor)
        .expect("source10s Infused Strike starts a real native cast");
    assert_eq!(cast.spell_id, INFUSED_STRIKE);
    let remaining = cast.duration - cast.elapsed;
    let before = h.app.world().get::<Health>(player).unwrap().current;
    h.advance(remaining + 0.2);
    assert_eq!(h.casts(0, leymor, INFUSED_STRIKE), 1);
    assert!(h.has_aura(player, INFUSED_STRIKE), "landed native DOT");
    assert!(
        h.app.world().get::<Health>(player).unwrap().current < before,
        "native Infused Strike damage reaches the victim"
    );
}
