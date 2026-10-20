//! Native ownership qualification; SimC76f8118 target data is actor-owned.
use super::kit_tests::level32;
use super::{Follower, spawning};
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use crate::spell_cast::{SpellHistory, creature_cast, history_now};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use shared::protocol::CombatLogKind;

const MOONFIRE: u32 = 8921;
const MOONFIRE_DOT: u32 = 164812;
const CRENNA: u32 = 209072;
const MARK_OF_THE_WILD: u32 = 1126;

fn ready_crenna() -> (Harness, Entity) {
    let (mut h, caster) = level32(CRENNA);
    creature_cast(
        h.app.world_mut(),
        caster,
        MARK_OF_THE_WILD,
        Some(caster),
        false,
    )
    .unwrap();
    for _ in 0..200 {
        h.advance(1);
        let world = h.app.world();
        let spell = world.resource::<SpellStore>().get(MOONFIRE).unwrap();
        let locked = world
            .get::<SpellHistory>(caster)
            .is_some_and(|history| history.on_gcd(spell, history_now(world)));
        if !locked {
            return (h, caster);
        }
    }
    panic!("ordinary preparation GCD did not finish");
}

fn add_native_druid(h: &mut Harness, caster: Entity) -> Entity {
    let profile = h
        .app
        .world()
        .get::<Follower>(caster)
        .unwrap()
        .profile
        .clone();
    let other = spawn_npc(&mut h.app, 0.0);
    let owner = h.caster;
    spawning::initialize(h.app.world_mut(), owner, other, profile).unwrap();
    other
}

fn has_owned_dot(h: &Harness, caster: Entity) -> bool {
    h.app
        .world()
        .get::<ActiveAuras>(h.npc)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == MOONFIRE_DOT && aura.caster == caster)
        })
}

fn apply_moonfire(h: &mut Harness, caster: Entity) {
    let target = h.npc;
    creature_cast(h.app.world_mut(), caster, MOONFIRE, Some(target), false).unwrap();
    h.advance(3);
    assert!(has_owned_dot(h, caster));
}

fn native_period_steps(h: &Harness) -> usize {
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(MOONFIRE_DOT)
        .unwrap();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.aura == crate::auras::PERIODIC_DAMAGE_AURA)
        .unwrap();
    assert!(effect.aura_period_ms > 0);
    let frame_ms = match h.app.world().resource::<TimeUpdateStrategy>() {
        TimeUpdateStrategy::ManualDuration(step) => step.as_millis(),
        _ => panic!("native qualification needs deterministic frame duration"),
    };
    u128::from(effect.aura_period_ms).div_ceil(frame_ms) as usize + 1
}

#[test]
#[ignore = "MOONFIRE_ACTOR_OWNED_MAINTENANCE: qualify ordinary periodic ownership first"]
fn follower_moonfire_ownership_two_native_casters_keep_distinct_periodic_damage() {
    let (mut h, caster) = ready_crenna();
    let other = add_native_druid(&mut h, caster);
    apply_moonfire(&mut h, other);
    apply_moonfire(&mut h, caster);
    assert!(has_owned_dot(&h, other));
    assert!(has_owned_dot(&h, caster));
    let steps = native_period_steps(&h);
    h.advance(steps);
    for source in [caster, other] {
        assert!(
            h.inbox
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.periodic
                    && event.spell_id == Some(MOONFIRE_DOT)
                    && event.source == Some(source.to_bits())
                    && event.target == Some(h.npc.to_bits())
                    && event.amount > 0),
            "real periodic damage missing for {source:?}"
        );
    }
}

#[test]
#[ignore = "MOONFIRE_ACTOR_OWNED_MAINTENANCE: foreign aura must not satisfy actor's dot"]
fn follower_moonfire_ownership_foreign_dot_does_not_suppress_native_follower() {
    let (mut h, caster) = ready_crenna();
    let other = add_native_druid(&mut h, caster);
    apply_moonfire(&mut h, other);
    assert!(!has_owned_dot(&h, caster));
    let steps = native_period_steps(&h);
    for _ in 0..steps {
        super::update_ai(h.app.world_mut());
        h.advance(1);
        if has_owned_dot(&h, caster) {
            break;
        }
    }
    assert!(
        has_owned_dot(&h, other),
        "foreign native DoT must not have expired during observation"
    );
    assert!(
        has_owned_dot(&h, caster),
        "foreign DoT suppressed the actor's own Moonfire"
    );
}

#[test]
fn follower_moonfire_ownership_keeps_foreign_hot_any_caster_semantics() {
    let (mut h, caster) = ready_crenna();
    let other = add_native_druid(&mut h, caster);
    const REJUVENATION: u32 = 774;
    creature_cast(h.app.world_mut(), other, REJUVENATION, Some(caster), false).unwrap();
    h.advance(3);
    let mut health = h
        .app
        .world_mut()
        .get_mut::<shared::components::Health>(caster)
        .unwrap();
    health.current = health.max * 0.7;
    super::update_ai(h.app.world_mut());
    h.advance(3);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(caster)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == REJUVENATION && aura.caster == other)
    );
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.caster == caster.to_bits() && go.spell_id == REJUVENATION),
        "foreign HoT must retain existing any-caster maintenance policy"
    );
}

#[test]
#[ignore = "MOONFIRE_ACTOR_OWNED_MAINTENANCE: retain own active aura refusal"]
fn follower_moonfire_ownership_own_active_dot_prevents_root_recast() {
    let (mut h, caster) = ready_crenna();
    apply_moonfire(&mut h, caster);
    let steps = native_period_steps(&h);
    for _ in 0..steps {
        super::update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(has_owned_dot(&h, caster));
    let casts = h
        .inbox
        .go
        .iter()
        .filter(|go| go.caster == caster.to_bits() && go.spell_id == MOONFIRE)
        .count();
    assert_eq!(
        casts, 1,
        "own active DoT must prevent immediate root recast"
    );
}
