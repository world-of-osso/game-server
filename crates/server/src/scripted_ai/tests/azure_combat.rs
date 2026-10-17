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
    // Population fixtures otherwise park the player at the Serpent's distant spawn.
    let position = *h.app.world().get::<Position>(leymor).unwrap();
    h.app.world_mut().entity_mut(player).insert(position);
    (h, leymor)
}

#[test]
fn azure_leymor_infused_strike_starts_at_ten_seconds_and_lands_native_effects() {
    let (mut h, leymor) = wake_leymor();
    let player = h.player();
    h.engage(leymor);
    h.advance(9.8);
    assert!(h.app.world().get::<Health>(player).unwrap().current > 0.0);
    assert_eq!(
        (Ctx {
            world: h.app.world_mut(),
            me: leymor,
        })
        .victim(),
        Some(player),
        "source combat clock has an actual living victim"
    );
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

#[test]
#[ignore = "RED: source Ley-Line Sprouts schedule not implemented"]
fn azure_leymor_sprouts_starts_at_three_seconds_with_native_cast_time() {
    const SPROUTS: u32 = 374364;
    let (mut h, leymor) = wake_leymor();
    let sprouts_casts = |h: &Harness| {
        h.inbox(0)
            .spell_gos
            .iter()
            .filter(|go| go.caster == leymor.to_bits() && go.spell_id == SPROUTS)
            .count()
    };
    h.engage(leymor);
    h.advance(2.8);
    assert_eq!(sprouts_casts(&h), 0);
    h.advance(0.3);
    let cast = h
        .app
        .world()
        .get::<CastState>(leymor)
        .expect("source3s Ley-Line Sprouts starts a real native cast");
    assert_eq!(cast.spell_id, SPROUTS);
    let remaining = cast.duration - cast.elapsed;
    h.advance(remaining + 0.2);
    assert_eq!(sprouts_casts(&h), 1, "completed native self-cast receipt");
    assert!(h.inbox(0).spell_gos.iter().any(|go| {
        go.caster == leymor.to_bits()
            && go.spell_id == SPROUTS
            && go.target == Some(leymor.to_bits())
    }));
}
