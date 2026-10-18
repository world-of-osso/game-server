//! First source Energy phase on the existing controlled encounter fixture.
//! Its pre-existing player health buffer is not ordinary-combat budget proof.
use super::*;
use crate::spell_cast::powers::power_value;
use shared::components::PowerType;

const ICE_POWER: u32 = 389453;
const ABSOLUTE_ZERO: u32 = 387928;
const ZERO_SHIELD: u32 = 388084;
const ACTIVATE_RUNE: u32 = 390209;
const PHASE_ENERGY: f32 = 100.0;
const ENERGY_PER_TICK: f32 = 2.0;
const SOURCE_POLL_SECONDS: f32 = 0.5;
const FIXTURE_STEP_SECONDS: f32 = 0.05;

fn completed_casts(h: &Harness, caster: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == caster.to_bits() && go.spell_id == spell)
        .count()
}

fn read_phase_wait_budget(h: &Harness, telash: Entity) -> f32 {
    let (energy, _) = power_value(h.app.world(), telash, PowerType::Energy).unwrap();
    let energy_wait = (PHASE_ENERGY - energy) / ENERGY_PER_TICK;
    let cast_remaining = h
        .app
        .world()
        .get::<CastState>(telash)
        .map_or(0.0, |cast| (cast.duration - cast.elapsed).max(0.0));
    // One native poll plus two 50ms fixture/transport collection frames.
    cast_remaining.max(energy_wait) + SOURCE_POLL_SECONDS + 2.0 * FIXTURE_STEP_SECONDS
}

fn advance_until_phase_or_ready_deadline(h: &mut Harness, telash: Entity) {
    let frames = (read_phase_wait_budget(h, telash) / FIXTURE_STEP_SECONDS).ceil() as usize;
    for _ in 0..frames {
        if completed_casts(h, telash, ABSOLUTE_ZERO) > 0 {
            return;
        }
        super::azure_telash::assert_living_victim(h, telash);
        h.advance(FIXTURE_STEP_SECONDS);
    }
    if completed_casts(h, telash, ABSOLUTE_ZERO) == 0 {
        // A failed native phase assertion is valid only after its actual gates.
        super::azure_telash::assert_living_victim(h, telash);
        assert_eq!(
            power_value(h.app.world(), telash, PowerType::Energy),
            Some((PHASE_ENERGY, PHASE_ENERGY)),
            "actual native energize must reach100 before phase RED"
        );
        assert!(
            h.app.world().get::<CastState>(telash).is_none(),
            "actual preceding cast/channel must finish before phase RED"
        );
    }
}

#[test]
fn azure_telash_natural_energy_phase_waits_for_casting_and_uses_native_cost_once() {
    let (mut h, telash) = super::azure_frost_bomb::fixture();
    assert_eq!(
        h.creatures_of(197204).len(),
        3,
        "actual native Rune startup prerequisite"
    );
    let costs = &h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(ABSOLUTE_ZERO)
        .unwrap()
        .power_costs;
    assert_eq!(
        costs
            .iter()
            .map(|cost| (cost.power_type, cost.mana_cost))
            .collect::<Vec<_>>(),
        vec![(3, 100)],
        "installed70448 native100Energy cost, not a manual reset"
    );
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(telash).insert(visibility);
    h.engage(telash);
    assert!(h.has_aura(telash, ICE_POWER));
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((60.0, PHASE_ENERGY))
    );
    h.advance(19.0);
    super::azure_telash::assert_living_victim(&mut h, telash);
    let (energy, _) = power_value(h.app.world(), telash, PowerType::Energy).unwrap();
    assert!(
        energy < PHASE_ENERGY,
        "ordinary ticks have not yet supplied100Energy"
    );
    assert!(h.has_aura(telash, ICE_POWER));
    assert_eq!(
        completed_casts(&h, telash, ABSOLUTE_ZERO),
        0,
        "no premature phase"
    );
    let preceding = h
        .app
        .world()
        .get::<CastState>(telash)
        .expect("actual preceding Icy channel");
    assert_eq!(preceding.spell_id, 387151);
    assert_eq!(preceding.cast_type, shared::casting::CastType::Channel);
    let channel_finishes_at = crate::auras::aura_now(h.app.world())
        + f64::from((preceding.duration - preceding.elapsed).max(0.0));
    advance_until_phase_or_ready_deadline(&mut h, telash);
    assert_eq!(
        completed_casts(&h, telash, ABSOLUTE_ZERO),
        1,
        "source500ms poll must execute after actual100Energy and the native casting gate"
    );
    assert!(
        crate::auras::aura_now(h.app.world()) + f64::from(FIXTURE_STEP_SECONDS)
            >= channel_finishes_at,
        "phase must not cancel the actual preceding channel; observations have50ms resolution"
    );
    assert!(h.app.world().get::<CastState>(telash).is_none());
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((0.0, PHASE_ENERGY))
    );
    assert!(!h.has_aura(telash, ICE_POWER));
    assert!(h.has_aura(telash, ZERO_SHIELD));
    assert_eq!(completed_casts(&h, telash, ZERO_SHIELD), 1);
    assert_eq!(completed_casts(&h, telash, ACTIVATE_RUNE), 1);
    assert_eq!(
        h.app.world().get::<ReactState>(telash),
        Some(&ReactState::Passive)
    );
    assert!(
        h.app
            .world()
            .get::<crate::scripted_ai::waypoint_path::GravityDisabled>(telash)
            .is_some()
    );
    let lines = [
        (
            "Now you will pay!",
            shared::protocol::ChatType::MonsterYell(telash.to_bits()),
        ),
        (
            "Telash Greywing begins casting |cFFFF0000|Hspell:388008|h[Absolute Zero]|h|r!",
            shared::protocol::ChatType::RaidBossEmote(telash.to_bits()),
        ),
    ];
    for (content, channel) in lines {
        assert_eq!(
            h.inbox(0)
                .chat
                .iter()
                .filter(|message| message.content == content && message.channel == channel)
                .count(),
            1,
            "actual source phase speech must reach the same-copy observer once"
        );
        assert!(
            !h.inbox(1)
                .chat
                .iter()
                .any(|message| message.content == content),
            "no other-copy speech leak"
        );
    }
    h.advance(1.1);
    assert_eq!(
        completed_casts(&h, telash, ABSOLUTE_ZERO),
        1,
        "native100Energy cost prevents duplicate phase"
    );
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((0.0, PHASE_ENERGY))
    );
    assert!(!h.has_aura(telash, ICE_POWER));
}

