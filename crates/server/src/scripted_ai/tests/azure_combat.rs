//! TC a352b1fa boss_leymor.cpp:142-193, ordinary combat after sourced intro.
use super::*;

const LEYMOR: u32 = 186644;
const TENDER: u32 = 191164;
const INFUSED_STRIKE: u32 = 374789;

pub(super) fn wake_leymor() -> (Harness, Entity) {
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
fn azure_leymor_erupting_fissure_starts_at_twenty_seconds_with_native_cast_time() {
    const FISSURE: u32 = 386660;
    let (mut h, leymor) = wake_leymor();
    let player = h.player();
    let map = *h.app.world().get::<InMap>(leymor).unwrap();
    super::azure_sprouts::prepare_sprout_templates(&mut h);
    super::azure_sprouts::install_azure_ground(&mut h, map);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(leymor).insert(visibility);
    h.engage(leymor);
    h.advance(19.8);
    assert_eq!(
        h.creatures_of(190509).len(),
        14,
        "ordinary native Sprouts prerequisite"
    );
    assert!(h.app.world().get::<Health>(player).unwrap().current > 0.0);
    assert_eq!(
        (Ctx {
            world: h.app.world_mut(),
            me: leymor,
        })
        .victim(),
        Some(player),
        "ordinary source clock retains its actual living victim"
    );
    assert_eq!(h.casts(0, leymor, FISSURE), 0);
    h.advance(0.3);
    let cast = h
        .app
        .world()
        .get::<CastState>(leymor)
        .expect("source20s Erupting Fissure starts its real native cast");
    assert_eq!(cast.spell_id, FISSURE);
    assert_eq!(cast.target, player.to_bits());
    let remaining = cast.duration - cast.elapsed;
    h.advance(remaining + 0.2);
    assert_eq!(h.casts(0, leymor, FISSURE), 1);
    assert!(h.inbox(0).spell_gos.iter().any(|go| {
        go.caster == leymor.to_bits()
            && go.spell_id == FISSURE
            && go.target == Some(player.to_bits())
    }));
}

fn prepare_leymor_for_later_clock() -> (Harness, Entity) {
    let (mut h, leymor) = wake_leymor();
    let map = *h.app.world().get::<InMap>(leymor).unwrap();
    super::azure_sprouts::prepare_sprout_templates(&mut h);
    super::azure_sprouts::install_azure_ground(&mut h, map);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(leymor).insert(visibility);
    h.engage(leymor);
    h.advance(6.0);
    assert_eq!(
        h.creatures_of(190509).len(),
        14,
        "ordinary native Sprouts prerequisite"
    );
    (h, leymor)
}

fn assert_first_fissure_and_living_victim(h: &mut Harness, leymor: Entity) {
    assert_eq!(
        h.casts(0, leymor, 386660),
        1,
        "earlier ordinary Fissure completed"
    );
    let player = h.player();
    assert!(h.app.world().get::<Health>(player).unwrap().current > 0.0);
    assert_eq!(
        (Ctx {
            world: h.app.world_mut(),
            me: leymor,
        })
        .victim(),
        Some(player),
        "ordinary later clock retains its actual living victim"
    );
}

#[test]
#[ignore = "RED: native Leymor first31s Explosive Brand clock unported"]
fn azure_leymor_later_clock_brand_starts_at_thirty_one_and_applies_native_aura() {
    const BRAND: u32 = 374567;
    let (mut h, leymor) = prepare_leymor_for_later_clock();
    h.advance(24.8);
    assert_first_fissure_and_living_victim(&mut h, leymor);
    assert_eq!(h.casts(0, leymor, BRAND), 0);
    assert!(!h.has_aura(h.player(), BRAND));
    h.advance(0.3);
    let cast = h
        .app
        .world()
        .get::<CastState>(leymor)
        .expect("source31s Explosive Brand starts its native cast");
    assert_eq!(cast.spell_id, BRAND);
    assert_eq!(cast.target, leymor.to_bits());
    let remaining = cast.duration - cast.elapsed;
    h.advance(remaining + 0.2);
    assert_eq!(h.casts(0, leymor, BRAND), 1);
    assert!(
        h.has_aura(h.player(), BRAND),
        "ordinary parent applies native Brand"
    );
    assert!(h.inbox(0).spell_gos.iter().any(|go| {
        go.caster == leymor.to_bits() && go.spell_id == BRAND && go.target == Some(leymor.to_bits())
    }));
}

#[test]
#[ignore = "RED: native Leymor first45s Consuming Stomp channel unported"]
fn azure_leymor_later_clock_stomp_channels_at_forty_five_with_native_duration() {
    const STOMP: u32 = 374720;
    let (mut h, leymor) = prepare_leymor_for_later_clock();
    h.advance(38.8);
    assert_first_fissure_and_living_victim(&mut h, leymor);
    assert_eq!(h.casts(0, leymor, STOMP), 0);
    h.advance(0.3);
    let cast = h
        .app
        .world()
        .get::<CastState>(leymor)
        .expect("source45s Consuming Stomp starts its native channel");
    assert_eq!(cast.spell_id, STOMP);
    assert_eq!(cast.cast_type, shared::casting::CastType::Channel);
    assert_eq!(cast.target, leymor.to_bits());
    assert_eq!(h.casts(0, leymor, STOMP), 1);
    let remaining = cast.duration - cast.elapsed;
    h.advance(remaining + 0.2);
    assert!(h.app.world().get::<CastState>(leymor).is_none());
    assert_eq!(h.casts(0, leymor, STOMP), 1);
}

#[test]
fn azure_leymor_sprouts_starts_at_three_seconds_with_native_cast_time() {
    const SPROUTS: u32 = 374364;
    let (mut h, leymor) = wake_leymor();
    // Headless fixture has no interest plugin; use the existing recipient-set contract.
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(leymor).insert(visibility);
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