#[test]
#[ignore = "next natural parent-hit outbound-jump callback RED"]
fn azure_telash_natural_absolute_zero_hit_launches_source_outbound_jump() {
    let (mut h, telash) = super::azure_frost_bomb::fixture();
    assert_eq!(h.creatures_of(197204).len(), 3);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(telash).insert(visibility);
    h.engage(telash);
    assert!(h.has_aura(telash, ICE_POWER));
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((60.0, PHASE_ENERGY))
    );
    h.advance(19.0);
    super::azure_telash::assert_living_victim(&mut h, telash);
    let preceding = h
        .app
        .world()
        .get::<CastState>(telash)
        .expect("actual preceding Icy channel before the ordinary Energy phase");
    assert_eq!(preceding.spell_id, 387151);
    assert_eq!(preceding.cast_type, shared::casting::CastType::Channel);
    let at = *h.app.world().get::<Position>(telash).unwrap();
    let from = Vec3::new(at.x, at.y, at.z);
    // TC boss_telash_greywing.cpp:85,390–397: authored center and387928 HIT.
    let center = Vec3::new(-5336.8003, 344.32678, -1066.6493);
    assert!(
        from.distance(center) > 1.0,
        "actual spawn is not already the center"
    );
    advance_until_phase_or_ready_deadline(&mut h, telash);
    assert_eq!(completed_casts(&h, telash, ABSOLUTE_ZERO), 1);
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((0.0, PHASE_ENERGY)),
        "real100Energy phase cost, never a manually filled or reset pool"
    );
    assert!(h.has_aura(telash, ZERO_SHIELD));
    assert_eq!(
        h.app.world().get::<ReactState>(telash),
        Some(&ReactState::Passive)
    );
    assert!(
        h.app
            .world()
            .get::<crate::scripted_ai::waypoint_path::GravityDisabled>(telash)
            .is_some()
    );
    h.advance(0.25);
    let at = *h.app.world().get::<Position>(telash).unwrap();
    let mid = Vec3::new(at.x, at.y, at.z);
    let toward_center = Vec2::new(center.x - from.x, center.z - from.z);
    let moved = Vec2::new(mid.x - from.x, mid.z - from.z);
    let progress = moved.dot(toward_center) / toward_center.length_squared();
    assert!(
        progress > 0.0 && progress < 1.0,
        "natural387928 effect0 must launch387981 mid-flight, not remain idle or teleport: from={from:?}, at={mid:?}, progress={progress}"
    );
    let linear_height = from.y + (center.y - from.y) * progress;
    assert!(
        mid.y > linear_height,
        "actual outbound source parabola above its linear segment"
    );
    assert_eq!(
        completed_casts(&h, telash, 387981),
        0,
        "TC uses a full-mask triggered child, not an ordinary child SpellGo"
    );
}
